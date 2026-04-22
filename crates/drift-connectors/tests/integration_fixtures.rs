//! End-to-end fixture tests for both first-class connectors.

use drift_connectors::{
    claude_code::ClaudeCodeConnector, codex::CodexConnector, SessionConnector, SessionRef,
};
use drift_core::model::Operation;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    // crate lives at crates/drift-connectors; repo root is two up.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("tests/fixtures");
    p.push(rel);
    assert!(p.exists(), "fixture missing: {}", p.display());
    p
}

#[test]
fn claude_code_write_edit_chain_produces_two_events() {
    let c = ClaudeCodeConnector::new(PathBuf::from("/dev/null"));
    let r = SessionRef {
        agent_slug: "claude-code",
        path: fixture("claude-code/02-write-edit.jsonl"),
    };
    let ns = c.parse(&r).unwrap();
    let drafts = c.extract_code_events(&ns).unwrap();
    assert_eq!(drafts.len(), 2);
    assert_eq!(drafts[0].operation, Operation::Create);
    assert!(drafts.iter().all(|d| !d.rejected));
}

#[test]
fn claude_code_failed_write_marks_rejected() {
    let c = ClaudeCodeConnector::new(PathBuf::from("/dev/null"));
    let r = SessionRef {
        agent_slug: "claude-code",
        path: fixture("claude-code/03-failed-retry.jsonl"),
    };
    let ns = c.parse(&r).unwrap();
    let drafts = c.extract_code_events(&ns).unwrap();
    assert_eq!(drafts.len(), 1);
    assert!(drafts[0].rejected);
}

#[test]
fn claude_code_bash_git_mv_emits_rename() {
    let c = ClaudeCodeConnector::new(PathBuf::from("/dev/null"));
    let r = SessionRef {
        agent_slug: "claude-code",
        path: fixture("claude-code/04-mv-rename-via-bash.jsonl"),
    };
    let ns = c.parse(&r).unwrap();
    let drafts = c.extract_code_events(&ns).unwrap();
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].operation, Operation::Rename);
    assert_eq!(drafts[0].rename_from.as_deref(), Some("old.rs"));
    assert_eq!(drafts[0].file_path, "new.rs");
}

#[test]
fn codex_apply_patch_add() {
    let c = CodexConnector::new(PathBuf::from("/dev/null"));
    let r = SessionRef {
        agent_slug: "codex",
        path: fixture("codex/02-apply-patch-add.jsonl"),
    };
    let ns = c.parse(&r).unwrap();
    let drafts = c.extract_code_events(&ns).unwrap();
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].operation, Operation::Create);
    assert!(drafts[0].after_content.contains("hello"));
    assert!(drafts[0].after_content.contains("drift"));
}

#[test]
fn codex_apply_patch_move() {
    let c = CodexConnector::new(PathBuf::from("/dev/null"));
    let r = SessionRef {
        agent_slug: "codex",
        path: fixture("codex/03-apply-patch-move.jsonl"),
    };
    let ns = c.parse(&r).unwrap();
    let drafts = c.extract_code_events(&ns).unwrap();
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].operation, Operation::Rename);
    assert_eq!(drafts[0].rename_from.as_deref(), Some("old.rs"));
    assert_eq!(drafts[0].file_path, "new.rs");
}

#[test]
fn claude_code_plain_chat_zero_events() {
    let c = ClaudeCodeConnector::new(PathBuf::from("/dev/null"));
    let r = SessionRef {
        agent_slug: "claude-code",
        path: fixture("claude-code/01-plain-chat.jsonl"),
    };
    let ns = c.parse(&r).unwrap();
    let drafts = c.extract_code_events(&ns).unwrap();
    assert_eq!(drafts.len(), 0);
    assert_eq!(ns.turns.len(), 2);
}
