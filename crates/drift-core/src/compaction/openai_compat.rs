//! OpenAI-protocol generic provider — DeepSeek / Groq / Mistral / Together AI /
//! LM Studio / vLLM / anything else that speaks `chat.completions`.
//!
//! Wire format reuses [`crate::compaction::openai::OpenAIProvider`] directly;
//! the only differences are (1) base URL is user-supplied, (2) cost is
//! user-supplied via per-1M-token rates in config because we deliberately do
//! not maintain a third-party price table (it would go stale within weeks).

use crate::compaction::openai::OpenAIProvider;
use crate::compaction::{
    CompactedSummary, CompactionError, CompactionProvider, CompactionRes, CompactionResult,
    CompactionUsage,
};
use crate::model::NormalizedSession;
use chrono::Utc;

/// User-supplied per-1M-token pricing for cost reporting. If both fields are
/// `None`, `cost_usd` is reported as `0.0` and `drift cost` shows `(unpriced)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct CustomPricing {
    pub input_per_mtok: Option<f64>,
    pub output_per_mtok: Option<f64>,
}

#[derive(Clone)]
pub struct OpenAICompatibleProvider {
    inner: OpenAIProvider,
    pricing: CustomPricing,
    name: String,
}

impl OpenAICompatibleProvider {
    /// Build a generic OpenAI-protocol provider from explicit args. `name`
    /// is the slug shown in `drift cost --by model` (typically the config
    /// key, e.g. "deepseek").
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: Option<String>,
        model: impl Into<String>,
        pricing: CustomPricing,
    ) -> Self {
        let inner = OpenAIProvider::with_key(api_key.unwrap_or_default(), Some(model.into()))
            .with_base_url(base_url);
        Self {
            inner,
            pricing,
            name: name.into(),
        }
    }

    pub async fn compact_async(
        &self,
        session: &NormalizedSession,
    ) -> CompactionRes<CompactionResult> {
        let mut result = self.inner.compact_async(session).await?;
        // Re-stamp the cost using user-supplied pricing.
        if let Some(usage) = result.usage.as_mut() {
            usage.cost_usd = self.cost_for(usage.input_tokens, usage.output_tokens);
        }
        Ok(result)
    }

    fn cost_for(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let p = self.pricing;
        match (p.input_per_mtok, p.output_per_mtok) {
            (Some(i), Some(o)) => {
                (input_tokens as f64 / 1_000_000.0) * i + (output_tokens as f64 / 1_000_000.0) * o
            }
            _ => 0.0,
        }
    }
}

impl CompactionProvider for OpenAICompatibleProvider {
    fn name(&self) -> &'static str {
        // We can't return a borrow of `self.name` from a `&'static str` API;
        // shipping `"openai-compatible"` is fine — `drift cost --by model`
        // groups by the `model` column anyway, which captures the actual model.
        "openai-compatible"
    }
    fn compact(&self, session: &NormalizedSession) -> CompactionRes<CompactionResult> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CompactionError::Other(anyhow::anyhow!(e)))?;
        rt.block_on(self.compact_async(session))
    }
}

// Helper: surface the configured friendly name (e.g. "deepseek") for callers
// that need it (factory diagnostics, cost reports).
impl OpenAICompatibleProvider {
    pub fn config_name(&self) -> &str {
        &self.name
    }
}

// Round-trip helper used by tests + factory to stamp a fake summary when the
// inner provider returns nothing useful.
#[allow(dead_code)]
pub(crate) fn stamp_unpriced_usage(
    summary: CompactedSummary,
    session: &NormalizedSession,
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
) -> CompactionResult {
    CompactionResult {
        summary,
        usage: Some(CompactionUsage {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session.session_id.clone(),
            model: model.to_string(),
            input_tokens,
            output_tokens,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            cost_usd: 0.0,
            called_at: Utc::now(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentSlug, NormalizedSession, Role, Turn};
    use chrono::{TimeZone, Utc};

    fn dummy_session() -> NormalizedSession {
        NormalizedSession {
            session_id: "compat-test".into(),
            agent_slug: AgentSlug::ClaudeCode,
            model: Some("deepseek-chat".into()),
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

    #[tokio::test]
    async fn deepseek_via_openai_compatible_happy_path() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"choices\":[{\"delta\":{\"content\":\"## Summary\\n\\nDeepSeek summary.\\n\\n## Files touched\\n\\n- src/lib.rs\\n\"}}]}\n\n\
                 data: {\"choices\":[{\"finish_reason\":\"stop\",\"delta\":{}}],\"usage\":{\"prompt_tokens\":1000,\"completion_tokens\":200}}\n\n\
                 data: [DONE]\n\n",
            )
            .create_async()
            .await;

        let provider = OpenAICompatibleProvider::new(
            "deepseek",
            server.url(),
            Some("sk-test".into()),
            "deepseek-chat",
            CustomPricing {
                input_per_mtok: Some(0.27),
                output_per_mtok: Some(1.10),
            },
        );

        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert!(res.summary.summary.contains("DeepSeek summary"));
        let u = res.usage.unwrap();
        assert_eq!(u.input_tokens, 1000);
        assert_eq!(u.output_tokens, 200);
        // 1000/1M * 0.27 + 200/1M * 1.10 = 0.00027 + 0.00022 = 0.00049
        assert!((u.cost_usd - 0.00049).abs() < 1e-7);
    }

    #[tokio::test]
    async fn unpriced_returns_zero_cost() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"choices\":[{\"delta\":{\"content\":\"## Summary\\n\\nx\"}}]}\n\n\
                 data: {\"choices\":[{\"finish_reason\":\"stop\",\"delta\":{}}],\"usage\":{\"prompt_tokens\":1000,\"completion_tokens\":1000}}\n\n\
                 data: [DONE]\n\n",
            )
            .create_async()
            .await;
        let provider = OpenAICompatibleProvider::new(
            "groq",
            server.url(),
            Some("k".into()),
            "llama-3.3-70b-versatile",
            CustomPricing::default(),
        );
        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert_eq!(res.usage.unwrap().cost_usd, 0.0);
    }

    #[tokio::test]
    async fn keyless_local_server() {
        // Local OpenAI-compatible servers (LM Studio, vLLM) often have no auth.
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "data: {\"choices\":[{\"delta\":{\"content\":\"## Summary\\n\\nlocal\"}}]}\n\n\
                 data: {\"choices\":[{\"finish_reason\":\"stop\",\"delta\":{}}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":3}}\n\n\
                 data: [DONE]\n\n",
            )
            .create_async()
            .await;
        let provider = OpenAICompatibleProvider::new(
            "lmstudio",
            server.url(),
            None,
            "local-model",
            CustomPricing::default(),
        );
        let res = provider.compact_async(&dummy_session()).await.unwrap();
        assert!(res.summary.summary.contains("local"));
    }

    #[tokio::test]
    async fn auth_invalid_propagates() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/chat/completions")
            .with_status(401)
            .with_body("{\"error\":{\"message\":\"bad\"}}")
            .create_async()
            .await;
        let provider = OpenAICompatibleProvider::new(
            "deepseek",
            server.url(),
            Some("bad".into()),
            "deepseek-chat",
            CustomPricing::default(),
        );
        let err = provider.compact_async(&dummy_session()).await.unwrap_err();
        assert!(matches!(err, CompactionError::AuthInvalid));
    }

    #[test]
    fn cost_for_basic() {
        let p = OpenAICompatibleProvider::new(
            "x",
            "http://localhost:0",
            None,
            "m",
            CustomPricing {
                input_per_mtok: Some(1.0),
                output_per_mtok: Some(2.0),
            },
        );
        // 1M input + 1M output = $1 + $2 = $3
        let c = p.cost_for(1_000_000, 1_000_000);
        assert!((c - 3.0).abs() < 1e-6);
    }
}
