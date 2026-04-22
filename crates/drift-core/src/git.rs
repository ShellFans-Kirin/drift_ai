//! Minimal git helpers: notes binding, log walking, rename-follow fallback.
//! Shells out to `git`; keeping this crate free of libgit2 for portability.

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::Command;

pub const NOTES_REF: &str = "refs/notes/drift";

fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("spawn git {:?}", args))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("git {:?} failed: {}", args, stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

pub fn is_git_repo(repo: &Path) -> bool {
    git(repo, &["rev-parse", "--is-inside-work-tree"]).is_ok()
}

pub fn head_sha(repo: &Path) -> Result<String> {
    git(repo, &["rev-parse", "HEAD"])
}

pub fn list_commits(repo: &Path, range: Option<&str>) -> Result<Vec<CommitRow>> {
    let mut args = vec!["log", "--pretty=format:%H%x09%ct%x09%s"];
    if let Some(r) = range {
        args.push(r);
    }
    let out = git(repo, &args)?;
    let mut rows = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() == 3 {
            rows.push(CommitRow {
                sha: parts[0].into(),
                committed_at_unix: parts[1].parse().unwrap_or(0),
                subject: parts[2].into(),
            });
        }
    }
    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct CommitRow {
    pub sha: String,
    pub committed_at_unix: i64,
    pub subject: String,
}

pub fn add_note(repo: &Path, commit: &str, body: &str) -> Result<()> {
    // use `git notes --ref drift append` so multiple sessions can share one commit.
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        .args(["notes", "--ref", "drift", "append", "-F", "-", commit]);
    cmd.stdin(std::process::Stdio::piped());
    let mut child = cmd.spawn()?;
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("failed to open git stdin"))?
        .write_all(body.as_bytes())?;
    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("git notes append failed"));
    }
    Ok(())
}

pub fn show_note(repo: &Path, commit: &str) -> Result<Option<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["notes", "--ref", "drift", "show", commit])
        .output()?;
    if out.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&out.stdout).trim_end().to_string(),
        ))
    } else {
        Ok(None)
    }
}

/// `git log --follow --diff-filter=R --format=... -- path` — best-effort
/// rename fallback (Tier 2). Returns ordered list of prior paths.
pub fn rename_chain(repo: &Path, path: &str) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "log",
            "--follow",
            "--name-status",
            "--diff-filter=R",
            "--format=",
            "--",
            path,
        ])
        .output()?;
    if !out.status.success() {
        return Ok(vec![]);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut chain = Vec::new();
    for line in text.lines() {
        if line.starts_with('R') {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                chain.push(parts[1].to_string());
            }
        }
    }
    Ok(chain)
}

pub fn push_notes(repo: &Path, remote: &str) -> Result<()> {
    let _ = git(repo, &["push", remote, NOTES_REF])?;
    Ok(())
}

pub fn pull_notes(repo: &Path, remote: &str) -> Result<()> {
    let _ = git(
        repo,
        &["fetch", remote, &format!("{}:{}", NOTES_REF, NOTES_REF)],
    )?;
    Ok(())
}
