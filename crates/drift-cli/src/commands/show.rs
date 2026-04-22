use super::{open_store, sessions_dir};
use anyhow::{Context, Result};
use std::path::Path;

pub fn run(repo: &Path, session_id: &str) -> Result<()> {
    // Find the file that begins with anything containing the short id.
    let dir = sessions_dir(repo);
    let short = session_id.chars().take(8).collect::<String>();
    if dir.exists() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains(&short) {
                let text = std::fs::read_to_string(entry.path())
                    .with_context(|| format!("read {}", entry.path().display()))?;
                println!("{}", text);
                return Ok(());
            }
        }
    }
    // Fallback — show session meta row.
    let store = open_store(repo)?;
    let rows = store.list_sessions(None)?;
    for r in rows {
        if r.session_id == session_id || r.session_id.starts_with(&short) {
            println!(
                "session_id: {}\nagent: {}\nturns: {}\nsummary: {}",
                r.session_id,
                r.agent_slug.as_str(),
                r.turn_count,
                r.summary
            );
            return Ok(());
        }
    }
    anyhow::bail!("no session matching `{}`", session_id)
}
