//! Claude Code connector.
//!
//! Reads `~/.claude/projects/<project>/<session_id>.jsonl`, each line a JSON
//! event. See PROPOSAL §C.1 for the schema reference.

use super::{SessionConnector, SessionRef};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use drift_core::attribution::CodeEventDraft;
use drift_core::model::{AgentSlug, NormalizedSession, Operation, Role, ToolCall, ToolResult, Turn};
use drift_core::shell_lexer::{detect_intents, ShellIntent};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct ClaudeCodeConnector {
    pub projects_dir: PathBuf,
}

impl ClaudeCodeConnector {
    pub fn new(projects_dir: PathBuf) -> Self {
        Self { projects_dir }
    }
    pub fn with_default_root() -> Self {
        let root = dirs::home_dir()
            .map(|h| h.join(".claude").join("projects"))
            .unwrap_or_else(|| PathBuf::from("/tmp/claude/projects"));
        Self::new(root)
    }
}

impl SessionConnector for ClaudeCodeConnector {
    fn agent_slug(&self) -> &'static str {
        "claude-code"
    }

    fn discover(&self) -> Result<Vec<SessionRef>> {
        let mut out = Vec::new();
        if !self.projects_dir.exists() {
            return Ok(out);
        }
        walk_jsonls(&self.projects_dir, &mut out, self.agent_slug());
        Ok(out)
    }

    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession> {
        parse_file(&r.path)
    }

    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>> {
        extract_events(ns)
    }
}

fn walk_jsonls(dir: &Path, out: &mut Vec<SessionRef>, slug: &'static str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_jsonls(&p, out, slug);
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
    let text =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut turns: Vec<Turn> = Vec::new();
    let mut session_id = String::new();
    let mut started_at: Option<DateTime<Utc>> = None;
    let mut ended_at: Option<DateTime<Utc>> = None;
    let mut model: Option<String> = None;
    let mut working_dir: Option<PathBuf> = None;
    let mut thinking_blocks: u32 = 0;

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let kind = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        if let Some(sid) = v.get("sessionId").and_then(|s| s.as_str()) {
            if session_id.is_empty() {
                session_id = sid.to_string();
            }
        }
        if let Some(cwd) = v.get("cwd").and_then(|s| s.as_str()) {
            if working_dir.is_none() {
                working_dir = Some(PathBuf::from(cwd));
            }
        }
        if let Some(ts_str) = v.get("timestamp").and_then(|t| t.as_str()) {
            if let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) {
                let u = ts.with_timezone(&Utc);
                started_at = Some(started_at.map_or(u, |s| s.min(u)));
                ended_at = Some(ended_at.map_or(u, |e| e.max(u)));
            }
        }

        match kind {
            "user" | "assistant" => {
                let ts = v
                    .get("timestamp")
                    .and_then(|t| t.as_str())
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);
                let turn_id = v
                    .get("uuid")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                let role = if kind == "user" {
                    Role::User
                } else {
                    Role::Assistant
                };

                let mut text_content = String::new();
                let mut tool_calls = Vec::new();
                let mut tool_results = Vec::new();

                if let Some(msg_model) = v
                    .get("message")
                    .and_then(|m| m.get("model"))
                    .and_then(|m| m.as_str())
                {
                    if model.is_none() {
                        model = Some(msg_model.to_string());
                    }
                }

                if let Some(content) = v.get("message").and_then(|m| m.get("content")) {
                    match content {
                        Value::String(s) => text_content.push_str(s),
                        Value::Array(blocks) => {
                            for b in blocks {
                                let btype =
                                    b.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                match btype {
                                    "text" => {
                                        if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                                            text_content.push_str(t);
                                            text_content.push('\n');
                                        }
                                    }
                                    "tool_use" => {
                                        tool_calls.push(ToolCall {
                                            id: b
                                                .get("id")
                                                .and_then(|s| s.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            name: b
                                                .get("name")
                                                .and_then(|s| s.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            input: b.get("input").cloned().unwrap_or(Value::Null),
                                        });
                                    }
                                    "tool_result" => {
                                        tool_results.push(ToolResult {
                                            tool_use_id: b
                                                .get("tool_use_id")
                                                .and_then(|s| s.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            is_error: b
                                                .get("is_error")
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(false),
                                            content: stringify_content(
                                                b.get("content").unwrap_or(&Value::Null),
                                            ),
                                        });
                                    }
                                    "thinking" => {
                                        thinking_blocks += 1;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }

                turns.push(Turn {
                    turn_id,
                    role,
                    content_text: text_content,
                    tool_calls,
                    tool_results,
                    timestamp: ts,
                });
            }
            _ => {}
        }
    }

    if session_id.is_empty() {
        return Err(anyhow!(
            "no sessionId found in {} — not a Claude Code jsonl?",
            path.display()
        ));
    }

    let now = Utc::now();
    Ok(NormalizedSession {
        session_id,
        agent_slug: AgentSlug::ClaudeCode,
        model,
        working_dir,
        git_head_at_start: None,
        started_at: started_at.unwrap_or(now),
        ended_at: ended_at.unwrap_or(now),
        turns,
        thinking_blocks,
    })
}

fn stringify_content(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .filter_map(|b| {
                b.get("text")
                    .and_then(|t| t.as_str().map(String::from))
                    .or_else(|| Some(b.to_string()))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => v.to_string(),
    }
}

/// Build code events from an already-normalised session.
fn extract_events(ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>> {
    let mut drafts = Vec::new();

    // Map tool_use_id -> is_error from subsequent user turns' tool_results.
    let mut errors: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    for t in &ns.turns {
        for tr in &t.tool_results {
            errors.insert(tr.tool_use_id.clone(), tr.is_error);
        }
    }

    // Maintain a tiny in-memory content map so intra-session chains emit
    // sane "before" values for Edit/MultiEdit.
    let mut files: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for t in &ns.turns {
        if !matches!(t.role, Role::Assistant) {
            continue;
        }
        for tc in &t.tool_calls {
            let rejected = *errors.get(&tc.id).unwrap_or(&false);
            match tc.name.as_str() {
                "Write" => {
                    let Some(file_path) = tc
                        .input
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                    else {
                        continue;
                    };
                    let content = tc
                        .input
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let before = files.get(&file_path).cloned().unwrap_or_default();
                    if !rejected {
                        files.insert(file_path.clone(), content.clone());
                    }
                    drafts.push(CodeEventDraft {
                        session_id: Some(ns.session_id.clone()),
                        agent_slug: AgentSlug::ClaudeCode,
                        turn_id: Some(t.turn_id.clone()),
                        timestamp: t.timestamp,
                        file_path: normalise_path(&ns.working_dir, &file_path),
                        operation: if before.is_empty() {
                            Operation::Create
                        } else {
                            Operation::Edit
                        },
                        rename_from: None,
                        before_content: before,
                        after_content: content,
                        rejected,
                        metadata: Default::default(),
                        event_id: None,
                        intra_call_parent: None,
                    });
                }
                "Edit" => {
                    let Some(file_path) = tc
                        .input
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                    else {
                        continue;
                    };
                    let old = tc
                        .input
                        .get("old_string")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let new = tc
                        .input
                        .get("new_string")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let replace_all = tc
                        .input
                        .get("replace_all")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let before = files.get(&file_path).cloned().unwrap_or_default();
                    let after = if replace_all {
                        before.replace(&old, &new)
                    } else {
                        before.replacen(&old, &new, 1)
                    };
                    if !rejected {
                        files.insert(file_path.clone(), after.clone());
                    }
                    drafts.push(CodeEventDraft {
                        session_id: Some(ns.session_id.clone()),
                        agent_slug: AgentSlug::ClaudeCode,
                        turn_id: Some(t.turn_id.clone()),
                        timestamp: t.timestamp,
                        file_path: normalise_path(&ns.working_dir, &file_path),
                        operation: Operation::Edit,
                        rename_from: None,
                        before_content: before,
                        after_content: after,
                        rejected,
                        metadata: Default::default(),
                        event_id: None,
                        intra_call_parent: None,
                    });
                }
                "MultiEdit" => {
                    let Some(file_path) = tc
                        .input
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                    else {
                        continue;
                    };
                    let Some(edits) = tc.input.get("edits").and_then(|e| e.as_array()) else {
                        continue;
                    };
                    let mut running = files.get(&file_path).cloned().unwrap_or_default();
                    let mut prev_id: Option<String> = None;
                    for edit in edits {
                        let old = edit
                            .get("old_string")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let new = edit
                            .get("new_string")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let replace_all = edit
                            .get("replace_all")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let before = running.clone();
                        let after = if replace_all {
                            before.replace(&old, &new)
                        } else {
                            before.replacen(&old, &new, 1)
                        };
                        running = after.clone();
                        let ev_id = drift_core::CodeEvent::new_id();
                        drafts.push(CodeEventDraft {
                            session_id: Some(ns.session_id.clone()),
                            agent_slug: AgentSlug::ClaudeCode,
                            turn_id: Some(t.turn_id.clone()),
                            timestamp: t.timestamp,
                            file_path: normalise_path(&ns.working_dir, &file_path),
                            operation: Operation::Edit,
                            rename_from: None,
                            before_content: before,
                            after_content: after,
                            rejected,
                            metadata: {
                                let mut m = serde_json::Map::new();
                                m.insert("multi_edit".into(), serde_json::Value::Bool(true));
                                m
                            },
                            event_id: Some(ev_id.clone()),
                            intra_call_parent: prev_id.clone(),
                        });
                        prev_id = Some(ev_id);
                    }
                    if !rejected {
                        files.insert(file_path.clone(), running);
                    }
                }
                "Bash" => {
                    let cmd = tc
                        .input
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    for intent in detect_intents(cmd) {
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

fn shell_intent_to_draft(
    ns: &NormalizedSession,
    t: &Turn,
    rejected: bool,
    intent: ShellIntent,
) -> Option<CodeEventDraft> {
    let mut md = serde_json::Map::new();
    md.insert(
        "detected_via".into(),
        serde_json::Value::String("shell-lexer".into()),
    );
    match intent {
        ShellIntent::Move { from, to } => Some(CodeEventDraft {
            session_id: Some(ns.session_id.clone()),
            agent_slug: AgentSlug::ClaudeCode,
            turn_id: Some(t.turn_id.clone()),
            timestamp: t.timestamp,
            file_path: normalise_path(&ns.working_dir, &to),
            operation: Operation::Rename,
            rename_from: Some(normalise_path(&ns.working_dir, &from)),
            before_content: String::new(),
            after_content: String::new(),
            rejected,
            metadata: md,
            event_id: None,
            intra_call_parent: None,
        }),
        ShellIntent::Remove { path } => Some(CodeEventDraft {
            session_id: Some(ns.session_id.clone()),
            agent_slug: AgentSlug::ClaudeCode,
            turn_id: Some(t.turn_id.clone()),
            timestamp: t.timestamp,
            file_path: normalise_path(&ns.working_dir, &path),
            operation: Operation::Delete,
            rename_from: None,
            before_content: String::new(),
            after_content: String::new(),
            rejected,
            metadata: md,
            event_id: None,
            intra_call_parent: None,
        }),
        // Copy / RedirectWrite / SedInPlace / PythonWriteBestEffort: we flag
        // them as Edit with best-effort metadata; their actual content is
        // learnt on the next SHA ladder tick.
        ShellIntent::Copy { to, .. } => Some(CodeEventDraft {
            session_id: Some(ns.session_id.clone()),
            agent_slug: AgentSlug::ClaudeCode,
            turn_id: Some(t.turn_id.clone()),
            timestamp: t.timestamp,
            file_path: normalise_path(&ns.working_dir, &to),
            operation: Operation::Edit,
            rename_from: None,
            before_content: String::new(),
            after_content: String::new(),
            rejected,
            metadata: md,
            event_id: None,
            intra_call_parent: None,
        }),
        ShellIntent::RedirectWrite { path, .. }
        | ShellIntent::SedInPlace { path }
        | ShellIntent::PythonWriteBestEffort { path } => Some(CodeEventDraft {
            session_id: Some(ns.session_id.clone()),
            agent_slug: AgentSlug::ClaudeCode,
            turn_id: Some(t.turn_id.clone()),
            timestamp: t.timestamp,
            file_path: normalise_path(&ns.working_dir, &path),
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

fn normalise_path(working_dir: &Option<PathBuf>, p: &str) -> String {
    if let Some(wd) = working_dir {
        if let Ok(rel) = Path::new(p).strip_prefix(wd) {
            return rel.to_string_lossy().to_string();
        }
    }
    p.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_write_then_edit_chain() {
        let json = r#"{"type":"assistant","uuid":"u1","timestamp":"2026-04-22T10:00:00Z","sessionId":"sess","cwd":"/repo","message":{"role":"assistant","content":[{"type":"tool_use","id":"tc1","name":"Write","input":{"file_path":"hi.txt","content":"hello\n"}}]}}
{"type":"assistant","uuid":"u2","timestamp":"2026-04-22T10:01:00Z","sessionId":"sess","cwd":"/repo","message":{"role":"assistant","content":[{"type":"tool_use","id":"tc2","name":"Edit","input":{"file_path":"hi.txt","old_string":"hello","new_string":"hello\ndrift","replace_all":false}}]}}
"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        std::fs::write(&p, json).unwrap();
        let ns = parse_file(&p).unwrap();
        let drafts = extract_events(&ns).unwrap();
        assert_eq!(drafts.len(), 2);
        assert_eq!(drafts[0].operation, Operation::Create);
        // Write "hello\n" -> Edit replaces "hello" with "hello\ndrift",
        // so the original trailing "\n" stays.
        assert_eq!(drafts[1].after_content, "hello\ndrift\n");
    }

    #[test]
    fn tool_result_error_marks_rejected() {
        let json = r#"{"type":"assistant","uuid":"u1","timestamp":"2026-04-22T10:00:00Z","sessionId":"sess","cwd":"/repo","message":{"role":"assistant","content":[{"type":"tool_use","id":"tc1","name":"Write","input":{"file_path":"forbid.txt","content":"no"}}]}}
{"type":"user","uuid":"u2","timestamp":"2026-04-22T10:00:01Z","sessionId":"sess","cwd":"/repo","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tc1","is_error":true,"content":"denied"}]}}
"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        std::fs::write(&p, json).unwrap();
        let ns = parse_file(&p).unwrap();
        let drafts = extract_events(&ns).unwrap();
        assert_eq!(drafts.len(), 1);
        assert!(drafts[0].rejected);
    }
}
