//! Codex connector.
//!
//! Reads `~/.codex/sessions/YYYY/MM/DD/rollout-<ts>-<uuid>.jsonl`.
//! Each line is `{"timestamp","type","payload":{...}}`, threaded by
//! `payload.turn_id`. See PROPOSAL §C.2.

use super::{SessionConnector, SessionRef};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use drift_core::attribution::CodeEventDraft;
use drift_core::model::{
    AgentSlug, NormalizedSession, Operation, Role, ToolCall, ToolResult, Turn,
};
use drift_core::shell_lexer::{detect_intents, ShellIntent};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct CodexConnector {
    pub sessions_dir: PathBuf,
}

impl CodexConnector {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }
    pub fn with_default_root() -> Self {
        let root = dirs::home_dir()
            .map(|h| h.join(".codex").join("sessions"))
            .unwrap_or_else(|| PathBuf::from("/tmp/codex/sessions"));
        Self::new(root)
    }
}

impl SessionConnector for CodexConnector {
    fn agent_slug(&self) -> &'static str {
        "codex"
    }

    fn discover(&self) -> Result<Vec<SessionRef>> {
        let mut out = Vec::new();
        if !self.sessions_dir.exists() {
            return Ok(out);
        }
        walk(&self.sessions_dir, &mut out, self.agent_slug());
        Ok(out)
    }

    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession> {
        parse_file(&r.path)
    }

    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>> {
        extract_events(ns)
    }
}

fn walk(dir: &Path, out: &mut Vec<SessionRef>, slug: &'static str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out, slug);
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            out.push(SessionRef {
                agent_slug: slug,
                path: p,
            });
        }
    }
}

fn parse_file(path: &Path) -> Result<NormalizedSession> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut turns: Vec<Turn> = Vec::new();
    let mut session_id: Option<String> = None;
    let mut started_at: Option<DateTime<Utc>> = None;
    let mut ended_at: Option<DateTime<Utc>> = None;
    let mut model: Option<String> = None;
    let mut thinking_blocks: u32 = 0;

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let kind = v.get("type").and_then(|s| s.as_str()).unwrap_or("");
        let payload = v.get("payload").unwrap_or(&Value::Null);
        let ts = v
            .get("timestamp")
            .and_then(|s| s.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
            .or_else(|| {
                payload
                    .get("timestamp")
                    .and_then(|s| s.as_str())
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc))
            })
            .unwrap_or_else(Utc::now);

        started_at = Some(started_at.map_or(ts, |s| s.min(ts)));
        ended_at = Some(ended_at.map_or(ts, |e| e.max(ts)));

        match kind {
            "session_meta" => {
                if let Some(sid) = payload.get("id").and_then(|s| s.as_str()) {
                    session_id = Some(sid.to_string());
                }
                if let Some(m) = payload.get("model").and_then(|s| s.as_str()) {
                    model = Some(m.to_string());
                }
            }
            "response_item" => {
                let inner_type = payload.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let turn_id = payload
                    .get("turn_id")
                    .and_then(|s| s.as_str())
                    .or_else(|| payload.get("id").and_then(|s| s.as_str()))
                    .unwrap_or("")
                    .to_string();
                match inner_type {
                    "message" => {
                        let role_s = payload
                            .get("role")
                            .and_then(|s| s.as_str())
                            .unwrap_or("assistant");
                        let role = if role_s == "user" {
                            Role::User
                        } else {
                            Role::Assistant
                        };
                        let text_content = stringify_message_content(payload.get("content"));
                        turns.push(Turn {
                            turn_id,
                            role,
                            content_text: text_content,
                            tool_calls: vec![],
                            tool_results: vec![],
                            timestamp: ts,
                        });
                    }
                    "reasoning" => {
                        thinking_blocks += 1;
                    }
                    "custom_tool_call" | "function_call" => {
                        let name = payload
                            .get("name")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let id = payload
                            .get("call_id")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let input = if inner_type == "function_call" {
                            payload
                                .get("arguments")
                                .and_then(|a| a.as_str())
                                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                                .unwrap_or_else(|| {
                                    payload.get("arguments").cloned().unwrap_or(Value::Null)
                                })
                        } else {
                            payload
                                .get("input")
                                .cloned()
                                .unwrap_or_else(|| Value::String("".into()))
                        };
                        turns.push(Turn {
                            turn_id,
                            role: Role::Assistant,
                            content_text: String::new(),
                            tool_calls: vec![ToolCall { id, name, input }],
                            tool_results: vec![],
                            timestamp: ts,
                        });
                    }
                    "function_call_output" | "custom_tool_call_output" => {
                        let call_id = payload
                            .get("call_id")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let content = payload
                            .get("output")
                            .map(|o| o.to_string())
                            .unwrap_or_default();
                        // Codex encodes failure as status field; when missing we assume success.
                        let is_error = matches!(
                            payload.get("status").and_then(|s| s.as_str()),
                            Some("failed") | Some("error")
                        ) || content.contains("ERROR")
                            || content.contains("sandbox denied");
                        turns.push(Turn {
                            turn_id,
                            role: Role::ToolResult,
                            content_text: String::new(),
                            tool_calls: vec![],
                            tool_results: vec![ToolResult {
                                tool_use_id: call_id,
                                is_error,
                                content,
                            }],
                            timestamp: ts,
                        });
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let sid = session_id
        .or_else(|| path.file_stem().and_then(|s| s.to_str()).map(String::from))
        .ok_or_else(|| anyhow!("no session id in {}", path.display()))?;

    let now = Utc::now();
    Ok(NormalizedSession {
        session_id: sid,
        agent_slug: AgentSlug::Codex,
        model,
        working_dir: None,
        git_head_at_start: None,
        started_at: started_at.unwrap_or(now),
        ended_at: ended_at.unwrap_or(now),
        turns,
        thinking_blocks,
    })
}

fn stringify_message_content(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str().map(String::from)))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn extract_events(ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>> {
    let mut drafts = Vec::new();

    let mut errors: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    for t in &ns.turns {
        for tr in &t.tool_results {
            errors.insert(tr.tool_use_id.clone(), tr.is_error);
        }
    }

    let mut files: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for t in &ns.turns {
        if !matches!(t.role, Role::Assistant) {
            continue;
        }
        for tc in &t.tool_calls {
            let rejected = *errors.get(&tc.id).unwrap_or(&false);
            match tc.name.as_str() {
                "apply_patch" => {
                    let patch_text = match &tc.input {
                        Value::String(s) => s.clone(),
                        Value::Object(o) => o
                            .get("input")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_default(),
                        _ => continue,
                    };
                    for hunk in parse_apply_patch(&patch_text) {
                        let (before, after) = match &hunk.kind {
                            ApplyPatchKind::Add => ("".to_string(), hunk.body.clone()),
                            ApplyPatchKind::Delete => (
                                files.get(&hunk.path).cloned().unwrap_or_default(),
                                String::new(),
                            ),
                            ApplyPatchKind::Update => {
                                let before = files.get(&hunk.path).cloned().unwrap_or_default();
                                let after = apply_update_hunk(&before, &hunk.body);
                                (before, after)
                            }
                            ApplyPatchKind::Move { .. } => (String::new(), String::new()),
                        };
                        let operation = match &hunk.kind {
                            ApplyPatchKind::Add => Operation::Create,
                            ApplyPatchKind::Delete => Operation::Delete,
                            ApplyPatchKind::Update => Operation::Edit,
                            ApplyPatchKind::Move { .. } => Operation::Rename,
                        };
                        let rename_from = match &hunk.kind {
                            ApplyPatchKind::Move { from } => Some(from.clone()),
                            _ => None,
                        };
                        if !rejected && !matches!(hunk.kind, ApplyPatchKind::Delete) {
                            files.insert(hunk.path.clone(), after.clone());
                        } else if matches!(hunk.kind, ApplyPatchKind::Delete) && !rejected {
                            files.remove(&hunk.path);
                        }
                        let mut md = serde_json::Map::new();
                        md.insert("source".into(), Value::String("apply_patch".into()));
                        drafts.push(CodeEventDraft {
                            session_id: Some(ns.session_id.clone()),
                            agent_slug: AgentSlug::Codex,
                            turn_id: Some(t.turn_id.clone()),
                            timestamp: t.timestamp,
                            file_path: hunk.path.clone(),
                            operation,
                            rename_from,
                            before_content: before,
                            after_content: after,
                            rejected,
                            metadata: md,
                            event_id: None,
                            intra_call_parent: None,
                        });
                    }
                }
                "exec_command" => {
                    let cmd = tc
                        .input
                        .get("cmd")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    for intent in detect_intents(&cmd) {
                        if let Some(draft) = shell_intent_to_draft(ns, t, rejected, intent) {
                            drafts.push(draft);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(drafts)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ApplyPatchKind {
    Add,
    Update,
    Delete,
    Move { from: String },
}

#[derive(Debug, Clone)]
struct ApplyPatchHunk {
    kind: ApplyPatchKind,
    path: String,
    body: String,
}

/// Parse the Codex `apply_patch` envelope into per-file hunks.
fn parse_apply_patch(text: &str) -> Vec<ApplyPatchHunk> {
    let mut out = Vec::new();
    let mut current: Option<ApplyPatchHunk> = None;
    let mut body = String::new();
    for line in text.lines() {
        if line == "*** Begin Patch" || line == "*** End Patch" {
            if let Some(mut h) = current.take() {
                h.body = std::mem::take(&mut body);
                out.push(h);
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("*** Add File: ") {
            if let Some(mut h) = current.take() {
                h.body = std::mem::take(&mut body);
                out.push(h);
            }
            current = Some(ApplyPatchHunk {
                kind: ApplyPatchKind::Add,
                path: rest.trim().to_string(),
                body: String::new(),
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("*** Update File: ") {
            if let Some(mut h) = current.take() {
                h.body = std::mem::take(&mut body);
                out.push(h);
            }
            current = Some(ApplyPatchHunk {
                kind: ApplyPatchKind::Update,
                path: rest.trim().to_string(),
                body: String::new(),
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("*** Delete File: ") {
            if let Some(mut h) = current.take() {
                h.body = std::mem::take(&mut body);
                out.push(h);
            }
            current = Some(ApplyPatchHunk {
                kind: ApplyPatchKind::Delete,
                path: rest.trim().to_string(),
                body: String::new(),
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("*** Move File: ") {
            // "from -> to" or "from → to"
            let rest = rest.trim();
            let (from, to) = if let Some((l, r)) = rest.split_once("→") {
                (l.trim().to_string(), r.trim().to_string())
            } else if let Some((l, r)) = rest.split_once("->") {
                (l.trim().to_string(), r.trim().to_string())
            } else {
                (rest.to_string(), rest.to_string())
            };
            if let Some(mut h) = current.take() {
                h.body = std::mem::take(&mut body);
                out.push(h);
            }
            current = Some(ApplyPatchHunk {
                kind: ApplyPatchKind::Move { from },
                path: to,
                body: String::new(),
            });
            continue;
        }

        if current.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(mut h) = current.take() {
        h.body = std::mem::take(&mut body);
        out.push(h);
    }
    // Normalise Add-file body (strip leading '+').
    for h in &mut out {
        if matches!(h.kind, ApplyPatchKind::Add) {
            h.body = h
                .body
                .lines()
                .map(|l| l.strip_prefix('+').unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n")
                + if h.body.ends_with('\n') { "\n" } else { "" };
        }
    }
    out
}

fn apply_update_hunk(before: &str, hunk_body: &str) -> String {
    // Minimal Codex update interpreter: strip a leading '+' to produce after
    // lines, strip leading '-' to identify removed lines, '@@' is context
    // marker we ignore. For v0.1.0 we apply a best-effort line-wise patch:
    // concatenate non-'-' lines with leading char stripped.
    let mut out = Vec::new();
    for line in hunk_body.lines() {
        if line.starts_with("@@") {
            continue;
        }
        if let Some(rest) = line.strip_prefix('-') {
            let _ = rest;
            continue; // removed
        }
        if let Some(rest) = line.strip_prefix('+') {
            out.push(rest.to_string());
            continue;
        }
        // context: passthrough
        out.push(line.to_string());
    }
    // Fallback: if we don't look like we produced anything, return before.
    if out.is_empty() {
        return before.to_string();
    }
    let mut res = out.join("\n");
    if !res.ends_with('\n') {
        res.push('\n');
    }
    res
}

fn shell_intent_to_draft(
    ns: &NormalizedSession,
    t: &Turn,
    rejected: bool,
    intent: ShellIntent,
) -> Option<CodeEventDraft> {
    let mut md = serde_json::Map::new();
    md.insert("detected_via".into(), Value::String("shell-lexer".into()));
    match intent {
        ShellIntent::Move { from, to } => Some(CodeEventDraft {
            session_id: Some(ns.session_id.clone()),
            agent_slug: AgentSlug::Codex,
            turn_id: Some(t.turn_id.clone()),
            timestamp: t.timestamp,
            file_path: to,
            operation: Operation::Rename,
            rename_from: Some(from),
            before_content: String::new(),
            after_content: String::new(),
            rejected,
            metadata: md,
            event_id: None,
            intra_call_parent: None,
        }),
        ShellIntent::Remove { path } => Some(CodeEventDraft {
            session_id: Some(ns.session_id.clone()),
            agent_slug: AgentSlug::Codex,
            turn_id: Some(t.turn_id.clone()),
            timestamp: t.timestamp,
            file_path: path,
            operation: Operation::Delete,
            rename_from: None,
            before_content: String::new(),
            after_content: String::new(),
            rejected,
            metadata: md,
            event_id: None,
            intra_call_parent: None,
        }),
        ShellIntent::Copy { to, .. }
        | ShellIntent::RedirectWrite { path: to, .. }
        | ShellIntent::SedInPlace { path: to }
        | ShellIntent::PythonWriteBestEffort { path: to } => Some(CodeEventDraft {
            session_id: Some(ns.session_id.clone()),
            agent_slug: AgentSlug::Codex,
            turn_id: Some(t.turn_id.clone()),
            timestamp: t.timestamp,
            file_path: to,
            operation: Operation::Edit,
            rename_from: None,
            before_content: String::new(),
            after_content: String::new(),
            rejected,
            metadata: md,
            event_id: None,
            intra_call_parent: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_apply_patch_add() {
        let patch = "*** Begin Patch\n*** Add File: hi.txt\n+hello\n+drift\n*** End Patch\n";
        let hs = parse_apply_patch(patch);
        assert_eq!(hs.len(), 1);
        assert!(matches!(hs[0].kind, ApplyPatchKind::Add));
        assert_eq!(hs[0].path, "hi.txt");
        assert!(hs[0].body.contains("hello"));
    }

    #[test]
    fn parse_apply_patch_move() {
        let patch = "*** Begin Patch\n*** Move File: old.txt -> new.txt\n*** End Patch\n";
        let hs = parse_apply_patch(patch);
        assert_eq!(hs.len(), 1);
        match &hs[0].kind {
            ApplyPatchKind::Move { from } => assert_eq!(from, "old.txt"),
            _ => panic!("expected Move"),
        }
        assert_eq!(hs[0].path, "new.txt");
    }

    #[test]
    fn parse_apply_patch_delete() {
        let patch = "*** Begin Patch\n*** Delete File: gone.txt\n*** End Patch\n";
        let hs = parse_apply_patch(patch);
        assert_eq!(hs.len(), 1);
        assert!(matches!(hs[0].kind, ApplyPatchKind::Delete));
    }
}
