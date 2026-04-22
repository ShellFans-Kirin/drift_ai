use super::{prompts_dir, sessions_dir};
use anyhow::{Context, Result};
use drift_core::config;
use std::path::Path;

pub fn run(repo: &Path) -> Result<()> {
    let p = prompts_dir(repo);
    std::fs::create_dir_all(&p).with_context(|| format!("create {}", p.display()))?;
    std::fs::create_dir_all(sessions_dir(repo))?;
    let cfg_path = config::write_project_default(repo)?;
    let gitignore = p.join(".gitignore");
    let mut ignore = String::new();
    ignore.push_str("# Drift AI — content-addressed diff cache (always ignored)\n");
    ignore.push_str("cache/\n");
    // When db_in_git is false we'd add events.db; keep the line commented so
    // users see the knob exists.
    ignore.push_str("# Set [attribution].db_in_git = false in config.toml to commit privately.\n");
    ignore.push_str("# events.db\n");
    std::fs::write(&gitignore, ignore)?;
    println!(
        "Initialised {} with config at {}",
        p.display(),
        cfg_path.display()
    );
    Ok(())
}
