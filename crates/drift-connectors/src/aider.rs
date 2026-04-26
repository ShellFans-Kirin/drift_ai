//! Aider connector — full implementation.
//!
//! Aider stores conversation history as plain markdown alongside the user's
//! repo:
//!
//! - `<repo>/.aider.chat.history.md` — turn-by-turn user / assistant log.
//! - `<repo>/.aider.input.history`   — raw CLI prompt history (we ignore).
//!
//! Aider also commits directly to git with subject prefix `aider:` (e.g.
//! `aider: add OAuth login`), so we can correlate commits ↔ sessions.
//!
//! [BEST-EFFORT] Aider has no `tool_call` / `tool_result` structure — every
//! assistant turn is just markdown text. We can't reliably distinguish
//! "this diff was applied" from "this diff was suggested but rejected"; we
//! default `rejected = false` and rely on the SHA-256 ladder in
//! `drift_core::attribution` to catch divergence.

use super::{SessionConnector, SessionRef};
use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use drift_core::attribution::CodeEventDraft;
use drift_core::model::{
    AgentSlug, NormalizedSession, Operation, Role, ToolCall, ToolResult, Turn,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub struct AiderConnector {
    /// Repository roots to search for `.aider.chat.history.md`. Defaults to
    /// the current working directory.
    pub roots: Vec<PathBuf>,
}

impl Default for AiderConnector {
    fn default() -> Self {
        Self::with_default_roots()
    }
}

impl AiderConnector {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    /// Default search root: the current working directory. Aider sits in the
    /// repo root, so this is the natural place to look.
    pub fn with_default_roots() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self { roots: vec![cwd] }
    }
}

impl SessionConnector for AiderConnector {
    fn agent_slug(&self) -> &'static str {
        "aider"
    }

    fn discover(&self) -> Result<Vec<SessionRef>> {
        let mut out = Vec::new();
        for root in &self.roots {
            let p = root.join(".aider.chat.history.md");
            if p.exists() {
                out.push(SessionRef {
                    agent_slug: self.agent_slug(),
                    path: p,
                });
            }
        }
        Ok(out)
    }

    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession> {
        let text = std::fs::read_to_string(&r.path)
            .with_context(|| format!("read {}", r.path.display()))?;
        parse_aider_markdown(&text, &r.path)
    }

    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>> {
        Ok(extract_events(ns))
    }
}

// ---------------------------------------------------------------------------
// Markdown parser
// ---------------------------------------------------------------------------

/// Parse Aider's markdown chat history.
///
/// Aider's format: a top-level header `# aider chat started at <ts>`,
/// then alternating user paragraphs (each line prefixed with `> `) and
/// assistant paragraphs. Assistant paragraphs may contain fenced
/// `\`\`\`diff` blocks, which we extract as edits.
///
/// Quirks tolerated: missing header (we fall back to `Utc::now()` for the
/// session timestamp), `\`\`\`patch` as an alias for `\`\`\`diff`, and
/// unterminated fences at EOF.
pub fn parse_aider_markdown(text: &str, source_path: &Path) -> Result<NormalizedSession> {
    let started_at = parse_started_at(text).unwrap_or_else(Utc::now);
    let session_id = synth_session_id(source_path, started_at);

    // First pass: split into "user" and "assistant" blocks. Each `> `
    // user-prefixed paragraph starts a new user turn; everything after,
    // until the next user paragraph, is the assistant turn.
    let mut blocks: Vec<(Role, String)> = Vec::new();
    let mut current_role: Option<Role> = None;
    let mut current_buf = String::new();
    let mut in_fence = false;

    let flush = |role: Option<Role>, buf: &mut String, blocks: &mut Vec<(Role, String)>| {
        if let Some(r) = role {
            if !buf.trim().is_empty() {
                blocks.push((r, std::mem::take(buf)));
            } else {
                buf.clear();
            }
        }
    };

    for line in text.lines() {
        // Toggle code fences so we don't misinterpret a `> ` *inside* a fence.
        if line.starts_with("```") {
            in_fence = !in_fence;
            current_buf.push_str(line);
            current_buf.push('\n');
            continue;
        }

        let is_user_line = !in_fence && line.starts_with("> ");

        if is_user_line {
            // Switching into user role: flush whatever was prior.
            if !matches!(current_role, Some(Role::User)) {
                flush(current_role, &mut current_buf, &mut blocks);
                current_role = Some(Role::User);
            }
            // strip the leading "> " prefix
            current_buf.push_str(&line[2..]);
            current_buf.push('\n');
        } else if line.is_empty() && matches!(current_role, Some(Role::User)) {
            // Blank line ends a user paragraph; switch to assistant.
            flush(current_role, &mut current_buf, &mut blocks);
            current_role = Some(Role::Assistant);
        } else {
            // Anything else: belongs to the current role (initialise to
            // assistant if we haven't seen a user yet).
            if current_role.is_none() {
                current_role = Some(Role::Assistant);
            }
            current_buf.push_str(line);
            current_buf.push('\n');
        }
    }
    flush(current_role, &mut current_buf, &mut blocks);

    // Pair user / assistant blocks into Turns. Preserve order; if assistant
    // appears with no preceding user (unusual), still emit it.
    let mut turns: Vec<Turn> = Vec::new();
    for (i, (role, body)) in blocks.iter().enumerate() {
        let timestamp = started_at;
        let turn_id = format!("aider-{}-{}", short_id(&session_id), i);
        let mut turn = Turn {
            turn_id,
            role: *role,
            content_text: body.trim_end_matches('\n').to_string(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            timestamp,
        };
        if matches!(role, Role::Assistant) {
            // Extract diff hunks → tool calls.
            for (j, block) in extract_diff_blocks(body).into_iter().enumerate() {
                let path = match path_from_diff_header(&block) {
                    Some(p) => p,
                    None => continue,
                };
                let after = synthesise_after_from_hunks(&block);
                let tc_id = format!("aider-edit-{}-{}-{}", short_id(&session_id), i, j);
                let mut input = serde_json::Map::new();
                input.insert("file_path".into(), Value::String(path));
                input.insert("diff".into(), Value::String(block.clone()));
                if !after.is_empty() {
                    input.insert("content".into(), Value::String(after));
                }
                turn.tool_calls.push(ToolCall {
                    id: tc_id.clone(),
                    name: "Edit".into(),
                    input: Value::Object(input),
                });
                // We can't tell from the markdown whether the user
                // accepted the diff, so default to "applied".
                turn.tool_results.push(ToolResult {
                    tool_use_id: tc_id,
                    content: "applied".into(),
                    is_error: false,
                });
            }
        }
        turns.push(turn);
    }

    Ok(NormalizedSession {
        session_id,
        agent_slug: AgentSlug::Aider,
        model: None,
        working_dir: source_path.parent().map(|p| p.to_path_buf()),
        git_head_at_start: None,
        started_at,
        ended_at: started_at,
        turns,
        thinking_blocks: 0,
    })
}

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

fn parse_started_at(text: &str) -> Option<DateTime<Utc>> {
    // Aider's typical header: `# aide chat started at 2026-04-25 10:00:00`
    // or `# aider chat started at 2026-04-25 10:00:00`.
    for line in text.lines().take(20) {
        if let Some(rest) = line.strip_prefix("# aider chat started at ").or(line
            .strip_prefix("# aide chat started at ")
            .or_else(|| line.strip_prefix("# chat started at ")))
        {
            return chrono::NaiveDateTime::parse_from_str(rest.trim(), "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|n| Utc.from_utc_datetime(&n));
        }
    }
    None
}

fn synth_session_id(path: &Path, ts: DateTime<Utc>) -> String {
    let mut h = Sha256::new();
    h.update(path.to_string_lossy().as_bytes());
    h.update(ts.to_rfc3339().as_bytes());
    let bytes = h.finalize();
    let mut s = String::with_capacity(16);
    for b in bytes.iter().take(8) {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn short_id(s: &str) -> &str {
    s.get(..8).unwrap_or(s)
}

/// Extract every fenced ```diff block from an assistant body. Returns the
/// text *inside* the fences (without the fence lines themselves).
pub fn extract_diff_blocks(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_diff = false;
    let mut buf = String::new();
    for line in body.lines() {
        let trimmed_open = line.trim_start();
        if !in_diff
            && (trimmed_open.starts_with("```diff")
                || trimmed_open.starts_with("```patch")
                || trimmed_open == "```")
        {
            in_diff = trimmed_open.starts_with("```diff") || trimmed_open.starts_with("```patch");
            // a bare ``` alone could open a code fence — only flip into
            // "diff mode" for ```diff / ```patch
            if !in_diff {
                continue;
            }
            buf.clear();
            continue;
        }
        if in_diff && line.trim_start() == "```" {
            in_diff = false;
            if !buf.trim().is_empty() {
                out.push(buf.clone());
            }
            buf.clear();
            continue;
        }
        if in_diff {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    if in_diff && !buf.trim().is_empty() {
        // Unterminated diff — still capture
        out.push(buf);
    }
    out
}

/// Parse the file path from a diff header like `+++ b/src/foo.rs`.
pub fn path_from_diff_header(diff: &str) -> Option<String> {
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            return Some(rest.trim().to_string());
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            // Some diffs lack the b/ prefix
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Best-effort reconstruction of the post-diff file content. Aider's diffs
/// are unified-format; we extract the `+`-prefixed lines (excluding the
/// `+++` header) as the new content. This is enough for SHA-ladder hashing
/// to differ from any prior state.
pub fn synthesise_after_from_hunks(diff: &str) -> String {
    let mut out = String::new();
    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if let Some(rest) = line.strip_prefix('+') {
            out.push_str(rest);
            out.push('\n');
        }
    }
    out
}

// ---------------------------------------------------------------------------
// CodeEvent extraction (re-uses the same shape as the Cursor connector)
// ---------------------------------------------------------------------------

fn extract_events(ns: &NormalizedSession) -> Vec<CodeEventDraft> {
    let mut out = Vec::new();
    for t in &ns.turns {
        for (tc_idx, tc) in t.tool_calls.iter().enumerate() {
            let path = match tc.input.get("file_path").and_then(|v| v.as_str()) {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => continue,
            };
            let after_content = tc
                .input
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut metadata = serde_json::Map::new();
            metadata.insert(
                "best_effort".into(),
                Value::String(
                    "aider markdown has no tool_call structure; rejected defaults to false".into(),
                ),
            );
            metadata.insert("tool_call_id".into(), Value::String(tc.id.clone()));
            if let Some(diff) = tc.input.get("diff").and_then(|v| v.as_str()) {
                metadata.insert("diff_text".into(), Value::String(diff.to_string()));
            }
            out.push(CodeEventDraft {
                session_id: Some(ns.session_id.clone()),
                agent_slug: ns.agent_slug,
                turn_id: Some(t.turn_id.clone()),
                timestamp: t.timestamp,
                file_path: path,
                operation: Operation::Edit,
                rename_from: None,
                before_content: String::new(),
                after_content,
                rejected: false, // aider markdown can't distinguish accepted/rejected
                metadata,
                event_id: Some(format!("{}-evt-{}", t.turn_id, tc_idx)),
                intra_call_parent: None,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    const FIXTURE: &str = "\
# aider chat started at 2026-04-25 10:00:00

> please add a rate limiter to login

I'll add a sliding window in src/auth/login.ts:

```diff
--- a/src/auth/login.ts
+++ b/src/auth/login.ts
@@ -1,3 +1,5 @@
+import { RateLimitError } from './errors'
+
 export async function login(req) {
   if (req.attempts > 5) throw new RateLimitError()
   return ok()
```

> looks good. now tests.

Adding tests:

```diff
--- /dev/null
+++ b/src/auth/login.test.ts
@@ -0,0 +1,2 @@
+test('rate limit', () => {})
+test('reset', () => {})
```
";

    fn write_fixture(dir: &Path) -> PathBuf {
        let p = dir.join(".aider.chat.history.md");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(FIXTURE.as_bytes()).unwrap();
        p
    }

    #[test]
    fn discover_finds_aider_history() {
        let dir = tempdir().unwrap();
        write_fixture(dir.path());
        let conn = AiderConnector::new(vec![dir.path().to_path_buf()]);
        let refs = conn.discover().unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].agent_slug, "aider");
    }

    #[test]
    fn discover_handles_missing_root() {
        let conn = AiderConnector::new(vec![PathBuf::from("/no/such/dir")]);
        let refs = conn.discover().unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn parse_extracts_user_assistant_alternation() {
        let dir = tempdir().unwrap();
        let p = write_fixture(dir.path());
        let conn = AiderConnector::new(vec![dir.path().to_path_buf()]);
        let r = SessionRef {
            agent_slug: "aider",
            path: p,
        };
        let ns = conn.parse(&r).unwrap();
        assert_eq!(ns.agent_slug, AgentSlug::Aider);
        // Expected: user / assistant / user / assistant (4 turns)
        let roles: Vec<Role> = ns.turns.iter().map(|t| t.role).collect();
        assert!(roles.contains(&Role::User));
        assert!(roles.contains(&Role::Assistant));
        assert!(ns.turns.len() >= 4);
    }

    #[test]
    fn parse_extracts_diff_blocks_into_tool_calls() {
        let dir = tempdir().unwrap();
        let p = write_fixture(dir.path());
        let conn = AiderConnector::new(vec![dir.path().to_path_buf()]);
        let r = SessionRef {
            agent_slug: "aider",
            path: p,
        };
        let ns = conn.parse(&r).unwrap();
        let total_tool_calls: usize = ns.turns.iter().map(|t| t.tool_calls.len()).sum();
        assert_eq!(total_tool_calls, 2, "expected 2 diff blocks → 2 tool calls");

        let events = conn.extract_code_events(&ns).unwrap();
        let paths: Vec<&str> = events.iter().map(|e| e.file_path.as_str()).collect();
        assert!(paths.contains(&"src/auth/login.ts"));
        assert!(paths.contains(&"src/auth/login.test.ts"));
    }

    #[test]
    fn extract_diff_blocks_handles_no_diffs() {
        let body = "Just text, no diff fences.";
        assert!(extract_diff_blocks(body).is_empty());
    }

    #[test]
    fn extract_diff_blocks_handles_patch_alias() {
        let body = "```patch\n--- a/x\n+++ b/x\n+1\n```\n";
        let blocks = extract_diff_blocks(body);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("+++"));
    }

    #[test]
    fn path_from_diff_header_with_b_prefix() {
        assert_eq!(
            path_from_diff_header("--- a/x\n+++ b/src/foo.rs\n"),
            Some("src/foo.rs".to_string())
        );
    }

    #[test]
    fn synthesise_after_drops_minus_lines() {
        let diff = "--- a/x\n+++ b/x\n-old\n+new line one\n+new line two\n";
        let after = synthesise_after_from_hunks(diff);
        assert!(after.contains("new line one"));
        assert!(after.contains("new line two"));
        assert!(!after.contains("old"));
    }

    #[test]
    fn started_at_parses_header() {
        let text = "# aider chat started at 2026-04-25 10:30:00\n\n> hi\n";
        let dt = parse_started_at(text).unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-04-25 10:30:00"
        );
    }

    #[test]
    fn started_at_falls_back_to_now_on_missing() {
        let text = "no header here\n> user\nresp\n";
        // synth_session_id uses Utc::now when fallback fires; just make sure
        // parse_aider_markdown doesn't panic and returns a session.
        let ns = parse_aider_markdown(text, Path::new("/tmp/x")).unwrap();
        assert_eq!(ns.agent_slug, AgentSlug::Aider);
    }

    #[test]
    fn rejected_defaults_to_false() {
        let dir = tempdir().unwrap();
        let p = write_fixture(dir.path());
        let conn = AiderConnector::new(vec![dir.path().to_path_buf()]);
        let r = SessionRef {
            agent_slug: "aider",
            path: p,
        };
        let ns = conn.parse(&r).unwrap();
        let events = conn.extract_code_events(&ns).unwrap();
        assert!(events.iter().all(|e| !e.rejected));
    }
}
