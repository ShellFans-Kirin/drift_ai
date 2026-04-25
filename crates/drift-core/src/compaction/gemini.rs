//! Google Gemini provider via AI Studio (`generativelanguage.googleapis.com`).
//!
//! Wire format quirks vs OpenAI:
//! - Auth: API key as `?key=` URL query param (no header).
//! - System prompt lives at top-level `systemInstruction.parts[0].text`,
//!   NOT as the first chat message.
//! - SSE event payloads contain a JSON object with
//!   `candidates[0].content.parts[0].text` and a final `usageMetadata` block.
//! - The endpoint is `:streamGenerateContent?alt=sse`.

use crate::compaction::streaming::for_each_sse_data;
use crate::compaction::{
    backoff, parse_retry_after, CompactionError, CompactionProvider, CompactionRes,
    CompactionResult, CompactionUsage, LlmCompletion,
};
use crate::model::NormalizedSession;
use chrono::Utc;
use std::time::Duration;

const DEFAULT_MODEL: &str = "gemini-2.5-pro";
const DEFAULT_MAX_TOKENS: u32 = 4096;

#[derive(Clone)]
pub struct GeminiProvider {
    api_key: String,
    base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub http: reqwest::Client,
}

impl GeminiProvider {
    pub fn try_new(model: Option<String>) -> Option<Self> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(Self::with_key(api_key, model))
    }

    pub fn with_key(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            base_url: "https://generativelanguage.googleapis.com".into(),
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

    pub async fn compact_async(
        &self,
        session: &NormalizedSession,
    ) -> CompactionRes<CompactionResult> {
        let prompt = build_compaction_prompt(session);
        let result = self
            .complete_async(compaction_system_prompt(), &prompt)
            .await?;
        let summary = crate::compaction::parse_summary_markdown(
            &result.text,
            &session.session_id,
            session.agent_slug,
            &self.model,
            session.turns.len() as u32,
            0,
        );
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

    pub async fn complete_async(&self, system: &str, user: &str) -> CompactionRes<LlmCompletion> {
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, self.model, self.api_key
        );
        let body = serde_json::json!({
            "systemInstruction": { "parts": [{ "text": system }] },
            "contents": [
                { "role": "user", "parts": [{ "text": user }] }
            ],
            "generationConfig": {
                "maxOutputTokens": self.max_tokens,
            }
        });

        let mut attempts = 0u32;
        let max_attempts = 5;
        loop {
            attempts += 1;
            let resp = self
                .http
                .post(&url)
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

            // Gemini returns 401 / 403 for bad/unauthorized keys; treat both as auth.
            if status.as_u16() == 401 || status.as_u16() == 403 {
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
                    || body_text.to_lowercase().contains("token limit"))
            {
                return Err(CompactionError::ContextTooLong {
                    tokens: 0,
                    limit: 0,
                });
            }
            if status.is_server_error() {
                if attempts >= 4 {
                    return Err(CompactionError::Other(anyhow::anyhow!(
                        "Gemini server error {}: {}",
                        status,
                        body_text
                    )));
                }
                tokio::time::sleep(backoff(attempts)).await;
                continue;
            }

            return Err(CompactionError::Other(anyhow::anyhow!(
                "unexpected Gemini response {}: {}",
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

        for_each_sse_data(resp, |payload| {
            match payload {
                None => {} // Gemini does not emit [DONE]
                Some(data) => {
                    let v: serde_json::Value = serde_json::from_str(data)
                        .map_err(|e| CompactionError::Stream(format!("bad chunk: {}", e)))?;
                    if let Some(text) = v
                        .pointer("/candidates/0/content/parts/0/text")
                        .and_then(|x| x.as_str())
                    {
                        accumulated.borrow_mut().push_str(text);
                    }
                    if let Some(u) = v.get("usageMetadata") {
                        if let Some(it) = u.get("promptTokenCount").and_then(|x| x.as_u64()) {
                            *usage_in.borrow_mut() = it as u32;
                        }
                        if let Some(ot) = u.get("candidatesTokenCount").and_then(|x| x.as_u64()) {
                            *usage_out.borrow_mut() = ot as u32;
                        }
                    }
                }
            }
            Ok(())
        })
        .await?;

        let text = accumulated.into_inner();
        let input_tokens = usage_in.into_inner();
        let output_tokens = usage_out.into_inner();
        let cost_usd = pricing_for_gemini(&self.model)
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

impl CompactionProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }
    fn compact(&self, session: &NormalizedSession) -> CompactionRes<CompactionResult> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CompactionError::Other(anyhow::anyhow!(e)))?;
        rt.block_on(self.compact_async(session))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GeminiPricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

pub fn pricing_for_gemini(model: &str) -> Option<GeminiPricing> {
    let m = model.to_lowercase();
    Some(match m.as_str() {
        s if s.starts_with("gemini-2.5-pro") => GeminiPricing {
            input_per_mtok: 1.25,
            output_per_mtok: 10.0,
        },
        s if s.starts_with("gemini-2.5-flash") => GeminiPricing {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
        },
        s if s.starts_with("gemini-2.0-pro") => GeminiPricing {
            input_per_mtok: 1.25,
            output_per_mtok: 5.0,
        },
        s if s.starts_with("gemini-1.5-pro") => GeminiPricing {
            input_per_mtok: 1.25,
            output_per_mtok: 5.0,
        },
        s if s.starts_with("gemini-1.5-flash") => GeminiPricing {
            input_per_mtok: 0.075,
            output_per_mtok: 0.30,
        },
        _ => return None,
    })
}

const TEMPLATE: &str = include_str!("../../templates/compaction.md");

fn compaction_system_prompt() -> &'static str {
    "You compact AI-assisted coding sessions into concise markdown records. \
     Output ONLY the markdown with the sections requested; no preamble, \
     no chain-of-thought, no commentary."
}

fn build_compaction_prompt(session: &NormalizedSession) -> String {
    let mut transcript = String::new();
    for t in &session.turns {
        transcript.push_str(&format!(
            "### {:?} @ {}\n{}\n\n",
            t.role,
            t.timestamp.to_rfc3339(),
            t.content_text
        ));
    }
    TEMPLATE
        .replace("{{session_id}}", &session.session_id)
        .replace("{{agent_slug}}", session.agent_slug.as_str())
        .replace("{{model}}", session.model.as_deref().unwrap_or(""))
        .replace("{{transcript}}", &transcript)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentSlug, NormalizedSession, Role, Turn};
    use chrono::{TimeZone, Utc};

    fn dummy_session() -> NormalizedSession {
        NormalizedSession {
            session_id: "gemini-test".into(),
            agent_slug: AgentSlug::ClaudeCode,
            model: Some("gemini-2.5-pro".into()),
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
        let prior = std::env::var("GEMINI_API_KEY").ok();
        std::env::remove_var("GEMINI_API_KEY");
        assert!(GeminiProvider::try_new(None).is_none());
        if let Some(v) = prior {
            std::env::set_var("GEMINI_API_KEY", v);
        }
    }

    #[test]
    fn pricing_known_models() {
        let pro = pricing_for_gemini("gemini-2.5-pro").unwrap();
        assert!((pro.input_per_mtok - 1.25).abs() < 0.001);
        let flash = pricing_for_gemini("gemini-2.5-flash").unwrap();
        assert!(flash.input_per_mtok < pro.input_per_mtok);
        assert!(pricing_for_gemini("nonexistent").is_none());
    }

    #[tokio::test]
    async fn happy_path_streaming() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"/v1beta/models/.*:streamGenerateContent.*".into()),
            )
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"## Summary\\n\\nA Gemini summary.\\n\\n## Files touched\\n\\n- src/foo.rs\\n\"}]}}]}\n\n\
                 data: {\"candidates\":[{\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":40,\"candidatesTokenCount\":25}}\n\n",
            )
            .create_async()
            .await;

        let provider = GeminiProvider::with_key("test-key".into(), Some("gemini-2.5-pro".into()))
            .with_base_url(server.url());

        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert!(res.summary.summary.contains("Gemini summary"));
        assert_eq!(res.summary.files_touched, vec!["src/foo.rs"]);
        let u = res.usage.unwrap();
        assert_eq!(u.input_tokens, 40);
        assert_eq!(u.output_tokens, 25);
    }

    #[tokio::test]
    async fn auth_invalid_on_403() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", mockito::Matcher::Regex(r"/v1beta/models/.*".into()))
            .with_status(403)
            .with_body("{\"error\":{\"message\":\"forbidden\"}}")
            .create_async()
            .await;

        let provider = GeminiProvider::with_key("bad".into(), Some("gemini-2.5-pro".into()))
            .with_base_url(server.url());
        let err = provider.compact_async(&dummy_session()).await.unwrap_err();
        assert!(matches!(err, CompactionError::AuthInvalid));
    }

    #[tokio::test]
    async fn rate_limit_then_success() {
        let mut server = mockito::Server::new_async().await;
        let _m1 = server
            .mock("POST", mockito::Matcher::Regex(r"/v1beta/models/.*".into()))
            .with_status(429)
            .with_header("retry-after", "1")
            .with_body("{}")
            .expect(1)
            .create_async()
            .await;
        let _m2 = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"/v1beta/models/.*".into()),
            )
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"## Summary\\n\\nOK\"}]}}],\"usageMetadata\":{\"promptTokenCount\":1,\"candidatesTokenCount\":1}}\n\n",
            )
            .create_async()
            .await;

        let provider = GeminiProvider::with_key("k".into(), Some("gemini-2.5-pro".into()))
            .with_base_url(server.url());
        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert!(res.summary.summary.contains("OK"));
    }

    #[tokio::test]
    async fn parses_usage_metadata() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"/v1beta/models/.*".into()),
            )
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"## Summary\\n\\ndone\"}]}}],\"usageMetadata\":{\"promptTokenCount\":100,\"candidatesTokenCount\":50}}\n\n",
            )
            .create_async()
            .await;
        let provider = GeminiProvider::with_key("k".into(), Some("gemini-2.5-flash".into()))
            .with_base_url(server.url());
        let res = provider.compact_async(&dummy_session()).await.unwrap();
        let u = res.usage.unwrap();
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.output_tokens, 50);
        assert!(u.cost_usd > 0.0);
    }
}
