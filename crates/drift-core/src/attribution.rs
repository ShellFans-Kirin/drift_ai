//! Attribution engine: turn [`NormalizedSession`]s into [`CodeEvent`]s,
//! plus the SHA-256 ladder that detects human edits between AI events.
//!
//! The engine is format-agnostic: individual connectors produce sequences of
//! `CodeEventDraft`s from tool calls, and this module folds them into the
//! `EventStore` while maintaining parent/lineage pointers.

use crate::diff::{line_ranges, sha256_hex, unified_diff};
use crate::model::{AgentSlug, CodeEvent, Operation};
use crate::store::EventStore;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;

/// Partial event emitted by a connector before the store fills in lineage
/// pointers (`parent_event_id`) and `content_sha256_after`.
#[derive(Debug, Clone)]
pub struct CodeEventDraft {
    pub session_id: Option<String>,
    pub agent_slug: AgentSlug,
    pub turn_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub file_path: String,
    pub operation: Operation,
    pub rename_from: Option<String>,
    pub before_content: String,
    pub after_content: String,
    pub rejected: bool,
    pub metadata: serde_json::Map<String, serde_json::Value>,
    /// Pre-assigned event id. Set when the connector needs to chain events
    /// inside a single tool call (e.g. Claude Code `MultiEdit`).
    pub event_id: Option<String>,
    /// Optional intra-call parent; rare — only for connector-local chains.
    pub intra_call_parent: Option<String>,
}

/// Persist drafts as full `CodeEvent`s, computing diff, line ranges, SHA,
/// and parent pointer.
pub fn commit_drafts(store: &EventStore, drafts: Vec<CodeEventDraft>) -> Result<Vec<CodeEvent>> {
    let mut out = Vec::new();
    for d in drafts {
        let diff_hunks = unified_diff(&d.before_content, &d.after_content, &d.file_path);
        let (lrb, lra) = line_ranges(&d.before_content, &d.after_content);

        let parent = if let Some(p) = d.intra_call_parent.clone() {
            Some(p)
        } else {
            store
                .last_known_sha(&d.file_path)?
                .map(|(_sha, last_event_id)| last_event_id)
        };

        let sha_after = if d.rejected {
            None
        } else {
            Some(sha256_hex(&d.after_content))
        };

        let event = CodeEvent {
            event_id: d.event_id.clone().unwrap_or_else(CodeEvent::new_id),
            session_id: d.session_id,
            agent_slug: d.agent_slug,
            turn_id: d.turn_id,
            timestamp: d.timestamp,
            file_path: d.file_path,
            operation: d.operation,
            rename_from: d.rename_from,
            line_ranges_before: lrb,
            line_ranges_after: lra,
            diff_hunks,
            rejected: d.rejected,
            parent_event_id: parent,
            content_sha256_after: sha_after,
            bound_commit_sha: None,
            metadata: d.metadata,
        };
        store.insert_event(&event)?;
        out.push(event);
    }
    Ok(out)
}

/// SHA-ladder human-edit detection. Compares the on-disk content of each
/// tracked file against the last known SHA; emits `agent_slug="human"` events
/// on drift. See PROPOSAL §D.3.
pub fn detect_human_edits(store: &EventStore, repo_root: &Path) -> Result<Vec<CodeEvent>> {
    // Pull a snapshot of tracked files with last-known SHAs.
    let mut stmt_paths = store
        .conn
        .prepare("SELECT file_path, last_known_sha256, last_event_id FROM file_shas")?;
    let rows = stmt_paths.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;

    let mut new_events = Vec::new();
    for r in rows {
        let (rel_path, last_sha, parent_event_id) = r?;
        let abs = repo_root.join(&rel_path);
        let current = match std::fs::read_to_string(&abs) {
            Ok(c) => c,
            Err(_) => continue, // file deleted; handled elsewhere
        };
        let cur_sha = sha256_hex(&current);
        if cur_sha == last_sha {
            continue;
        }
        // Emit a human-slug event; before_content is unknown (blob cache
        // not implemented in v0.1.0), so the diff is best-effort with an
        // empty "before". The SHA drift itself is the reliable signal.
        let diff_hunks = unified_diff("", &current, &rel_path);
        let (lrb, lra) = line_ranges("", &current);
        let ev = CodeEvent {
            event_id: CodeEvent::new_id(),
            session_id: None,
            agent_slug: AgentSlug::Human,
            turn_id: None,
            timestamp: Utc::now(),
            file_path: rel_path.clone(),
            operation: Operation::Edit,
            rename_from: None,
            line_ranges_before: lrb,
            line_ranges_after: lra,
            diff_hunks,
            rejected: false,
            parent_event_id: Some(parent_event_id.clone()),
            content_sha256_after: Some(cur_sha),
            bound_commit_sha: None,
            metadata: {
                let mut m = serde_json::Map::new();
                m.insert(
                    "detected_via".into(),
                    serde_json::Value::String("sha-ladder".into()),
                );
                m
            },
        };
        store.insert_event(&ev)?;
        new_events.push(ev);
    }
    Ok(new_events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn mk_draft(file: &str, before: &str, after: &str, rejected: bool) -> CodeEventDraft {
        CodeEventDraft {
            session_id: Some("s1".into()),
            agent_slug: AgentSlug::ClaudeCode,
            turn_id: Some("t1".into()),
            timestamp: Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap(),
            file_path: file.into(),
            operation: Operation::Edit,
            rename_from: None,
            before_content: before.into(),
            after_content: after.into(),
            rejected,
            metadata: Default::default(),
            event_id: None,
            intra_call_parent: None,
        }
    }

    #[test]
    fn rejected_draft_has_no_sha() {
        let s = EventStore::open_in_memory().unwrap();
        let d = mk_draft("src/a.rs", "a\n", "b\n", true);
        let out = commit_drafts(&s, vec![d]).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].rejected);
        assert!(out[0].content_sha256_after.is_none());
        // file_shas not updated for rejected events
        assert!(s.last_known_sha("src/a.rs").unwrap().is_none());
    }

    #[test]
    fn parent_chains_across_drafts() {
        let s = EventStore::open_in_memory().unwrap();
        let d1 = mk_draft("x", "", "a\n", false);
        let d2 = mk_draft("x", "a\n", "a\nb\n", false);
        let evs = commit_drafts(&s, vec![d1, d2]).unwrap();
        assert_eq!(evs.len(), 2);
        assert!(evs[1].parent_event_id.is_some());
        assert_eq!(evs[1].parent_event_id.as_ref().unwrap(), &evs[0].event_id);
    }
}
