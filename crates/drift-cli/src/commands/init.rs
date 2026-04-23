use super::{prompts_dir, sessions_dir};
use anyhow::{Context, Result};
use drift_core::config;
use std::path::Path;

pub fn run(repo: &Path) -> Result<()> {
    let p = prompts_dir(repo);
    std::fs::create_dir_all(&p).with_context(|| format!("create {}", p.display()))?;
    std::fs::create_dir_all(sessions_dir(repo))?;

    // Only write the default config when it doesn't yet exist — this keeps
    // `drift init` (and the implicit init from `drift capture`) idempotent,
    // so a user-edited `[compaction]` block survives subsequent runs.
    let cfg_path = config::project_config_path(repo);
    if !cfg_path.exists() {
        config::write_project_default(repo)?;
    }

    let gitignore = p.join(".gitignore");
    if !gitignore.exists() {
        let mut ignore = String::new();
        ignore.push_str("# Drift AI — content-addressed diff cache (always ignored)\n");
        ignore.push_str("cache/\n");
        ignore.push_str(
            "# Set [attribution].db_in_git = false in config.toml to commit privately.\n",
        );
        ignore.push_str("# events.db\n");
        std::fs::write(&gitignore, ignore)?;
    }
    println!(
        "Initialised {} with config at {}",
        p.display(),
        cfg_path.display()
    );
    Ok(())
}
