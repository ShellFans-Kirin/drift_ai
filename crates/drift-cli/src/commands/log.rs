use super::open_store;
use anyhow::Result;
use drift_core::git;
use std::path::Path;

pub fn run(repo: &Path, git_args: &[String]) -> Result<()> {
    let range = if git_args.is_empty() {
        None
    } else {
        Some(git_args.join(" "))
    };
    let commits = git::list_commits(repo, range.as_deref())?;
    let store = open_store(repo)?;

    for c in commits {
        println!("commit {} — {}", &c.sha[..c.sha.len().min(10)], c.subject);
        let events = store.events_for_commit(&c.sha).unwrap_or_default();
        if events.is_empty() {
            if let Ok(Some(note)) = git::show_note(repo, &c.sha) {
                for line in note.lines().take(6) {
                    println!("   {}", line);
                }
            }
            continue;
        }
        let mut by_agent: std::collections::BTreeMap<&str, (u32, u32)> = Default::default();
        for ev in &events {
            let slot = by_agent.entry(ev.agent_slug.as_str()).or_default();
            if ev.rejected {
                slot.1 += 1;
            } else {
                slot.0 += 1;
            }
        }
        for (agent, (acc, rej)) in by_agent {
            let glyph = if agent == "human" { "✋" } else { "💭" };
            println!(
                "   {} [{}] {} events accepted, {} rejected",
                glyph, agent, acc, rej
            );
        }
    }
    Ok(())
}
