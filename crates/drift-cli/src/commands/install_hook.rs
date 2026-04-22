use anyhow::{Context, Result};
use std::path::Path;

const HOOK: &str = r#"#!/usr/bin/env bash
# Drift AI post-commit hook. Fire-and-forget; never block the commit.
(
  drift capture >/dev/null 2>&1 || true
  drift auto-bind >/dev/null 2>&1 || true
) &
exit 0
"#;

pub fn run(repo: &Path) -> Result<()> {
    let hook = repo.join(".git").join("hooks").join("post-commit");
    if let Some(parent) = hook.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&hook, HOOK).with_context(|| format!("write {}", hook.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&hook)?.permissions();
        p.set_mode(0o755);
        std::fs::set_permissions(&hook, p)?;
    }
    println!("installed post-commit hook at {}", hook.display());
    Ok(())
}
