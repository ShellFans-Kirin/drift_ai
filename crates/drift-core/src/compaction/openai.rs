//! OpenAI Chat Completions provider — gpt-5 / gpt-4o / o1 / o3 series.
//!
//! Uses the shared `streaming::for_each_sse_data` helper. Wire format is OpenAI's
//! standard `chat.completions` SSE: each `data:` payload contains a chunk
//! object with `choices[0].delta.content` and a final chunk with `usage`.
//!
//! Reasoning tokens (o1/o3 series): folded into `output_tokens` for billing.
//! OpenAI's API returns them under `usage.completion_tokens_details.reasoning_tokens`.
//! The wallet-visible cost is unchanged whether they're labelled "reasoning"
//! or "output" — they bill at the output rate either way.

use crate::compaction::streaming::for_each_sse_data;
use crate::compaction::{
    backoff, parse_retry_after, CompactedSummary, CompactionError, CompactionProvider,
    CompactionRes, CompactionResult, CompactionUsage, LlmCompletion,
};
// `CompactedSummary` is used in `parse_summary` below.
use crate::model::NormalizedSession;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_MODEL: &str = "gpt-5";
const DEFAULT_MAX_TOKENS: u32 = 4096;

#[derive(Clone)]
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub http: reqwest::Client,
}

impl OpenAIProvider {
    /// Build from `OPENAI_API_KEY`. Returns `None` if unset / empty.
    pub fn try_new(model: Option<String>) -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(Self::with_key(api_key, model))
    }

    pub fn with_key(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com".into(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.into()),
            max_tokens: DEFAULT_MAX_TOKENS,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(180))
                .user_agent(concat!("drift_ai/", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Compact a session — the trait entry point.
    pub async fn compact_async(
        &self,
        session: &NormalizedSession,
    ) -> CompactionRes<CompactionResult> {
        let prompt = build_compaction_prompt(session);
        let system = compaction_system_prompt();
        let result = self.complete_async(system, &prompt).await?;
        let summary = parse_summary(&result.text, session, &self.model);
        Ok(CompactionResult {
            summary,
            usage: Some(CompactionUsage {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: session.session_id.clone(),
                model: self.model.clone(),
                input_tokens: result.input_tokens,
                output_tokens: result.output_tokens,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                cost_usd: result.cost_usd,
                called_at: Utc::now(),
            }),
        })
    }

    /// Generic completion — used by `drift handoff` for the second-pass LLM
    /// call with a different prompt shape.
    pub async fn complete_async(&self, system: &str, user: &str) -> CompactionRes<LlmCompletion> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "max_completion_tokens": self.max_tokens,
            "stream": true,
            "stream_options": { "include_usage": true },
            "messages": [
                { "role": "system", "content": system },
                { "role": "user",   "content": user   },
            ],
        });

        let mut attempts = 0u32;
        let max_attempts = 5;
        loop {
            attempts += 1;
            let resp = self
                .http
                .post(&url)
                .bearer_auth(&self.api_key)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await;

            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    if attempts >= max_attempts || (!e.is_timeout() && !e.is_connect()) {
                        return Err(CompactionError::TransientNetwork(e));
                    }
                    tokio::time::sleep(backoff(attempts)).await;
                    continue;
                }
            };

            let status = resp.status();
            if status.is_success() {
                return self.consume_stream(resp).await;
            }

            let retry_after = parse_retry_after(resp.headers());
            let body_text = resp.text().await.unwrap_or_default();

            if status.as_u16() == 401 {
                return Err(CompactionError::AuthInvalid);
            }
            if status.as_u16() == 404 {
                return Err(CompactionError::ModelNotFound(self.model.clone()));
            }
            if status.as_u16() == 429 {
                if attempts >= max_attempts {
                    return Err(CompactionError::RateLimited { retry_after });
                }
                let wait = retry_after.unwrap_or_else(|| backoff(attempts));
                tokio::time::sleep(wait).await;
                continue;
            }
            if status.as_u16() == 400
                && (body_text.to_lowercase().contains("context")
                    || body_text.to_lowercase().contains("maximum context"))
            {
                return Err(CompactionError::ContextTooLong {
                    tokens: 0,
                    limit: 0,
                });
            }
            if status.is_server_error() {
                if attempts >= 4 {
                    return Err(CompactionError::Other(anyhow::anyhow!(
                        "OpenAI server error {}: {}",
                        status,
                        body_text
                    )));
                }
                tokio::time::sleep(backoff(attempts)).await;
                continue;
            }

            return Err(CompactionError::Other(anyhow::anyhow!(
                "unexpected OpenAI response {}: {}",
                status,
                body_text
            )));
        }
    }

    async fn consume_stream(&self, resp: reqwest::Response) -> CompactionRes<LlmCompletion> {
        use std::cell::RefCell;
        let accumulated = RefCell::new(String::new());
        let usage_in = RefCell::new(0u32);
        let usage_out = RefCell::new(0u32);
        let usage_reasoning = RefCell::new(0u32);
        let saw_done = RefCell::new(false);

        for_each_sse_data(resp, |payload| {
            match payload {
                None => {
                    *saw_done.borrow_mut() = true;
                }
                Some(data) => {
                    let chunk: serde_json::Value = serde_json::from_str(data)
                        .map_err(|e| CompactionError::Stream(format!("bad chunk: {}", e)))?;
                    if let Some(text) = chunk
                        .pointer("/choices/0/delta/content")
                        .and_then(|v| v.as_str())
                    {
                        accumulated.borrow_mut().push_str(text);
                    }
                    if let Some(u) = chunk.get("usage") {
                        if let Some(it) = u.get("prompt_tokens").and_then(|v| v.as_u64()) {
                            *usage_in.borrow_mut() = it as u32;
                        }
                        if let Some(ot) = u.get("completion_tokens").and_then(|v| v.as_u64()) {
                            *usage_out.borrow_mut() = ot as u32;
                        }
                        if let Some(rt) = u
                            .pointer("/completion_tokens_details/reasoning_tokens")
                            .and_then(|v| v.as_u64())
                        {
                            *usage_reasoning.borrow_mut() = rt as u32;
                        }
                    }
                }
            }
            Ok(())
        })
        .await?;

        let text = accumulated.into_inner();
        if text.is_empty() && !*saw_done.borrow() {
            return Err(CompactionError::Stream(
                "OpenAI stream ended without content or [DONE]".into(),
            ));
        }

        // OpenAI's `completion_tokens` ALREADY includes reasoning tokens for
        // o1/o3 models — fold defensively only if it doesn't.
        let mut output_tokens = usage_out.into_inner();
        let reasoning = usage_reasoning.into_inner();
        if output_tokens < reasoning {
            output_tokens += reasoning;
        }

        let input_tokens = usage_in.into_inner();
        let cost_usd = pricing_for_openai(&self.model)
            .map(|p| {
                (input_tokens as f64 / 1_000_000.0) * p.input_per_mtok
                    + (output_tokens as f64 / 1_000_000.0) * p.output_per_mtok
            })
            .unwrap_or(0.0);

        Ok(LlmCompletion {
            text,
            model: self.model.clone(),
            input_tokens,
            output_tokens,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            cost_usd,
        })
    }
}

impl CompactionProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "openai"
    }
    fn compact(&self, session: &NormalizedSession) -> CompactionRes<CompactionResult> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CompactionError::Other(anyhow::anyhow!(e)))?;
        rt.block_on(self.compact_async(session))
    }
}

// ---------------------------------------------------------------------------
// Pricing — built-in, conservative. Cross-check OpenAI's pricing page.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct OpenAIPricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

pub fn pricing_for_openai(model: &str) -> Option<OpenAIPricing> {
    let m = model.to_lowercase();
    Some(match m.as_str() {
        s if s.starts_with("gpt-5") => OpenAIPricing {
            input_per_mtok: 1.25,
            output_per_mtok: 10.0,
        },
        s if s.starts_with("gpt-4o-mini") => OpenAIPricing {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
        },
        s if s.starts_with("gpt-4o") => OpenAIPricing {
            input_per_mtok: 2.50,
            output_per_mtok: 10.0,
        },
        s if s.starts_with("o3-mini") => OpenAIPricing {
            input_per_mtok: 1.10,
            output_per_mtok: 4.40,
        },
        s if s.starts_with("o3") => OpenAIPricing {
            input_per_mtok: 10.0,
            output_per_mtok: 40.0,
        },
        s if s.starts_with("o1-mini") => OpenAIPricing {
            input_per_mtok: 3.0,
            output_per_mtok: 12.0,
        },
        s if s.starts_with("o1") => OpenAIPricing {
            input_per_mtok: 15.0,
            output_per_mtok: 60.0,
        },
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Prompt construction — borrows the same template the Anthropic provider uses
// ---------------------------------------------------------------------------

const TEMPLATE: &str = include_str!("../../templates/compaction.md");

fn compaction_system_prompt() -> &'static str {
    "You compact AI-assisted coding sessions into concise markdown records. \
     Output ONLY the markdown with the sections requested; no preamble, \
     no chain-of-thought, no commentary."
}

fn build_compaction_prompt(session: &NormalizedSession) -> String {
    let transcript = render_transcript(session);
    TEMPLATE
        .replace("{{session_id}}", &session.session_id)
        .replace("{{agent_slug}}", session.agent_slug.as_str())
        .replace("{{model}}", session.model.as_deref().unwrap_or(""))
        .replace("{{transcript}}", &transcript)
}

fn render_transcript(session: &NormalizedSession) -> String {
    let mut out = String::new();
    for t in &session.turns {
        out.push_str(&format!(
            "### {:?} @ {}\n{}\n\n",
            t.role,
            t.timestamp.to_rfc3339(),
            t.content_text
        ));
    }
    out
}

fn parse_summary(text: &str, session: &NormalizedSession, model: &str) -> CompactedSummary {
    crate::compaction::parse_summary_markdown(
        text,
        &session.session_id,
        session.agent_slug,
        model,
        session.turns.len() as u32,
        0,
    )
}

// ---------------------------------------------------------------------------
// Wire types (kept private — we only round-trip via serde_json::Value)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct ChatCompletionChunk {
    id: Option<String>,
    object: Option<String>,
    choices: Option<Vec<ChatChoice>>,
    usage: Option<UsageBlock>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct ChatChoice {
    index: Option<u32>,
    delta: Option<DeltaBlock>,
    finish_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct DeltaBlock {
    role: Option<String>,
    content: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct UsageBlock {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
    completion_tokens_details: Option<CompletionDetails>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct CompletionDetails {
    reasoning_tokens: Option<u32>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentSlug, NormalizedSession, Role, Turn};
    use chrono::{TimeZone, Utc};

    fn dummy_session() -> NormalizedSession {
        NormalizedSession {
            session_id: "openai-test".into(),
            agent_slug: AgentSlug::ClaudeCode,
            model: Some("gpt-5".into()),
            working_dir: None,
            git_head_at_start: None,
            started_at: Utc.with_ymd_and_hms(2026, 4, 25, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 4, 25, 10, 5, 0).unwrap(),
            turns: vec![Turn {
                turn_id: "t1".into(),
                role: Role::User,
                content_text: "hi".into(),
                tool_calls: vec![],
                tool_results: vec![],
                timestamp: Utc.with_ymd_and_hms(2026, 4, 25, 10, 1, 0).unwrap(),
            }],
            thinking_blocks: 0,
        }
    }

    #[test]
    fn missing_key_returns_none() {
        let prior = std::env::var("OPENAI_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");
        assert!(OpenAIProvider::try_new(None).is_none());
        if let Some(v) = prior {
            std::env::set_var("OPENAI_API_KEY", v);
        }
    }

    #[test]
    fn pricing_known_models() {
        let gpt5 = pricing_for_openai("gpt-5").unwrap();
        assert!((gpt5.input_per_mtok - 1.25).abs() < 0.01);
        let mini = pricing_for_openai("gpt-4o-mini").unwrap();
        assert!((mini.input_per_mtok - 0.15).abs() < 0.001);
        assert!(pricing_for_openai("nonexistent-model-xyz").is_none());
    }

    #[tokio::test]
    async fn happy_path_streaming() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"choices\":[{\"delta\":{\"content\":\"## Summary\\n\\nReal summary.\\n\\n## Files touched\\n\\n- src/lib.rs\\n\"}}]}\n\n\
                 data: {\"choices\":[{\"finish_reason\":\"stop\",\"delta\":{}}],\"usage\":{\"prompt_tokens\":50,\"completion_tokens\":30}}\n\n\
                 data: [DONE]\n\n",
            )
            .create_async()
            .await;

        let provider = OpenAIProvider::with_key("sk-test".into(), Some("gpt-5".into()))
            .with_base_url(server.url());

        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert!(res.summary.summary.contains("Real summary"));
        assert_eq!(res.summary.files_touched, vec!["src/lib.rs"]);
        let u = res.usage.unwrap();
        assert_eq!(u.input_tokens, 50);
        assert_eq!(u.output_tokens, 30);
        assert!(u.cost_usd > 0.0);
    }

    #[tokio::test]
    async fn rate_limited_then_succeeds() {
        let mut server = mockito::Server::new_async().await;
        let _m1 = server
            .mock("POST", "/v1/chat/completions")
            .with_status(429)
            .with_header("retry-after", "1")
            .with_body("{\"error\":{\"message\":\"rate limited\"}}")
            .expect(1)
            .create_async()
            .await;
        let _m2 = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"choices\":[{\"delta\":{\"content\":\"## Summary\\n\\nOK\"}}]}\n\n\
                 data: {\"choices\":[{\"finish_reason\":\"stop\",\"delta\":{}}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n\
                 data: [DONE]\n\n",
            )
            .create_async()
            .await;

        let provider = OpenAIProvider::with_key("sk-test".into(), Some("gpt-4o".into()))
            .with_base_url(server.url());
        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert!(res.summary.summary.contains("OK"));
    }

    #[tokio::test]
    async fn auth_invalid_on_401() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/chat/completions")
            .with_status(401)
            .with_body("{\"error\":{\"message\":\"bad key\"}}")
            .create_async()
            .await;

        let provider = OpenAIProvider::with_key("sk-bogus".into(), Some("gpt-5".into()))
            .with_base_url(server.url());
        let err = provider.compact_async(&dummy_session()).await.unwrap_err();
        assert!(matches!(err, CompactionError::AuthInvalid));
    }

    #[tokio::test]
    async fn model_not_found_on_404() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/chat/completions")
            .with_status(404)
            .with_body("{\"error\":{\"message\":\"model not found\"}}")
            .create_async()
            .await;

        let provider = OpenAIProvider::with_key("sk-test".into(), Some("gpt-fake".into()))
            .with_base_url(server.url());
        let err = provider.compact_async(&dummy_session()).await.unwrap_err();
        assert!(matches!(err, CompactionError::ModelNotFound(_)));
    }

    #[tokio::test]
    async fn reasoning_tokens_folded() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"choices\":[{\"delta\":{\"content\":\"## Summary\\n\\nDone\"}}]}\n\n\
                 data: {\"choices\":[{\"finish_reason\":\"stop\",\"delta\":{}}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"completion_tokens_details\":{\"reasoning_tokens\":50}}}\n\n\
                 data: [DONE]\n\n",
            )
            .create_async()
            .await;

        let provider = OpenAIProvider::with_key("sk-test".into(), Some("o3".into()))
            .with_base_url(server.url());
        let res = provider.compact_async(&dummy_session()).await.unwrap();
        let u = res.usage.unwrap();
        // completion_tokens=5 < reasoning=50, so we fold to 55 defensively.
        assert_eq!(u.output_tokens, 55);
    }
}
