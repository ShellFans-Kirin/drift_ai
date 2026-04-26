//! Cursor session connector — reads composer / chat history from Cursor's
//! per-workspace SQLite store (`state.vscdb`).
//!
//! [BEST-EFFORT] Cursor's on-disk schema is undocumented. This connector is
//! reverse-engineered from current Cursor stable (early 2026) and may break
//! when Cursor changes its storage format. We tag emitted events as
//! `[BEST-EFFORT]` and emit warnings rather than failing `drift capture` on
//! parse error so a Cursor schema change doesn't take down the whole pipeline.
//!
//! Storage paths:
//! - macOS:   `~/Library/Application Support/Cursor/User/workspaceStorage/<hash>/state.vscdb`
//! - Linux:   `~/.config/Cursor/User/workspaceStorage/<hash>/state.vscdb`
//! - Windows: `%APPDATA%\Cursor\User\workspaceStorage\<hash>\state.vscdb`
//!
//! Each `<hash>` is one workspace. `state.vscdb` contains a single relevant
//! table:
//!
//! ```sql
//! CREATE TABLE cursorDiskKV (
//!   key   TEXT PRIMARY KEY,
//!   value BLOB
//! );
//! ```
//!
//! Keys we read:
//! - `composerData:<id>` — composer (chat) sessions: messages + edits.
//! - `cursorPanelView:<id>` — sidebar chat sessions (treated like composer).
//!
//! Each `value` is a JSON object with the following shape (typed from
//! observed fixtures; missing fields are tolerated):
//!
//! ```json
//! {
//!   "composerId": "<uuid>",
//!   "messages": [
//!     { "role": "user", "content": "...", "timestamp": 1712345678 },
//!     { "role": "assistant", "content": "...", "edits": [...] }
//!   ],
//!   "edits": [
//!     { "filePath": "src/foo.rs", "before": "...", "after": "...",
//!       "status": "accepted" | "rejected", "diff": "..." }
//!   ],
//!   "createdAt": 1712345678,
//!   "updatedAt": 1712345999
//! }
//! ```
//!
//! Output:
//! - `agent_slug = "cursor"` (a first-class slug introduced in v0.4)
//! - One [`NormalizedSession`] per `composerData:<id>` key
//! - One [`CodeEventDraft`] per `edits[].filePath` with operation inferred from
//!   before/after presence

use super::{SessionConnector, SessionRef};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use drift_core::attribution::CodeEventDraft;
use drift_core::model::{
    AgentSlug, NormalizedSession, Operation, Role, ToolCall, ToolResult, Turn,
};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub struct CursorConnector {
    pub workspace_storage_root: PathBuf,
}

impl CursorConnector {
    pub fn new(workspace_storage_root: PathBuf) -> Self {
        Self {
            workspace_storage_root,
        }
    }

    /// Resolve the platform-default `workspaceStorage` directory.
    pub fn with_default_root() -> Self {
        let root = default_workspace_storage_root();
        Self::new(root)
    }
}

/// Returns the platform-default `Cursor/User/workspaceStorage/` directory.
/// Falls back to `/tmp/cursor/workspaceStorage` if the system home directory
/// can't be resolved (e.g. very minimal CI containers).
pub fn default_workspace_storage_root() -> PathBuf {
    // Prefer a per-platform-natural path. dirs::config_dir() returns:
    // - macOS:   ~/Library/Application Support
    // - Linux:   ~/.config
    // - Windows: %APPDATA%
    if let Some(cfg) = dirs::config_dir() {
        return cfg.join("Cursor").join("User").join("workspaceStorage");
    }
    PathBuf::from("/tmp/cursor/workspaceStorage")
}

impl SessionConnector for CursorConnector {
    fn agent_slug(&self) -> &'static str {
        "cursor"
    }

    fn discover(&self) -> Result<Vec<SessionRef>> {
        let mut out = Vec::new();
        if !self.workspace_storage_root.exists() {
            return Ok(out);
        }
        let entries = match std::fs::read_dir(&self.workspace_storage_root) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    "cursor: cannot read {}: {}",
                    self.workspace_storage_root.display(),
                    e
                );
                return Ok(out);
            }
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let db = p.join("state.vscdb");
            if db.exists() {
                out.push(SessionRef {
                    agent_slug: self.agent_slug(),
                    path: db,
                });
            }
        }
        Ok(out)
    }

    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession> {
        // A `state.vscdb` typically holds *multiple* composer sessions; the
        // SessionConnector trait expects one [`NormalizedSession`] per
        // `SessionRef`. We pick the most-recently-updated composer in the DB
        // so the output of `drift capture` reflects the user's latest work.
        // Capturing every composer separately requires a connector-level
        // generalisation we defer to v0.5.
        parse_latest_composer(&r.path)
    }

    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>> {
        Ok(extract_events(ns))
    }
}

// ---------------------------------------------------------------------------
// SQLite read
// ---------------------------------------------------------------------------

fn open_readonly(path: &Path) -> Result<Connection> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("open Cursor SQLite (read-only) at {}", path.display()))
}

/// Read all `composerData:*` rows from a Cursor `state.vscdb` and return
/// `(key, parsed_value)` pairs. Rows that fail to parse as JSON are skipped
/// with a warning so a single corrupted blob doesn't take down capture.
pub fn read_composers(path: &Path) -> Result<Vec<(String, Value)>> {
    let conn = open_readonly(path)?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM cursorDiskKV WHERE key LIKE 'composerData:%' OR key LIKE 'cursorPanelView:%'")
        .context("prepare cursor composer query")?;
    let rows = stmt.query_map([], |row| {
        let k: String = row.get(0)?;
        let v: Vec<u8> = row.get(1)?;
        Ok((k, v))
    })?;

    let mut out = Vec::new();
    for r in rows {
        let (k, raw) = match r {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("cursor: row read failed: {}", e);
                continue;
            }
        };
        // Cursor stores values as JSON-encoded strings (sometimes as raw JSON
        // bytes, sometimes JSON-escaped strings). Try the natural shape first.
        let parsed = match serde_json::from_slice::<Value>(&raw) {
            Ok(v) => v,
            Err(_) => {
                // Some Cursor versions wrap in a JSON string; unwrap one level.
                if let Ok(s) = std::str::from_utf8(&raw) {
                    match serde_json::from_str::<Value>(s) {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!("cursor: skip {}: bad JSON: {}", k, e);
                            continue;
                        }
                    }
                } else {
                    tracing::warn!("cursor: skip {}: non-utf8 value", k);
                    continue;
                }
            }
        };
        out.push((k, parsed));
    }
    Ok(out)
}

/// Read all composers and pick the one with the largest `updatedAt`.
fn parse_latest_composer(path: &Path) -> Result<NormalizedSession> {
    let composers = read_composers(path)?;
    if composers.is_empty() {
        return Err(anyhow!(
            "cursor: no composer data found in {}",
            path.display()
        ));
    }
    let (key, value) = composers
        .into_iter()
        .max_by_key(|(_, v)| {
            v.get("updatedAt")
                .and_then(|x| x.as_i64())
                .or_else(|| v.get("createdAt").and_then(|x| x.as_i64()))
                .unwrap_or(0)
        })
        .unwrap();
    composer_to_session(&key, &value, path)
}

// ---------------------------------------------------------------------------
// Cursor JSON → NormalizedSession
// ---------------------------------------------------------------------------

/// Strip the `composerData:` / `cursorPanelView:` prefix and return the bare ID.
fn session_id_from_key(key: &str) -> String {
    if let Some(s) = key.strip_prefix("composerData:") {
        return s.to_string();
    }
    if let Some(s) = key.strip_prefix("cursorPanelView:") {
        return s.to_string();
    }
    key.to_string()
}

fn cursor_role(role: &str) -> Role {
    match role.to_lowercase().as_str() {
        "user" | "human" => Role::User,
        "assistant" | "agent" | "model" | "system" => Role::Assistant,
        _ => Role::Assistant,
    }
}

fn unix_seconds_to_dt(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).single().unwrap_or_else(Utc::now)
}

/// Turn a single composer JSON blob into a [`NormalizedSession`].
pub fn composer_to_session(key: &str, v: &Value, source_path: &Path) -> Result<NormalizedSession> {
    let session_id = v
        .get("composerId")
        .and_then(|x| x.as_str())
        .map(String::from)
        .unwrap_or_else(|| session_id_from_key(key));

    let created_at = v
        .get("createdAt")
        .and_then(|x| x.as_i64())
        .map(unix_seconds_to_dt)
        .unwrap_or_else(Utc::now);
    let updated_at = v
        .get("updatedAt")
        .and_then(|x| x.as_i64())
        .map(unix_seconds_to_dt)
        .unwrap_or(created_at);

    let messages = v
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    let edits = v
        .get("edits")
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default();

    // Build turns. Each `messages[i]` becomes a Turn. We attach edits to
    // assistant turns: the simplest heuristic is "all edits attach to the
    // most recent assistant turn" — Cursor's JSON often doesn't carry an
    // explicit message-id / edit-id linkage, so this is best-effort. If a
    // future Cursor schema exposes a tighter binding we'll switch then.
    let mut turns: Vec<Turn> = Vec::with_capacity(messages.len());
    let mut last_assistant_turn_idx: Option<usize> = None;
    for (i, m) in messages.iter().enumerate() {
        let role_str = m
            .get("role")
            .and_then(|x| x.as_str())
            .unwrap_or("user")
            .to_string();
        let role = cursor_role(&role_str);
        let content_text = m
            .get("content")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let timestamp = m
            .get("timestamp")
            .and_then(|x| x.as_i64())
            .map(unix_seconds_to_dt)
            .unwrap_or(created_at);
        let turn_id = format!("cursor-{}-{}", session_id_short(&session_id), i);
        if matches!(role, Role::Assistant) {
            last_assistant_turn_idx = Some(turns.len());
        }
        turns.push(Turn {
            turn_id,
            role,
            content_text,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            timestamp,
        });
    }

    // Attach edits as ToolCall / ToolResult on the most recent assistant turn.
    // If no assistant turn exists yet, attach to the last turn we have so the
    // file-path information isn't lost.
    let attach_idx = last_assistant_turn_idx.or_else(|| {
        if turns.is_empty() {
            None
        } else {
            Some(turns.len() - 1)
        }
    });
    if let Some(idx) = attach_idx {
        for (j, e) in edits.iter().enumerate() {
            let path = e
                .get("filePath")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if path.is_empty() {
                continue;
            }
            let tc_id = format!("cursor-edit-{}-{}", session_id_short(&session_id), j);
            // Map status → tool_result `is_error` so v0.1 attribution code
            // marks rejected suggestions consistently with Claude Code / Codex.
            let status = e
                .get("status")
                .and_then(|x| x.as_str())
                .unwrap_or("accepted");
            let is_error =
                status.eq_ignore_ascii_case("rejected") || status.eq_ignore_ascii_case("declined");

            let mut input = serde_json::Map::new();
            input.insert("file_path".into(), Value::String(path.clone()));
            if let Some(diff) = e.get("diff").and_then(|x| x.as_str()) {
                input.insert("diff".into(), Value::String(diff.to_string()));
            }
            if let Some(after) = e.get("after").and_then(|x| x.as_str()) {
                input.insert("content".into(), Value::String(after.to_string()));
            }
            turns[idx].tool_calls.push(ToolCall {
                id: tc_id.clone(),
                name: cursor_op_name(e).into(),
                input: Value::Object(input),
            });
            turns[idx].tool_results.push(ToolResult {
                tool_use_id: tc_id,
                content: status.to_string(),
                is_error,
            });
        }
    }

    Ok(NormalizedSession {
        session_id,
        agent_slug: AgentSlug::Cursor,
        model: v.get("model").and_then(|x| x.as_str()).map(String::from),
        working_dir: source_path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf()),
        git_head_at_start: None,
        started_at: created_at,
        ended_at: updated_at,
        turns,
        thinking_blocks: 0,
    })
}

/// Map a Cursor edit blob to a tool-call name approximating Claude Code /
/// Codex conventions so downstream attribution is uniform.
fn cursor_op_name(e: &Value) -> &'static str {
    let has_before = e
        .get("before")
        .map(|x| !x.is_null() && x.as_str().is_some_and(|s| !s.is_empty()))
        .unwrap_or(false);
    let has_after = e
        .get("after")
        .map(|x| !x.is_null() && x.as_str().is_some_and(|s| !s.is_empty()))
        .unwrap_or(false);
    match (has_before, has_after) {
        (false, true) => "Write",
        (true, false) => "Delete",
        _ => "Edit",
    }
}

fn session_id_short(s: &str) -> String {
    s.chars().take(8).collect()
}

// ---------------------------------------------------------------------------
// CodeEvent extraction
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let bytes = h.finalize();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes.iter() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

fn extract_events(ns: &NormalizedSession) -> Vec<CodeEventDraft> {
    let mut out = Vec::new();
    for t in &ns.turns {
        for (tc_idx, tc) in t.tool_calls.iter().enumerate() {
            let path = match tc.input.get("file_path").and_then(|v| v.as_str()) {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => continue,
            };
            let op = match tc.name.as_str() {
                "Write" => Operation::Create,
                "Delete" => Operation::Delete,
                _ => Operation::Edit,
            };
            let after_content = tc
                .input
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rejected = t
                .tool_results
                .get(tc_idx)
                .map(|r| r.is_error)
                .unwrap_or(false);
            let mut metadata = serde_json::Map::new();
            metadata.insert(
                "best_effort".into(),
                serde_json::Value::String("cursor schema reverse-engineered".into()),
            );
            metadata.insert(
                "tool_call_id".into(),
                serde_json::Value::String(tc.id.clone()),
            );
            if let Some(diff) = tc.input.get("diff").and_then(|v| v.as_str()) {
                metadata.insert(
                    "diff_text".into(),
                    serde_json::Value::String(diff.to_string()),
                );
            }
            out.push(CodeEventDraft {
                session_id: Some(ns.session_id.clone()),
                agent_slug: ns.agent_slug,
                turn_id: Some(t.turn_id.clone()),
                timestamp: t.timestamp,
                file_path: path,
                operation: op,
                rename_from: None,
                before_content: String::new(),
                after_content,
                rejected,
                metadata,
                event_id: Some(format!("{}-evt-{}", t.turn_id, tc_idx)),
                intra_call_parent: None,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests — fixture-driven, never reads real Cursor user data
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use serde_json::json;
    use tempfile::tempdir;

    /// Build a fixture `state.vscdb` at `path` containing the supplied
    /// (key, value) pairs as `cursorDiskKV` rows. JSON values are stored
    /// as bytes (mirroring how Cursor stores them).
    fn build_fixture_db(path: &Path, rows: &[(&str, &Value)]) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch("CREATE TABLE cursorDiskKV (key TEXT PRIMARY KEY, value BLOB);")
            .unwrap();
        for (k, v) in rows {
            let bytes = serde_json::to_vec(v).unwrap();
            conn.execute(
                "INSERT INTO cursorDiskKV (key, value) VALUES (?, ?);",
                rusqlite::params![*k, bytes],
            )
            .unwrap();
        }
    }

    fn one_composer_fixture() -> Value {
        json!({
            "composerId": "abc12345",
            "createdAt": 1_712_345_000,
            "updatedAt": 1_712_345_999,
            "messages": [
                { "role": "user", "content": "Add a rate limiter", "timestamp": 1_712_345_100 },
                { "role": "assistant", "content": "Adding sliding window in src/auth/login.ts.", "timestamp": 1_712_345_200 }
            ],
            "edits": [
                {
                    "filePath": "src/auth/login.ts",
                    "before": "old content",
                    "after": "new content with rate limit",
                    "status": "accepted",
                    "diff": "@@ -1 +1 @@\n-old\n+new"
                },
                {
                    "filePath": "src/auth/login.test.ts",
                    "before": "",
                    "after": "tests added",
                    "status": "accepted"
                },
                {
                    "filePath": "src/legacy.ts",
                    "before": "deleted code",
                    "after": "",
                    "status": "rejected"
                }
            ]
        })
    }

    #[test]
    fn discover_finds_state_vscdb_in_each_workspace() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ws_a = root.join("hash-aaa");
        let ws_b = root.join("hash-bbb");
        let ws_c = root.join("hash-ccc"); // no DB
        std::fs::create_dir_all(&ws_a).unwrap();
        std::fs::create_dir_all(&ws_b).unwrap();
        std::fs::create_dir_all(&ws_c).unwrap();
        build_fixture_db(&ws_a.join("state.vscdb"), &[]);
        build_fixture_db(&ws_b.join("state.vscdb"), &[]);
        let conn = CursorConnector::new(root);
        let refs = conn.discover().unwrap();
        let mut paths: Vec<String> = refs
            .iter()
            .map(|r| r.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        paths.sort();
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().all(|r| r.agent_slug == "cursor"));
    }

    #[test]
    fn discover_handles_missing_root() {
        let conn = CursorConnector::new(PathBuf::from("/no/such/dir"));
        let refs = conn.discover().unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn parse_picks_latest_composer_by_updated_at() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.vscdb");
        let older = json!({
            "composerId": "old-1",
            "createdAt": 1_000,
            "updatedAt": 1_000,
            "messages": [{ "role": "user", "content": "old", "timestamp": 1_000 }],
            "edits": []
        });
        let newer = one_composer_fixture();
        build_fixture_db(
            &db,
            &[
                ("composerData:old-1", &older),
                ("composerData:new-1", &newer),
            ],
        );
        let conn = CursorConnector::new(dir.path().to_path_buf());
        let r = SessionRef {
            agent_slug: "cursor",
            path: db,
        };
        let ns = conn.parse(&r).unwrap();
        assert_eq!(ns.session_id, "abc12345");
        assert_eq!(ns.agent_slug, AgentSlug::Cursor);
        assert_eq!(ns.turns.len(), 2);
    }

    #[test]
    fn extract_events_emits_per_edit_codeevent() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.vscdb");
        let v = one_composer_fixture();
        build_fixture_db(&db, &[("composerData:abc12345", &v)]);
        let conn = CursorConnector::new(dir.path().to_path_buf());
        let r = SessionRef {
            agent_slug: "cursor",
            path: db,
        };
        let ns = conn.parse(&r).unwrap();
        let events = conn.extract_code_events(&ns).unwrap();
        assert_eq!(events.len(), 3);
        let by_path: Vec<_> = events.iter().map(|e| e.file_path.as_str()).collect();
        assert!(by_path.contains(&"src/auth/login.ts"));
        assert!(by_path.contains(&"src/auth/login.test.ts"));
        assert!(by_path.contains(&"src/legacy.ts"));
        // rejected status round-trips
        let rejected_evt = events
            .iter()
            .find(|e| e.file_path == "src/legacy.ts")
            .unwrap();
        assert!(rejected_evt.rejected);
        // status=accepted ones are not marked rejected
        let kept_evt = events
            .iter()
            .find(|e| e.file_path == "src/auth/login.ts")
            .unwrap();
        assert!(!kept_evt.rejected);
    }

    #[test]
    fn op_inference_from_before_after() {
        // before empty + after present → Create
        assert_eq!(
            cursor_op_name(&json!({ "before": "", "after": "x" })),
            "Write"
        );
        // before present + after empty → Delete
        assert_eq!(
            cursor_op_name(&json!({ "before": "x", "after": "" })),
            "Delete"
        );
        // both present → Edit
        assert_eq!(
            cursor_op_name(&json!({ "before": "x", "after": "y" })),
            "Edit"
        );
        // missing both → Edit (default)
        assert_eq!(cursor_op_name(&json!({})), "Edit");
    }

    #[test]
    fn empty_db_produces_helpful_error() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.vscdb");
        build_fixture_db(&db, &[]);
        let conn = CursorConnector::new(dir.path().to_path_buf());
        let r = SessionRef {
            agent_slug: "cursor",
            path: db,
        };
        let err = conn.parse(&r).unwrap_err();
        assert!(format!("{}", err).contains("no composer data"));
    }

    #[test]
    fn cursor_panel_view_keys_also_recognised() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.vscdb");
        let panel_value = json!({
            "composerId": "panel-1",
            "createdAt": 2_000,
            "updatedAt": 2_500,
            "messages": [
                { "role": "user", "content": "panel question", "timestamp": 2_100 }
            ],
            "edits": []
        });
        build_fixture_db(&db, &[("cursorPanelView:p1", &panel_value)]);
        let conn = CursorConnector::new(dir.path().to_path_buf());
        let r = SessionRef {
            agent_slug: "cursor",
            path: db,
        };
        let ns = conn.parse(&r).unwrap();
        assert_eq!(ns.turns.len(), 1);
        assert_eq!(ns.session_id, "panel-1");
    }

    #[test]
    fn corrupted_value_skipped_gracefully() {
        // A row whose value isn't valid JSON should be skipped, not fail
        // the whole parse.
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.vscdb");
        let conn_init = Connection::open(&db).unwrap();
        conn_init
            .execute_batch("CREATE TABLE cursorDiskKV (key TEXT PRIMARY KEY, value BLOB);")
            .unwrap();
        conn_init
            .execute(
                "INSERT INTO cursorDiskKV (key, value) VALUES (?, ?);",
                rusqlite::params!["composerData:bad", b"not json {{".as_ref()],
            )
            .unwrap();
        let good = one_composer_fixture();
        let good_bytes = serde_json::to_vec(&good).unwrap();
        conn_init
            .execute(
                "INSERT INTO cursorDiskKV (key, value) VALUES (?, ?);",
                rusqlite::params!["composerData:good", good_bytes],
            )
            .unwrap();
        drop(conn_init);

        let conn = CursorConnector::new(dir.path().to_path_buf());
        let r = SessionRef {
            agent_slug: "cursor",
            path: db,
        };
        let ns = conn.parse(&r).unwrap();
        assert_eq!(ns.session_id, "abc12345");
    }

    #[test]
    fn agent_slug_and_default_root_smoke() {
        let conn = CursorConnector::with_default_root();
        assert_eq!(conn.agent_slug(), "cursor");
        assert!(conn.workspace_storage_root.components().any(|c| c
            .as_os_str()
            .to_string_lossy()
            .contains("Cursor")
            || c.as_os_str().to_string_lossy().contains("/tmp")));
    }
}
