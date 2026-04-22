use anyhow::Result;
use drift_core::git;
use std::path::Path;

pub fn push(repo: &Path, remote: &str) -> Result<()> {
    git::push_notes(repo, remote)?;
    println!("pushed refs/notes/drift to {}", remote);
    Ok(())
}

pub fn pull(repo: &Path, remote: &str) -> Result<()> {
    git::pull_notes(repo, remote)?;
    println!("pulled refs/notes/drift from {}", remote);
    Ok(())
}
