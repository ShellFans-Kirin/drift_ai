use super::open_store;
use anyhow::Result;
use drift_core::git;
use std::path::Path;

pub fn run(repo: &Path, commit: &str, session_id: &str) -> Result<()> {
    let store = open_store(repo)?;
    // Resolve short session id to full.
    let rows = store.list_sessions(None)?;
    let full = rows
        .iter()
        .find(|r| r.session_id == session_id || r.session_id.starts_with(session_id))
        .map(|r| r.session_id.clone())
        .ok_or_else(|| anyhow::anyhow!("no captured session matches `{}`", session_id))?;

    let events = store.events_for_session(&full)?;
    for ev in &events {
        store.bind_commit(&ev.event_id, commit)?;
    }

    let note = build_note_line(&rows, &full);
    git::add_note(repo, commit, &format!("{}\n", note))?;
    println!("Bound {} to {} ({} events).", full, commit, events.len());
    Ok(())
}

fn build_note_line(rows: &[drift_core::store::SessionRow], full_sid: &str) -> String {
    if let Some(s) = rows.iter().find(|r| r.session_id == full_sid) {
        format!(
            "[{}] {} turns; {}",
            s.agent_slug.as_str(),
            s.turn_count,
            s.summary
        )
    } else {
        format!("[unknown] session {}", full_sid)
    }
}
