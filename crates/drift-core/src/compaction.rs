//! Compaction: turn a [`NormalizedSession`] into a short Markdown record.
//!
//! Two providers:
//!
//! * [`MockProvider`] — deterministic, fully offline. Used in tests and as a
//!   documented fallback; it never leaves the `[MOCK]` tag behind, so callers
//!   can see at a glance which compactions ran against the real model.
//! * [`AnthropicProvider`] — POSTs `/v1/messages?stream=true` to
//!   `api.anthropic.com`, consumes the SSE stream for live progress on
//!   stderr, logs token usage, and classifies errors (auth / rate /
//!   context-too-long / transient) so the CLI can render a human message
//!   rather than a stack trace.
//!
//! Secrets are read from environment (`ANTHROPIC_API_KEY`). A missing key is
//! not an error for library users — [`AnthropicProvider::try_new`] returns
//! `None` and the caller falls back to [`MockProvider`].

use crate::model::{AgentSlug, NormalizedSession};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write as _;
use std::time::Duration;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors + Usage + Result
// ---------------------------------------------------------------------------

/// Typed error surface for compaction. Each variant maps to a distinct
/// operator-visible message in the CLI; callers should match exhaustively
/// rather than stringifying.
#[derive(Debug, Error)]
pub enum CompactionError {
    #[error("Anthropic API key was rejected (401). Rotate via https://console.anthropic.com/settings/keys and re-export ANTHROPIC_API_KEY.")]
    AuthInvalid,

    #[error("Rate-limited by the Anthropic API (429); give up after 5 retries. Retry-After: {retry_after:?}.")]
    RateLimited { retry_after: Option<Duration> },

    #[error("Model `{0}` not found. Pick one from `https://docs.anthropic.com/en/docs/models`.")]
    ModelNotFound(String),

    #[error("Session has {tokens} tokens but model context limit is {limit}. Shrink the session, switch to a larger-context model, or enable hierarchical summarization in config.")]
    ContextTooLong { tokens: u32, limit: u32 },

    #[error("Network error talking to api.anthropic.com: {0}")]
    TransientNetwork(#[from] reqwest::Error),

    #[error("Streaming parse error: {0}")]
    Stream(String),

    #[error("Unexpected compaction failure: {0}")]
    Other(#[from] anyhow::Error),
}

pub type CompactionRes<T> = std::result::Result<T, CompactionError>;

/// Per-call usage metadata, filled in by providers that talk to a real API.
/// [`MockProvider`] returns `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionUsage {
    pub id: String,
    pub session_id: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_tokens: u32,
    pub cache_read_tokens: u32,
    pub cost_usd: f64,
    pub called_at: DateTime<Utc>,
}

/// What `compact()` hands back: the summary to render and, for real
/// providers, the billing record to persist.
#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub summary: CompactedSummary,
    pub usage: Option<CompactionUsage>,
}

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
    fn compact(&self, session: &NormalizedSession) -> CompactionRes<CompactionResult>;
}

// ---------------------------------------------------------------------------
// MockProvider — deterministic fallback
// ---------------------------------------------------------------------------

/// Deterministic compactor used for tests and for every run where
/// `ANTHROPIC_API_KEY` is unset. Output labels itself `[MOCK]` so you never
/// confuse a fallback compaction for a real one.
pub struct MockProvider;

impl CompactionProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn compact(&self, session: &NormalizedSession) -> CompactionRes<CompactionResult> {
        let files_touched = collect_files_touched(session);
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
        Ok(CompactionResult {
            summary: CompactedSummary {
                session_id: session.session_id.clone(),
                agent_slug: session.agent_slug,
                model: session.model.clone(),
                turn_count: session.turns.len() as u32,
                summary,
                key_decisions: vec![],
                files_touched,
                open_threads: vec![],
                rejected_approaches: vec![],
            },
            usage: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Model metadata — context windows + pricing
// ---------------------------------------------------------------------------

/// Per-million-token USD pricing (input / output / cache-creation / cache-read).
/// Values reflect Anthropic's public price list at the time of v0.1.1 ship;
/// they can drift — cross-check against https://www.anthropic.com/pricing
/// before trusting cost reports as invoice-ready.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_creation_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub context_window: u32,
}

pub fn pricing_for(model: &str) -> ModelPricing {
    match model {
        m if m.starts_with("claude-opus-4-7") => ModelPricing {
            input_per_mtok: 15.0,
            output_per_mtok: 75.0,
            cache_creation_per_mtok: 18.75,
            cache_read_per_mtok: 1.50,
            context_window: 200_000,
        },
        m if m.starts_with("claude-opus-4-6") || m.starts_with("claude-opus-4-5") => ModelPricing {
            input_per_mtok: 15.0,
            output_per_mtok: 75.0,
            cache_creation_per_mtok: 18.75,
            cache_read_per_mtok: 1.50,
            context_window: 200_000,
        },
        m if m.starts_with("claude-sonnet-4-6") || m.starts_with("claude-sonnet-4-5") => {
            ModelPricing {
                input_per_mtok: 3.0,
                output_per_mtok: 15.0,
                cache_creation_per_mtok: 3.75,
                cache_read_per_mtok: 0.30,
                context_window: 1_000_000,
            }
        }
        m if m.starts_with("claude-haiku-4-5") => ModelPricing {
            input_per_mtok: 1.0,
            output_per_mtok: 5.0,
            cache_creation_per_mtok: 1.25,
            cache_read_per_mtok: 0.10,
            context_window: 200_000,
        },
        // Unknown / future model — use Opus-class rates + 200K window as a
        // conservative default. Caller should prefer an explicit match.
        _ => ModelPricing {
            input_per_mtok: 15.0,
            output_per_mtok: 75.0,
            cache_creation_per_mtok: 18.75,
            cache_read_per_mtok: 1.50,
            context_window: 200_000,
        },
    }
}

pub fn compute_cost_usd(model: &str, input: u32, output: u32, cache_c: u32, cache_r: u32) -> f64 {
    let p = pricing_for(model);
    let per = 1_000_000.0;
    (input as f64 / per) * p.input_per_mtok
        + (output as f64 / per) * p.output_per_mtok
        + (cache_c as f64 / per) * p.cache_creation_per_mtok
        + (cache_r as f64 / per) * p.cache_read_per_mtok
}

// ---------------------------------------------------------------------------
// AnthropicProvider
// ---------------------------------------------------------------------------

/// Live Anthropic Messages API provider. Streaming enabled; on CLI the
/// delta text is echoed to stderr with `\r` progress so the user sees
/// compaction happening. On MCP / batch callers, the final aggregated text
/// is what matters and stderr is unused.
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub http: reqwest::Client,
    pub progress_to_stderr: bool,
}

impl AnthropicProvider {
    /// Build a provider from env + optional model override. Returns `None`
    /// if `ANTHROPIC_API_KEY` is unset — callers should fall back to
    /// [`MockProvider`].
    pub fn try_new(model: Option<String>) -> Option<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(Self {
            api_key,
            base_url: "https://api.anthropic.com".to_string(),
            model: model.unwrap_or_else(|| "claude-opus-4-7".to_string()),
            max_tokens: 4096,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .user_agent(concat!("drift_ai/", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            progress_to_stderr: std::env::var("DRIFT_COMPACT_QUIET").is_err(),
        })
    }

    /// Override the base URL (used by tests to point at `mockito`).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_progress(mut self, to_stderr: bool) -> Self {
        self.progress_to_stderr = to_stderr;
        self
    }

    /// Async variant of [`compact`]. Public so async callers (and tests
    /// already inside a runtime) can invoke it directly.
    pub async fn compact_async(
        &self,
        session: &NormalizedSession,
    ) -> CompactionRes<CompactionResult> {
        let pricing = pricing_for(&self.model);

        // Build the user-facing prompt + system message.
        let (prompt, estimated_tokens, truncated) =
            build_prompt_with_truncation(session, pricing.context_window);

        if let Some(over) = prompt_too_long(estimated_tokens, pricing.context_window) {
            return Err(CompactionError::ContextTooLong {
                tokens: estimated_tokens,
                limit: over,
            });
        }

        let url = format!("{}/v1/messages", self.base_url);

        let system = "You compact AI-assisted coding sessions into concise markdown \
                      records. Output ONLY the markdown with the sections requested; \
                      no preamble, no chain-of-thought, no commentary.";

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "stream": true,
            "system": system,
            "messages": [{"role": "user", "content": prompt}],
        });

        // Retry policy: 5 tries for 429 (honouring Retry-After), 4 tries for
        // 5xx / network errors with exponential backoff, instant-fail for
        // 401 / 404.
        let mut attempts = 0u32;
        let max_attempts = 5;
        loop {
            attempts += 1;
            let resp = self
                .http
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await;

            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    if attempts >= max_attempts || !e.is_timeout() && !e.is_connect() {
                        return Err(CompactionError::TransientNetwork(e));
                    }
                    let wait = backoff(attempts);
                    progress_line(
                        self.progress_to_stderr,
                        &format!("network error, retrying in {:?} ...", wait),
                    );
                    tokio::time::sleep(wait).await;
                    continue;
                }
            };

            let status = resp.status();
            if status.is_success() {
                return self.consume_stream(resp, session, truncated).await;
            }

            // Error classification
            let retry_after = parse_retry_after(resp.headers());
            let body_text = resp.text().await.unwrap_or_default();

            if status.as_u16() == 401 {
                return Err(CompactionError::AuthInvalid);
            }
            if status.as_u16() == 404 && body_text.to_lowercase().contains("model") {
                return Err(CompactionError::ModelNotFound(self.model.clone()));
            }
            if status.as_u16() == 429 {
                if attempts >= max_attempts {
                    return Err(CompactionError::RateLimited { retry_after });
                }
                let wait = retry_after.unwrap_or_else(|| backoff(attempts));
                progress_line(
                    self.progress_to_stderr,
                    &format!(
                        "rate-limited (attempt {}/{}), retrying in {:?} ...",
                        attempts, max_attempts, wait
                    ),
                );
                tokio::time::sleep(wait).await;
                continue;
            }
            if status.as_u16() == 400 && body_text.to_lowercase().contains("context") {
                return Err(CompactionError::ContextTooLong {
                    tokens: estimated_tokens,
                    limit: pricing.context_window,
                });
            }
            if status.is_server_error() {
                if attempts >= 4 {
                    return Err(CompactionError::Other(anyhow::anyhow!(
                        "Anthropic server error {}: {}",
                        status,
                        body_text
                    )));
                }
                let wait = backoff(attempts);
                progress_line(
                    self.progress_to_stderr,
                    &format!("5xx ({}), retrying in {:?} ...", status, wait),
                );
                tokio::time::sleep(wait).await;
                continue;
            }

            return Err(CompactionError::Other(anyhow::anyhow!(
                "unexpected Anthropic response {}: {}",
                status,
                body_text
            )));
        }
    }

    async fn consume_stream(
        &self,
        resp: reqwest::Response,
        session: &NormalizedSession,
        truncated_turns: u32,
    ) -> CompactionRes<CompactionResult> {
        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::with_capacity(8192);
        let mut accumulated = String::new();
        let mut input_tokens: u32 = 0;
        let mut output_tokens: u32 = 0;
        let mut cache_c: u32 = 0;
        let mut cache_r: u32 = 0;
        let mut message_done = false;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(CompactionError::TransientNetwork)?;
            buf.extend_from_slice(&bytes);

            // Split on blank-line event boundaries (\n\n).
            while let Some(idx) = find_double_newline(&buf) {
                let event_bytes: Vec<u8> = buf.drain(..idx + 2).collect();
                let event_str = String::from_utf8_lossy(&event_bytes);

                // Each block is a set of `field: value` lines. We only look
                // at `data:` — the `event:` name duplicates what's in the
                // JSON body under `"type"`, and Anthropic guarantees it.
                for line in event_str.lines() {
                    if let Some(rest) = line.strip_prefix("data:") {
                        let data = rest.trim();
                        if data.is_empty() {
                            continue;
                        }
                        if data == "[DONE]" {
                            message_done = true;
                            continue;
                        }
                        let v: serde_json::Value = match serde_json::from_str(data) {
                            Ok(v) => v,
                            Err(e) => {
                                return Err(CompactionError::Stream(format!(
                                    "bad JSON in SSE data: {} / {}",
                                    e, data
                                )))
                            }
                        };
                        let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        match ty {
                            "message_start" => {
                                if let Some(u) = v.pointer("/message/usage") {
                                    input_tokens =
                                        u.get("input_tokens").and_then(|x| x.as_u64()).unwrap_or(0)
                                            as u32;
                                    cache_c = u
                                        .get("cache_creation_input_tokens")
                                        .and_then(|x| x.as_u64())
                                        .unwrap_or(0)
                                        as u32;
                                    cache_r = u
                                        .get("cache_read_input_tokens")
                                        .and_then(|x| x.as_u64())
                                        .unwrap_or(0)
                                        as u32;
                                }
                            }
                            "content_block_delta" => {
                                if let Some(text) =
                                    v.pointer("/delta/text").and_then(|t| t.as_str())
                                {
                                    accumulated.push_str(text);
                                    progress_chunk(self.progress_to_stderr, text);
                                }
                            }
                            "message_delta" => {
                                if let Some(ot) =
                                    v.pointer("/usage/output_tokens").and_then(|x| x.as_u64())
                                {
                                    output_tokens = ot as u32;
                                }
                            }
                            "message_stop" => {
                                message_done = true;
                            }
                            "error" => {
                                let msg = v
                                    .pointer("/error/message")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("unknown streaming error");
                                return Err(CompactionError::Stream(msg.to_string()));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if self.progress_to_stderr {
            let _ = writeln!(std::io::stderr()); // terminate progress line
        }

        if !message_done {
            return Err(CompactionError::Stream(
                "stream ended without message_stop".into(),
            ));
        }

        // Parse the accumulated markdown into structured fields.
        let summary = parse_summary_markdown(
            &accumulated,
            &session.session_id,
            session.agent_slug,
            self.model.as_str(),
            session.turns.len() as u32,
            truncated_turns,
        );
        let cost = compute_cost_usd(&self.model, input_tokens, output_tokens, cache_c, cache_r);

        Ok(CompactionResult {
            summary,
            usage: Some(CompactionUsage {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: session.session_id.clone(),
                model: self.model.clone(),
                input_tokens,
                output_tokens,
                cache_creation_tokens: cache_c,
                cache_read_tokens: cache_r,
                cost_usd: cost,
                called_at: Utc::now(),
            }),
        })
    }
}

impl CompactionProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn compact(&self, session: &NormalizedSession) -> CompactionRes<CompactionResult> {
        // Run the async code on an ad-hoc current-thread runtime. `compact`
        // is sync in its trait signature because capture / CLI code is sync,
        // and wrapping here keeps that contract intact without forcing the
        // whole CLI into tokio.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CompactionError::Other(anyhow::anyhow!(e)))?;
        rt.block_on(self.compact_async(session))
    }
}

// ---------------------------------------------------------------------------
// Prompt construction + token estimation + truncation
// ---------------------------------------------------------------------------

const TEMPLATE: &str = include_str!("../templates/compaction.md");

fn render_prompt(session: &NormalizedSession, truncated_middle: Option<u32>) -> String {
    let transcript = render_transcript(session, truncated_middle);
    TEMPLATE
        .replace("{{session_id}}", &session.session_id)
        .replace("{{agent_slug}}", session.agent_slug.as_str())
        .replace("{{model}}", session.model.as_deref().unwrap_or(""))
        .replace("{{transcript}}", &transcript)
}

fn render_transcript(session: &NormalizedSession, truncated_middle: Option<u32>) -> String {
    let turns = &session.turns;
    let mut out = String::new();
    match truncated_middle {
        None => {
            for t in turns {
                out.push_str(&format!(
                    "### {:?} @ {}\n{}\n\n",
                    t.role,
                    t.timestamp.to_rfc3339(),
                    t.content_text
                ));
            }
        }
        Some(middle_elided) => {
            let keep = 8;
            let head = turns.iter().take(keep);
            let tail = turns.iter().skip(turns.len().saturating_sub(keep));
            for t in head {
                out.push_str(&format!(
                    "### {:?} @ {}\n{}\n\n",
                    t.role,
                    t.timestamp.to_rfc3339(),
                    t.content_text
                ));
            }
            out.push_str(&format!(
                "\n[TRUNCATED: {} turns elided from the middle to fit context window]\n\n",
                middle_elided
            ));
            for t in tail {
                out.push_str(&format!(
                    "### {:?} @ {}\n{}\n\n",
                    t.role,
                    t.timestamp.to_rfc3339(),
                    t.content_text
                ));
            }
        }
    }
    out
}

/// Rough token estimate. Anthropic's BPE tokenizer runs ~3.8 chars/token
/// for English prose and ~3.0 for code-heavy transcripts; 3.3 is a
/// conservative middle ground we use to decide truncation thresholds.
/// Good enough for 80%-of-context decisions; not a billing replacement —
/// actual billable tokens come back in the `usage` response from the API.
pub fn estimate_tokens(s: &str) -> u32 {
    ((s.chars().count() as f64) / 3.3) as u32
}

/// Returns (prompt, estimated_tokens, turns_truncated). If the full
/// transcript fits under 80% of the model's context window, returns it
/// verbatim; otherwise applies Strategy 1 (keep head + tail, elide middle).
fn build_prompt_with_truncation(
    session: &NormalizedSession,
    context_window: u32,
) -> (String, u32, u32) {
    let full = render_prompt(session, None);
    let full_tokens = estimate_tokens(&full);
    let ceiling = (context_window as f64 * 0.80) as u32;
    if full_tokens <= ceiling {
        return (full, full_tokens, 0);
    }
    // Need to drop middle turns. Keep head(8) + tail(8), elide rest.
    let keep_each = 8usize;
    let total = session.turns.len();
    let elided = total.saturating_sub(keep_each * 2) as u32;
    let truncated = render_prompt(session, Some(elided));
    let truncated_tokens = estimate_tokens(&truncated);
    (truncated, truncated_tokens, elided)
}

fn prompt_too_long(tokens: u32, window: u32) -> Option<u32> {
    if tokens > window {
        Some(window)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Response parsing (loose markdown → CompactedSummary)
// ---------------------------------------------------------------------------

fn parse_summary_markdown(
    md: &str,
    session_id: &str,
    agent_slug: AgentSlug,
    model: &str,
    turn_count: u32,
    truncated_turns: u32,
) -> CompactedSummary {
    let sections = split_sections(md);

    let summary = sections
        .get("summary")
        .cloned()
        .unwrap_or_else(|| md.lines().take(3).collect::<Vec<_>>().join("\n"))
        .trim()
        .to_string();
    let key_decisions = sections
        .get("key decisions")
        .map(|s| extract_bullets(s))
        .unwrap_or_default();
    let files_touched = sections
        .get("files touched")
        .map(|s| extract_bullets(s))
        .unwrap_or_default();
    let rejected = sections
        .get("rejected approaches")
        .map(|s| extract_bullets(s))
        .unwrap_or_default();
    let open = sections
        .get("open threads")
        .map(|s| extract_bullets(s))
        .unwrap_or_default();

    let summary_with_note = if truncated_turns > 0 {
        format!(
            "{summary}\n\n> Note: {truncated_turns} turn(s) were elided from the middle of the transcript to fit the model's context window."
        )
    } else {
        summary
    };

    CompactedSummary {
        session_id: session_id.to_string(),
        agent_slug,
        model: Some(model.to_string()),
        turn_count,
        summary: summary_with_note,
        key_decisions,
        files_touched,
        open_threads: open,
        rejected_approaches: rejected,
    }
}

fn split_sections(md: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_buf = String::new();
    for line in md.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("## ") {
            if let Some(k) = current_key.take() {
                out.insert(k, std::mem::take(&mut current_buf));
            }
            current_key = Some(normalize_heading(rest));
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            if let Some(k) = current_key.take() {
                out.insert(k, std::mem::take(&mut current_buf));
            }
            current_key = Some(normalize_heading(rest));
        } else if let Some(num_heading) = strip_numbered_heading(trimmed) {
            if let Some(k) = current_key.take() {
                out.insert(k, std::mem::take(&mut current_buf));
            }
            current_key = Some(num_heading);
        } else if current_key.is_some() {
            current_buf.push_str(line);
            current_buf.push('\n');
        }
    }
    if let Some(k) = current_key {
        out.insert(k, current_buf);
    }
    out
}

fn normalize_heading(s: &str) -> String {
    s.trim()
        .trim_end_matches(':')
        .trim_matches(|c: char| c == '*' || c == '_')
        .to_lowercase()
}

// Matches lines like "1. **Summary**" or "1. Summary —" and returns the
// normalized heading name.
fn strip_numbered_heading(s: &str) -> Option<String> {
    let mut chars = s.chars();
    let mut saw_digit = false;
    for c in chars.by_ref() {
        if c.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if !saw_digit {
            return None;
        }
        if c == '.' {
            break;
        } else {
            return None;
        }
    }
    if !saw_digit {
        return None;
    }
    let rest: String = chars.collect();
    let rest = rest.trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| *c != '—' && *c != '-' && *c != ':')
        .collect();
    let n = normalize_heading(&name);
    if n.is_empty() {
        None
    } else {
        Some(n)
    }
}

fn extract_bullets(s: &str) -> Vec<String> {
    s.lines()
        .filter_map(|l| {
            let t = l.trim_start();
            t.strip_prefix("- ")
                .or_else(|| t.strip_prefix("* "))
                .map(|rest| rest.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_files_touched(session: &NormalizedSession) -> Vec<String> {
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
}

fn short(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

fn find_double_newline(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\n\n")
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let v = headers.get(reqwest::header::RETRY_AFTER)?;
    let s = v.to_str().ok()?;
    if let Ok(secs) = s.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    // HTTP-date form → rough parse; Anthropic usually sends seconds.
    None
}

fn backoff(attempt: u32) -> Duration {
    let secs = 2u64.pow(attempt.saturating_sub(1).min(6));
    Duration::from_secs(secs)
}

fn progress_line(enabled: bool, msg: &str) {
    if !enabled {
        return;
    }
    let _ = write!(std::io::stderr(), "\r\x1b[2K{}", msg);
    let _ = std::io::stderr().flush();
}

fn progress_chunk(enabled: bool, chunk: &str) {
    if !enabled {
        return;
    }
    let preview: String = chunk
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .collect();
    let _ = write!(std::io::stderr(), "\r\x1b[2K… {} ", preview);
    let _ = std::io::stderr().flush();
}

// ---------------------------------------------------------------------------
// Markdown rendering (stable across providers)
// ---------------------------------------------------------------------------

/// Serialise a [`CompactedSummary`] into a Markdown file with YAML
/// frontmatter suitable for committing under `.prompts/sessions/`.
pub fn summary_to_markdown(s: &CompactedSummary) -> String {
    let frontmatter = serde_yaml_like(s);
    let mut body = String::new();
    body.push_str(&format!(
        "---\n{}---\n\n# {} — {}\n\n",
        frontmatter,
        s.session_id,
        s.agent_slug.as_str()
    ));
    body.push_str("## Summary\n\n");
    body.push_str(s.summary.trim());
    body.push_str("\n\n");

    body.push_str("## Key decisions\n\n");
    if s.key_decisions.is_empty() {
        body.push_str("_(none recorded)_\n\n");
    } else {
        for d in &s.key_decisions {
            body.push_str(&format!("- {}\n", d));
        }
        body.push('\n');
    }

    body.push_str("## Files touched\n\n");
    if s.files_touched.is_empty() {
        body.push_str("_(none)_\n\n");
    } else {
        for f in &s.files_touched {
            body.push_str(&format!("- `{}`\n", f));
        }
        body.push('\n');
    }

    body.push_str("## Rejected approaches\n\n");
    if s.rejected_approaches.is_empty() {
        body.push_str("_(none)_\n\n");
    } else {
        for r in &s.rejected_approaches {
            body.push_str(&format!("- {}\n", r));
        }
        body.push('\n');
    }

    body.push_str("## Open threads\n\n");
    if s.open_threads.is_empty() {
        body.push_str("_(none)_\n");
    } else {
        for o in &s.open_threads {
            body.push_str(&format!("- {}\n", o));
        }
    }

    body
}

fn serde_yaml_like(s: &CompactedSummary) -> String {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NormalizedSession, Role, ToolCall, Turn};
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    fn dummy_session() -> NormalizedSession {
        NormalizedSession {
            session_id: "abcdef12-3456".into(),
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
        let r = MockProvider.compact(&ns).unwrap();
        assert_eq!(r.summary.files_touched, vec!["x.txt"]);
        assert!(r.summary.summary.starts_with("[MOCK]"));
        assert!(r.usage.is_none());
    }

    #[test]
    fn markdown_frontmatter_has_session_id() {
        let ns = dummy_session();
        let r = MockProvider.compact(&ns).unwrap();
        let md = summary_to_markdown(&r.summary);
        assert!(md.contains("session_id: \"abcdef12-3456\""));
        assert!(md.starts_with("---"));
    }

    #[test]
    fn pricing_known_models() {
        let opus = pricing_for("claude-opus-4-7");
        assert!((opus.input_per_mtok - 15.0).abs() < 0.01);
        let haiku = pricing_for("claude-haiku-4-5-20251001");
        assert!((haiku.input_per_mtok - 1.0).abs() < 0.01);
        let sonnet = pricing_for("claude-sonnet-4-6");
        assert_eq!(sonnet.context_window, 1_000_000);
    }

    #[test]
    fn cost_computation() {
        // 1M input + 1M output on Opus = $15 + $75 = $90
        let c = compute_cost_usd("claude-opus-4-7", 1_000_000, 1_000_000, 0, 0);
        assert!((c - 90.0).abs() < 0.001);
    }

    #[test]
    fn token_estimate_roughly_linear() {
        let s = "a".repeat(3300);
        let t = estimate_tokens(&s);
        assert!((990..=1010).contains(&t));
    }

    #[test]
    fn parse_summary_markdown_structured() {
        let md = "\
## Summary

The session implemented feature X.

## Key decisions

- Chose approach A over B
- Used SQLite not Postgres

## Files touched

- src/foo.rs
- src/bar.rs

## Rejected approaches

- Tried async first

## Open threads

- tests still flake on macOS
";
        let s = parse_summary_markdown(md, "sid", AgentSlug::ClaudeCode, "claude-opus-4-7", 12, 0);
        assert!(s.summary.contains("implemented feature X"));
        assert_eq!(s.key_decisions.len(), 2);
        assert_eq!(s.files_touched, vec!["src/foo.rs", "src/bar.rs"]);
        assert_eq!(s.rejected_approaches, vec!["Tried async first"]);
        assert_eq!(s.open_threads, vec!["tests still flake on macOS"]);
    }

    #[test]
    fn parse_numbered_headings() {
        let md = "\
1. **Summary**

   did stuff

2. **Key decisions**

   - chose A
";
        let s = parse_summary_markdown(md, "sid", AgentSlug::ClaudeCode, "claude-opus-4-7", 1, 0);
        assert!(s.summary.contains("did stuff"));
        assert_eq!(s.key_decisions, vec!["chose A"]);
    }

    #[test]
    fn truncation_kicks_in_above_80_percent() {
        // Build a session whose transcript would obviously exceed 80% of a
        // tiny fake context window (we don't actually call the API here).
        let mut ns = dummy_session();
        for i in 0..50 {
            ns.turns.push(Turn {
                turn_id: format!("t{}", i),
                role: Role::User,
                content_text: "x".repeat(1000),
                tool_calls: vec![],
                tool_results: vec![],
                timestamp: ns.started_at,
            });
        }
        let (prompt, _, elided) = build_prompt_with_truncation(&ns, 1000);
        assert!(elided > 0, "expected truncation with tight window");
        assert!(prompt.contains("[TRUNCATED:"));
    }

    #[test]
    fn double_newline_finder() {
        assert_eq!(find_double_newline(b"abc\n\ndef"), Some(3));
        assert_eq!(find_double_newline(b"abc"), None);
    }

    #[test]
    fn anthropic_provider_missing_key_returns_none() {
        let prior = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(AnthropicProvider::try_new(None).is_none());
        if let Some(v) = prior {
            std::env::set_var("ANTHROPIC_API_KEY", v);
        }
    }

    #[tokio::test]
    async fn anthropic_sse_streaming_happy_path() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "event: message_start\n\
                 data: {\"type\":\"message_start\",\"message\":{\"id\":\"m1\",\"usage\":{\"input_tokens\":120,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0}}}\n\n\
                 event: content_block_delta\n\
                 data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"## Summary\\n\\nA real summary.\\n\\n## Files touched\\n\\n- src/main.rs\\n\"}}\n\n\
                 event: message_delta\n\
                 data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":42}}\n\n\
                 event: message_stop\n\
                 data: {\"type\":\"message_stop\"}\n\n",
            )
            .create_async()
            .await;

        std::env::set_var("ANTHROPIC_API_KEY", "sk-test");
        std::env::set_var("DRIFT_COMPACT_QUIET", "1");
        let provider = AnthropicProvider::try_new(Some("claude-opus-4-7".into()))
            .unwrap()
            .with_base_url(server.url())
            .with_progress(false);

        let ns = dummy_session();
        let res = provider.compact_async(&ns).await.expect("should succeed");
        assert!(res.summary.summary.contains("real summary"));
        assert_eq!(res.summary.files_touched, vec!["src/main.rs"]);
        let u = res.usage.unwrap();
        assert_eq!(u.input_tokens, 120);
        assert_eq!(u.output_tokens, 42);
        assert!(u.cost_usd > 0.0);
    }

    #[tokio::test]
    async fn anthropic_429_then_200_retries() {
        let mut server = mockito::Server::new_async().await;
        let _m1 = server
            .mock("POST", "/v1/messages")
            .with_status(429)
            .with_header("retry-after", "1")
            .with_body("{\"error\":{\"message\":\"rate limited\"}}")
            .expect(1)
            .create_async()
            .await;
        let _m2 = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(
                "event: message_start\n\
                 data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0}}}\n\n\
                 event: content_block_delta\n\
                 data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"## Summary\\n\\nOK\"}}\n\n\
                 event: message_delta\n\
                 data: {\"type\":\"message_delta\",\"delta\":{},\"usage\":{\"output_tokens\":1}}\n\n\
                 event: message_stop\n\
                 data: {\"type\":\"message_stop\"}\n\n",
            )
            .create_async()
            .await;

        std::env::set_var("ANTHROPIC_API_KEY", "sk-test");
        std::env::set_var("DRIFT_COMPACT_QUIET", "1");
        let provider = AnthropicProvider::try_new(Some("claude-haiku-4-5".into()))
            .unwrap()
            .with_base_url(server.url())
            .with_progress(false);

        let ns = dummy_session();
        let res = provider
            .compact_async(&ns)
            .await
            .expect("retry should succeed");
        assert!(res.summary.summary.contains("OK"));
    }

    #[tokio::test]
    async fn anthropic_401_maps_to_auth_invalid() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/v1/messages")
            .with_status(401)
            .with_body("{\"error\":{\"message\":\"invalid x-api-key\"}}")
            .create_async()
            .await;

        std::env::set_var("ANTHROPIC_API_KEY", "sk-bogus");
        std::env::set_var("DRIFT_COMPACT_QUIET", "1");
        let provider = AnthropicProvider::try_new(Some("claude-opus-4-7".into()))
            .unwrap()
            .with_base_url(server.url())
            .with_progress(false);

        let ns = dummy_session();
        let err = provider.compact_async(&ns).await.unwrap_err();
        assert!(matches!(err, CompactionError::AuthInvalid));
    }
}
