//! drift-mcp — a minimal stdio MCP server exposing Drift AI's read-only
//! query surface.
//!
//! MCP wire protocol: newline-delimited JSON-RPC 2.0 over stdin/stdout.
//! We implement just enough of the Model Context Protocol (v2024-11-05 /
//! 2025-06-18 handshake) to be discoverable by Claude Code, Codex, and any
//! MCP-conformant client.
//!
//! Tools exposed (all read-only):
//!   - drift_blame(file: string, line?: int, range?: string)
//!   - drift_trace(session_id: string)
//!   - drift_rejected(since?: string)
//!   - drift_log(commit_range?: string)
//!   - drift_show_event(event_id: string)
//!
//! Writes (capture / bind / sync) are intentionally excluded — anything that
//! mutates state should go through the CLI.

use anyhow::Result;
use drift_core::git;
use drift_core::EventStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "drift";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

/// Run the stdio MCP server bound to `repo`. Blocks until stdin closes.
pub fn run_stdio(repo: &Path) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let reader = stdin.lock();
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                write_line(
                    &mut out,
                    &JsonRpcResponse {
                        jsonrpc: "2.0",
                        id: Value::Null,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("parse error: {}", e),
                        }),
                    },
                )?;
                continue;
            }
        };

        if req.id.is_none() {
            // notification — no response expected.
            continue;
        }
        let id = req.id.clone().unwrap_or(Value::Null);

        let response = match req.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
                })),
                error: None,
            },
            "tools/list" => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({ "tools": tool_defs() })),
                error: None,
            },
            "tools/call" => match handle_tool_call(repo, &req.params) {
                Ok(v) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(v),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: e.to_string(),
                    }),
                },
            },
            "ping" => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({})),
                error: None,
            },
            _ => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("method not found: {}", req.method),
                }),
            },
        };
        write_line(&mut out, &response)?;
    }
    Ok(())
}

fn write_line<W: Write>(w: &mut W, r: &JsonRpcResponse) -> std::io::Result<()> {
    let s = serde_json::to_string(r).expect("serialise response");
    writeln!(w, "{}", s)?;
    w.flush()?;
    Ok(())
}

pub fn tool_defs() -> Vec<Value> {
    vec![
        json!({
            "name": "drift_blame",
            "description": "Reverse lookup: return the timeline of CodeEvents that touched a file (optionally filtered to a specific line or range). Each entry includes agent_slug, session_id, timestamp, diff_hunks, and rejected flag.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file":  { "type": "string", "description": "Repo-relative path" },
                    "line":  { "type": "integer", "minimum": 1 },
                    "range": { "type": "string", "description": "e.g. `42-50`" }
                },
                "required": ["file"]
            }
        }),
        json!({
            "name": "drift_trace",
            "description": "Forward lookup: return every CodeEvent a session produced.",
            "inputSchema": {
                "type": "object",
                "properties": { "session_id": { "type": "string" } },
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "drift_rejected",
            "description": "List rejected AI suggestions, optionally since an RFC3339 timestamp.",
            "inputSchema": {
                "type": "object",
                "properties": { "since": { "type": "string" } }
            }
        }),
        json!({
            "name": "drift_log",
            "description": "Return commits with per-agent session summaries merged in.",
            "inputSchema": {
                "type": "object",
                "properties": { "commit_range": { "type": "string" } }
            }
        }),
        json!({
            "name": "drift_show_event",
            "description": "Return a single CodeEvent by id (diff + metadata).",
            "inputSchema": {
                "type": "object",
                "properties": { "event_id": { "type": "string" } },
                "required": ["event_id"]
            }
        }),
    ]
}

fn handle_tool_call(repo: &Path, params: &Value) -> Result<Value> {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let store = EventStore::open(store_path(repo))?;

    let text = match name {
        "drift_blame" => {
            let file = args.get("file").and_then(|v| v.as_str()).unwrap_or("");
            let events = store.events_for_file(file)?;
            serde_json::to_string_pretty(&events)?
        }
        "drift_trace" => {
            let sid = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let events = store.events_for_session(sid)?;
            serde_json::to_string_pretty(&events)?
        }
        "drift_rejected" => {
            let since = args.get("since").and_then(|v| v.as_str());
            let events = store.rejected_events(since)?;
            serde_json::to_string_pretty(&events)?
        }
        "drift_log" => {
            let range = args
                .get("commit_range")
                .and_then(|v| v.as_str())
                .map(String::from);
            let commits = git::list_commits(repo, range.as_deref())?;
            let mut enriched = Vec::new();
            for c in commits {
                let events = store.events_for_commit(&c.sha).unwrap_or_default();
                enriched.push(json!({
                    "commit_sha": c.sha,
                    "subject": c.subject,
                    "committed_at_unix": c.committed_at_unix,
                    "event_count": events.len(),
                    "events": events,
                }));
            }
            serde_json::to_string_pretty(&enriched)?
        }
        "drift_show_event" => {
            let id = args.get("event_id").and_then(|v| v.as_str()).unwrap_or("");
            match store.event_by_id(id)? {
                Some(ev) => serde_json::to_string_pretty(&ev)?,
                None => format!("event {} not found", id),
            }
        }
        _ => anyhow::bail!("unknown tool `{}`", name),
    };

    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
}

fn store_path(repo: &Path) -> PathBuf {
    repo.join(".prompts").join("events.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_defs_have_required_fields() {
        let defs = tool_defs();
        let names: Vec<&str> = defs
            .iter()
            .filter_map(|d| d.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"drift_blame"));
        assert!(names.contains(&"drift_trace"));
        assert!(names.contains(&"drift_rejected"));
        assert!(names.contains(&"drift_log"));
        assert!(names.contains(&"drift_show_event"));
        for d in defs {
            assert!(d.get("description").is_some());
            assert!(d.get("inputSchema").is_some());
        }
    }
}
