//! End-to-end test for the SHA-256 ladder: an AI event records a SHA,
//! the file then drifts, and `detect_human_edits` emits a human event.

use drift_core::attribution::{commit_drafts, detect_human_edits, CodeEventDraft};
use drift_core::model::{AgentSlug, Operation};
use drift_core::EventStore;
use std::fs;

#[test]
fn human_edit_detected_on_sha_drift() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    let db_path = repo.join("events.db");
    let store = EventStore::open(&db_path).unwrap();

    // AI writes src/a.rs = "fn main() {}\n"
    let initial = "fn main() {}\n";
    let file = repo.join("src/a.rs");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, initial).unwrap();

    let draft = CodeEventDraft {
        session_id: Some("sess-1".into()),
        agent_slug: AgentSlug::ClaudeCode,
        turn_id: Some("t1".into()),
        timestamp: chrono::Utc::now(),
        file_path: "src/a.rs".into(),
        operation: Operation::Create,
        rename_from: None,
        before_content: String::new(),
        after_content: initial.to_string(),
        rejected: false,
        metadata: Default::default(),
        event_id: None,
        intra_call_parent: None,
    };
    let ai_events = commit_drafts(&store, vec![draft]).unwrap();
    assert_eq!(ai_events.len(), 1);

    // No drift yet.
    let none = detect_human_edits(&store, repo).unwrap();
    assert!(none.is_empty(), "expected no human events before drift");

    // Human edits the file.
    fs::write(&file, "fn main() { println!(\"hi\"); }\n").unwrap();

    let human = detect_human_edits(&store, repo).unwrap();
    assert_eq!(human.len(), 1, "expected one human event after drift");
    assert_eq!(human[0].agent_slug, AgentSlug::Human);
    assert_eq!(human[0].file_path, "src/a.rs");
    assert_eq!(
        human[0].parent_event_id.as_deref(),
        Some(ai_events[0].event_id.as_str())
    );
}
