//! Compaction provider trait + Mock (deterministic) and Anthropic (real API)
//! implementations.
//!
//! Secrets are read from environment (`ANTHROPIC_API_KEY`). A missing key is
//! not an error — callers fall back to [`MockProvider`].

use crate::model::{AgentSlug, NormalizedSession};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedSummary {
    pub session_id: String,
    pub agent_slug: AgentSlug,
    pub model: Option<String>,
    pub turn_count: u32,
    pub summary: String,
    pub key_decisions: Vec<String>,
    pub files_touched: Vec<String>,
    pub open_threads: Vec<String>,
    pub rejected_approaches: Vec<String>,
}

pub trait CompactionProvider {
    fn name(&self) -> &'static str;
    fn compact(&self, session: &NormalizedSession) -> Result<CompactedSummary>;
}

/// Deterministic compactor used for tests and for every run where
/// `ANTHROPIC_API_KEY` is unset. Output labels itself `[MOCK]`.
pub struct MockProvider;

impl CompactionProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn compact(&self, session: &NormalizedSession) -> Result<CompactedSummary> {
        let files_touched = {
            let mut fs: Vec<String> = session
                .turns
                .iter()
                .flat_map(|t| t.tool_calls.iter())
                .filter_map(|tc| {
                    tc.input
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .collect();
            fs.sort();
            fs.dedup();
            fs
        };

        let summary = format!(
            "[MOCK] {} session {} with {} turns; files touched: {}",
            session.agent_slug.as_str(),
            short(&session.session_id),
            session.turns.len(),
            if files_touched.is_empty() {
                "(none)".to_string()
            } else {
                files_touched.join(", ")
            }
        );

        Ok(CompactedSummary {
            session_id: session.session_id.clone(),
            agent_slug: session.agent_slug,
            model: session.model.clone(),
            turn_count: session.turns.len() as u32,
            summary,
            key_decisions: vec![],
            files_touched,
            open_threads: vec![],
            rejected_approaches: vec![],
        })
    }
}

/// Anthropic Messages API provider.
/// Reads `ANTHROPIC_API_KEY` from env; if missing, [`AnthropicProvider::try_new`]
/// returns None — callers should fall back to [`MockProvider`].
pub struct AnthropicProvider {
    pub api_key: String,
    pub model: String,
}

impl AnthropicProvider {
    pub fn try_new(model: Option<String>) -> Option<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
        Some(Self {
            api_key,
            model: model.unwrap_or_else(|| "claude-opus-4-7".to_string()),
        })
    }
}

impl CompactionProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn compact(&self, session: &NormalizedSession) -> Result<CompactedSummary> {
        // v0.1.0: we build the prompt but do not ship the HTTP call wired up
        // on this host (no ANTHROPIC_API_KEY available for CI). The shape
        // here is what the wire-up will use once a key is provided; the
        // MockProvider path covers the default release.
        //
        // To enable: add `reqwest = { version = "0.11", features = ["blocking","json"] }`,
        // send POST to https://api.anthropic.com/v1/messages with
        // x-api-key + anthropic-version, parse the content.
        let _prompt = render_prompt(session)?;
        anyhow::bail!(
            "AnthropicProvider HTTP call is not wired up in v0.1.0; \
             unset ANTHROPIC_API_KEY to use MockProvider, or wire up reqwest \
             before calling. See crates/drift-core/src/compaction.rs."
        );
    }
}

fn render_prompt(session: &NormalizedSession) -> Result<String> {
    let template = include_str!("../templates/compaction.md");
    let transcript = session
        .turns
        .iter()
        .map(|t| {
            format!(
                "### {:?} @ {}\n{}\n",
                t.role,
                t.timestamp.to_rfc3339(),
                t.content_text
            )
        })
        .collect::<String>();

    let out = template
        .replace("{{session_id}}", &session.session_id)
        .replace("{{agent_slug}}", session.agent_slug.as_str())
        .replace("{{model}}", session.model.as_deref().unwrap_or(""))
        .replace("{{transcript}}", &transcript);
    Ok(out)
}

fn short(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// Serialise a [`CompactedSummary`] into a Markdown file with YAML
/// frontmatter suitable for committing under `.prompts/sessions/`.
pub fn summary_to_markdown(s: &CompactedSummary) -> String {
    let frontmatter = serde_yaml_like(s);
    format!(
        "---\n{}---\n\n# {} — {}\n\n{}\n",
        frontmatter,
        s.session_id,
        s.agent_slug.as_str(),
        s.summary
    )
}

fn serde_yaml_like(s: &CompactedSummary) -> String {
    // Tiny hand-rolled YAML frontmatter to avoid pulling in serde_yaml for
    // one use case. Keys are fixed and values are escaped by JSON encoding
    // which is a YAML-compatible subset for scalars.
    let mut out = String::new();
    out.push_str(&format!("session_id: {}\n", json_scalar(&s.session_id)));
    out.push_str(&format!(
        "agent_slug: {}\n",
        json_scalar(s.agent_slug.as_str())
    ));
    if let Some(m) = &s.model {
        out.push_str(&format!("model: {}\n", json_scalar(m)));
    }
    out.push_str(&format!("turn_count: {}\n", s.turn_count));
    out.push_str("files_touched:\n");
    for f in &s.files_touched {
        out.push_str(&format!("  - {}\n", json_scalar(f)));
    }
    out.push_str("rejected_approaches:\n");
    for r in &s.rejected_approaches {
        out.push_str(&format!("  - {}\n", json_scalar(r)));
    }
    out
}

fn json_scalar(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s.replace('"', "\\\"")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NormalizedSession, Role, ToolCall, Turn};
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    fn dummy_session() -> NormalizedSession {
        NormalizedSession {
            session_id: "abc".into(),
            agent_slug: AgentSlug::ClaudeCode,
            model: Some("claude-opus-4-7".into()),
            working_dir: None,
            git_head_at_start: None,
            started_at: Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 4, 22, 10, 5, 0).unwrap(),
            turns: vec![Turn {
                turn_id: "t1".into(),
                role: Role::Assistant,
                content_text: "hi".into(),
                tool_calls: vec![ToolCall {
                    id: "tc1".into(),
                    name: "Write".into(),
                    input: json!({"file_path": "x.txt", "content": "hi\n"}),
                }],
                tool_results: vec![],
                timestamp: Utc.with_ymd_and_hms(2026, 4, 22, 10, 1, 0).unwrap(),
            }],
            thinking_blocks: 0,
        }
    }

    #[test]
    fn mock_provider_lists_files_touched() {
        let ns = dummy_session();
        let cs = MockProvider.compact(&ns).unwrap();
        assert_eq!(cs.files_touched, vec!["x.txt"]);
        assert!(cs.summary.starts_with("[MOCK]"));
    }

    #[test]
    fn markdown_frontmatter_has_session_id() {
        let ns = dummy_session();
        let cs = MockProvider.compact(&ns).unwrap();
        let md = summary_to_markdown(&cs);
        assert!(md.contains("session_id: \"abc\""));
        assert!(md.starts_with("---"));
    }
}
