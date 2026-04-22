use super::open_store;
use anyhow::Result;
use std::path::Path;

pub fn run(repo: &Path, since: Option<&str>) -> Result<()> {
    let store = open_store(repo)?;
    let rows = store.rejected_events(since)?;
    if rows.is_empty() {
        println!("(no rejected events)");
        return Ok(());
    }
    for ev in rows {
        println!(
            "{}  {:<12}  {:<6}  {}",
            ev.timestamp.format("%Y-%m-%d %H:%M"),
            ev.agent_slug.as_str(),
            ev.operation.as_str(),
            ev.file_path
        );
    }
    Ok(())
}
