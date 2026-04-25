//! Ollama local-LLM provider (`http://localhost:11434/api/chat`).
//!
//! No auth. Stream is NDJSON (one JSON object per line, terminated by `\n`).
//! Cost is always 0.0 — Ollama is local. We surface a friendly message when
//! the daemon isn't running so users see actionable text rather than a
//! reqwest error string.

use crate::compaction::streaming::for_each_ndjson;
use crate::compaction::{
    backoff, parse_retry_after, CompactionError, CompactionProvider, CompactionRes,
    CompactionResult, CompactionUsage, LlmCompletion,
};
use crate::model::NormalizedSession;
use chrono::Utc;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3.3:70b";
const DEFAULT_MAX_TOKENS: u32 = 4096;

#[derive(Clone)]
pub struct OllamaProvider {
    base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub http: reqwest::Client,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new(DEFAULT_BASE_URL.into(), DEFAULT_MODEL.into())
    }
}

impl OllamaProvider {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            max_tokens: DEFAULT_MAX_TOKENS,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(600))
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
                cost_usd: 0.0,
                called_at: Utc::now(),
            }),
        })
    }

    pub async fn complete_async(&self, system: &str, user: &str) -> CompactionRes<LlmCompletion> {
        let url = format!("{}/api/chat", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "stream": true,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user",   "content": user   },
            ],
            "options": {
                "num_predict": self.max_tokens,
            }
        });

        let mut attempts = 0u32;
        let max_attempts = 4;
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
                    if e.is_connect() {
                        return Err(CompactionError::Other(anyhow::anyhow!(
                            "Ollama is not running on {}. Start it with: ollama serve",
                            self.base_url
                        )));
                    }
                    if attempts >= max_attempts || !e.is_timeout() {
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
            if status.as_u16() == 404
                && (body_text.to_lowercase().contains("model")
                    || body_text.to_lowercase().contains("not found"))
            {
                return Err(CompactionError::ModelNotFound(self.model.clone()));
            }
            // Rare: Ollama can return 429 in some proxy setups.
            if status.as_u16() == 429 {
                if attempts >= max_attempts {
                    return Err(CompactionError::RateLimited { retry_after });
                }
                tokio::time::sleep(backoff(attempts)).await;
                continue;
            }
            if status.is_server_error() {
                if attempts >= max_attempts {
                    return Err(CompactionError::Other(anyhow::anyhow!(
                        "Ollama server error {}: {}",
                        status,
                        body_text
                    )));
                }
                tokio::time::sleep(backoff(attempts)).await;
                continue;
            }

            return Err(CompactionError::Other(anyhow::anyhow!(
                "unexpected Ollama response {}: {}",
                status,
                body_text
            )));
        }
    }

    async fn consume_stream(&self, resp: reqwest::Response) -> CompactionRes<LlmCompletion> {
        use std::cell::RefCell;
        let accumulated = RefCell::new(String::new());
        let prompt_eval = RefCell::new(0u32);
        let eval_count = RefCell::new(0u32);
        let saw_done = RefCell::new(false);

        for_each_ndjson(resp, |v| {
            if let Some(text) = v.pointer("/message/content").and_then(|x| x.as_str()) {
                accumulated.borrow_mut().push_str(text);
            }
            if v.get("done").and_then(|x| x.as_bool()).unwrap_or(false) {
                *saw_done.borrow_mut() = true;
                if let Some(it) = v.get("prompt_eval_count").and_then(|x| x.as_u64()) {
                    *prompt_eval.borrow_mut() = it as u32;
                }
                if let Some(ot) = v.get("eval_count").and_then(|x| x.as_u64()) {
                    *eval_count.borrow_mut() = ot as u32;
                }
            }
            Ok(())
        })
        .await?;

        if !*saw_done.borrow() {
            return Err(CompactionError::Stream(
                "Ollama stream ended without done=true".into(),
            ));
        }

        Ok(LlmCompletion {
            text: accumulated.into_inner(),
            model: self.model.clone(),
            input_tokens: prompt_eval.into_inner(),
            output_tokens: eval_count.into_inner(),
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            cost_usd: 0.0,
        })
    }
}

impl CompactionProvider for OllamaProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }
    fn compact(&self, session: &NormalizedSession) -> CompactionRes<CompactionResult> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CompactionError::Other(anyhow::anyhow!(e)))?;
        rt.block_on(self.compact_async(session))
    }
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
            session_id: "ollama-test".into(),
            agent_slug: AgentSlug::ClaudeCode,
            model: Some("llama3.3:70b".into()),
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
    fn defaults() {
        let p = OllamaProvider::default();
        assert!(p.base_url.starts_with("http://"));
        assert!(p.model.starts_with("llama"));
    }

    #[tokio::test]
    async fn happy_path_ndjson() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_header("content-type", "application/x-ndjson")
            .with_body(
                "{\"message\":{\"content\":\"## Summary\\n\\n\"},\"done\":false}\n\
                 {\"message\":{\"content\":\"Local LLM said hi.\\n\\n## Files touched\\n\\n- src/x.rs\\n\"},\"done\":false}\n\
                 {\"message\":{\"content\":\"\"},\"done\":true,\"prompt_eval_count\":42,\"eval_count\":17}\n",
            )
            .create_async()
            .await;
        let provider = OllamaProvider::default().with_base_url(server.url());
        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert!(res.summary.summary.contains("Local LLM said hi"));
        assert_eq!(res.summary.files_touched, vec!["src/x.rs"]);
        let u = res.usage.unwrap();
        assert_eq!(u.input_tokens, 42);
        assert_eq!(u.output_tokens, 17);
        assert_eq!(u.cost_usd, 0.0);
    }

    #[tokio::test]
    async fn model_not_found() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/api/chat")
            .with_status(404)
            .with_body("{\"error\":\"model 'fake:7b' not found\"}")
            .create_async()
            .await;
        let provider = OllamaProvider::new(server.url(), "fake:7b".into());
        let err = provider.compact_async(&dummy_session()).await.unwrap_err();
        assert!(matches!(err, CompactionError::ModelNotFound(_)));
    }

    #[tokio::test]
    async fn ollama_not_running_friendly_error() {
        // Point at a port that's almost certainly closed.
        let provider = OllamaProvider::new("http://127.0.0.1:1".into(), "llama3:8b".into());
        let err = provider.compact_async(&dummy_session()).await.unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("Ollama is not running") || msg.contains("Network error"),
            "unexpected error: {}",
            msg
        );
    }

    #[tokio::test]
    async fn missing_done_flag_errors() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_body("{\"message\":{\"content\":\"hi\"},\"done\":false}\n")
            .create_async()
            .await;
        let provider = OllamaProvider::default().with_base_url(server.url());
        let err = provider.compact_async(&dummy_session()).await.unwrap_err();
        assert!(matches!(err, CompactionError::Stream(_)));
    }

    #[tokio::test]
    async fn handles_large_content_split_across_chunks() {
        let mut server = mockito::Server::new_async().await;
        let body = "{\"message\":{\"content\":\"## Summary\\n\\nstart \"},\"done\":false}\n\
                    {\"message\":{\"content\":\"middle \"},\"done\":false}\n\
                    {\"message\":{\"content\":\"end\"},\"done\":false}\n\
                    {\"message\":{\"content\":\"\"},\"done\":true,\"prompt_eval_count\":1,\"eval_count\":1}\n";
        let _m = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;
        let provider = OllamaProvider::default().with_base_url(server.url());
        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert!(res.summary.summary.contains("start middle end"));
    }
}
