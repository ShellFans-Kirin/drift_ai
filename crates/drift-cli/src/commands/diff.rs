use super::open_store;
use anyhow::Result;
use std::path::Path;

pub fn run(repo: &Path, event_id: &str) -> Result<()> {
    let store = open_store(repo)?;
    let ev = store
        .event_by_id(event_id)?
        .ok_or_else(|| anyhow::anyhow!("event {} not found", event_id))?;
    println!(
        "event {}  {:<12}  {}  {}",
        ev.event_id,
        ev.agent_slug.as_str(),
        ev.operation.as_str(),
        ev.file_path
    );
    if ev.rejected {
        println!("(rejected)");
    }
    if let Some(p) = &ev.parent_event_id {
        println!("parent: {}", p);
    }
    if let Some(r) = &ev.rename_from {
        println!("rename_from: {}", r);
    }
    println!();
    println!("{}", ev.diff_hunks);
    Ok(())
}
