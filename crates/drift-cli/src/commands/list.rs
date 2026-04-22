use super::open_store;
use anyhow::Result;
use drift_core::model::AgentSlug;
use std::path::Path;

pub fn run(repo: &Path, agent: Option<&str>) -> Result<()> {
    let store = open_store(repo)?;
    let slug = agent.map(AgentSlug::parse);
    let rows = store.list_sessions(slug)?;
    if rows.is_empty() {
        println!("(no sessions captured yet; run `drift capture`)");
        return Ok(());
    }
    for r in rows {
        println!(
            "{}  {:<12}  turns={:<3} {}",
            &r.session_id.chars().take(8).collect::<String>(),
            r.agent_slug.as_str(),
            r.turn_count,
            r.summary
        );
    }
    Ok(())
}
