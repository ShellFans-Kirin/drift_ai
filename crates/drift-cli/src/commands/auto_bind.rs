use super::open_store;
use anyhow::Result;
use chrono::{DateTime, Utc};
use drift_core::git;
use std::path::Path;

pub fn run(repo: &Path) -> Result<()> {
    let store = open_store(repo)?;
    let commits = git::list_commits(repo, None)?;
    if commits.is_empty() {
        println!("(no commits yet)");
        return Ok(());
    }

    let sessions = store.list_sessions(None)?;
    let mut bound = 0usize;
    for s in &sessions {
        let target = closest_commit(&commits, s.ended_at);
        if let Some(c) = target {
            let events = store.events_for_session(&s.session_id)?;
            for ev in &events {
                store.bind_commit(&ev.event_id, &c.sha)?;
            }
            let body = format!(
                "[{}] {} turns; {}\n",
                s.agent_slug.as_str(),
                s.turn_count,
                s.summary
            );
            git::add_note(repo, &c.sha, &body)?;
            bound += events.len();
        }
    }
    println!("auto-bind bound {} event(s) across {} session(s).", bound, sessions.len());
    Ok(())
}

fn closest_commit(commits: &[git::CommitRow], ended_at: DateTime<Utc>) -> Option<&git::CommitRow> {
    commits
        .iter()
        .filter(|c| c.committed_at_unix >= ended_at.timestamp())
        .min_by_key(|c| c.committed_at_unix - ended_at.timestamp())
        .or_else(|| commits.first())
}
