use super::open_store;
use anyhow::Result;
use std::path::Path;

pub fn run(repo: &Path, session_id: &str) -> Result<()> {
    let store = open_store(repo)?;
    // Accept short prefix.
    let rows = store.list_sessions(None)?;
    let full = rows
        .iter()
        .find(|r| r.session_id == session_id || r.session_id.starts_with(session_id))
        .map(|r| r.session_id.clone())
        .unwrap_or_else(|| session_id.to_string());

    let events = store.events_for_session(&full)?;
    if events.is_empty() {
        println!("(no events for session {})", session_id);
        return Ok(());
    }
    let accepted = events.iter().filter(|e| !e.rejected).count();
    let rejected = events.len() - accepted;
    let files: std::collections::BTreeSet<_> = events.iter().map(|e| e.file_path.clone()).collect();
    println!(
        "Session {} — {} events total ({} accepted, {} rejected)",
        full.chars().take(8).collect::<String>(),
        events.len(),
        accepted,
        rejected
    );
    println!("  files_touched: {}", files.iter().cloned().collect::<Vec<_>>().join(", "));
    for ev in &events {
        println!(
            "  {}  {:<6}  {:<40}  {}",
            ev.timestamp.format("%H:%M:%S"),
            ev.operation.as_str(),
            ev.file_path,
            if ev.rejected { "[REJECTED]" } else { "" }
        );
    }
    Ok(())
}
