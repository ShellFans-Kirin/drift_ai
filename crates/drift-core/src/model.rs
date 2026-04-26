//! Normalised session + code event model.
//!
//! This is the single source of truth the rest of the system works against.
//! The shapes here are the direct materialisation of
//! `docs/PHASE0-PROPOSAL.md` §D.1 and §D.2 — when the proposal and the code
//! disagree, the proposal wins and the code is wrong.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Which side produced a turn or event. `Human` is the "no AI session
/// produced this" slug (see VISION.md — not an authorship claim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentSlug {
    ClaudeCode,
    Codex,
    Cursor,
    Aider,
    Human,
    Unknown,
}

impl AgentSlug {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentSlug::ClaudeCode => "claude-code",
            AgentSlug::Codex => "codex",
            AgentSlug::Cursor => "cursor",
            AgentSlug::Aider => "aider",
            AgentSlug::Human => "human",
            AgentSlug::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "claude-code" => AgentSlug::ClaudeCode,
            "codex" => AgentSlug::Codex,
            "cursor" => AgentSlug::Cursor,
            "aider" => AgentSlug::Aider,
            "human" => AgentSlug::Human,
            _ => AgentSlug::Unknown,
        }
    }
}

/// File-level operation observed in a session tool call or inferred from a
/// SHA drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Create,
    Edit,
    Delete,
    Rename,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Operation::Create => "create",
            Operation::Edit => "edit",
            Operation::Delete => "delete",
            Operation::Rename => "rename",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "create" => Operation::Create,
            "edit" => Operation::Edit,
            "delete" => Operation::Delete,
            "rename" => Operation::Rename,
            _ => return None,
        })
    }
}

/// Role a single turn plays in the normalised conversation view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    ToolResult,
}

/// Raw tool invocation in an assistant turn — agent-agnostic shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// JSON-typed input. Arbitrary; connectors expose what the agent emits.
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub is_error: bool,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub turn_id: String,
    pub role: Role,
    pub content_text: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
    pub timestamp: DateTime<Utc>,
}

/// Flattened session view shared by every connector. See PROPOSAL §D.1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSession {
    pub session_id: String,
    pub agent_slug: AgentSlug,
    pub model: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub git_head_at_start: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub turns: Vec<Turn>,
    pub thinking_blocks: u32,
}

/// One row per observed (or inferred) file mutation. See PROPOSAL §D.2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub agent_slug: AgentSlug,
    pub turn_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub file_path: String,
    pub operation: Operation,
    pub rename_from: Option<String>,
    pub line_ranges_before: Vec<(u32, u32)>,
    pub line_ranges_after: Vec<(u32, u32)>,
    pub diff_hunks: String,
    pub rejected: bool,
    pub parent_event_id: Option<String>,
    pub content_sha256_after: Option<String>,
    pub bound_commit_sha: Option<String>,
    /// Free-form metadata. Connectors set hints like `detected_via=shell-lexer`;
    /// stored as JSON in the DB.
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl CodeEvent {
    pub fn new_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
