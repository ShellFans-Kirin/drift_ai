use super::{open_store, sessions_dir};
use anyhow::{Context, Result};
use chrono::DateTime;
use drift_core::attribution::commit_drafts;
use drift_core::compaction::{summary_to_markdown, CompactionProvider, MockProvider};
use drift_connectors::default_connectors;
use std::path::Path;

pub fn run(
    repo: &Path,
    session_filter: Option<&str>,
    agent_filter: Option<&str>,
    all_since: Option<&str>,
) -> Result<()> {
    super::init::run(repo).ok(); // ensure .prompts/ exists
    let store = open_store(repo)?;
    let connectors = default_connectors();
    let provider: Box<dyn CompactionProvider> = Box::new(MockProvider);
    let sessions_dir_p = sessions_dir(repo);
    std::fs::create_dir_all(&sessions_dir_p)?;

    let since_ts = all_since
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.naive_utc());

    let mut n_sessions = 0;
    let mut n_events = 0;

    for c in connectors {
        if let Some(a) = agent_filter {
            if c.agent_slug() != a {
                continue;
            }
        }
        let refs = c.discover()?;
        for r in refs {
            let ns = match c.parse(&r) {
                Ok(ns) => ns,
                Err(e) => {
                    tracing::warn!("skip {:?}: {}", r.path, e);
                    continue;
                }
            };
            if let Some(sid) = session_filter {
                if ns.session_id != sid {
                    continue;
                }
            }
            if let Some(ts) = since_ts {
                if ns.started_at.naive_utc() < ts {
                    continue;
                }
            }
            let drafts = c.extract_code_events(&ns)?;
            let events = commit_drafts(&store, drafts)?;
            n_events += events.len();

            let summary = provider.compact(&ns)?;
            let md = summary_to_markdown(&summary);
            let date = ns.started_at.format("%Y-%m-%d");
            let short = ns.session_id.chars().take(8).collect::<String>();
            let filename = format!("{}-{}-{}.md", date, c.agent_slug(), short);
            let path = sessions_dir_p.join(&filename);
            std::fs::write(&path, md).with_context(|| format!("write {}", path.display()))?;

            store.insert_session_meta(
                &ns.session_id,
                ns.agent_slug,
                ns.model.as_deref(),
                ns.working_dir.as_ref().map(|p| p.to_string_lossy().to_string()).as_deref(),
                ns.started_at,
                ns.ended_at,
                ns.turns.len() as u32,
                ns.thinking_blocks,
                Some(&filename),
                &summary.summary,
            )?;
            n_sessions += 1;
        }
    }

    println!(
        "Captured {} session(s), wrote {} event(s) to {}",
        n_sessions,
        n_events,
        super::events_db_path(repo).display()
    );
    Ok(())
}
