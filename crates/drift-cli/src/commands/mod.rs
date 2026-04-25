pub mod auto_bind;
pub mod bind;
pub mod blame;
pub mod capture;
pub mod config;
pub mod cost;
pub mod diff;
pub mod handoff;
pub mod init;
pub mod install_hook;
pub mod list;
pub mod log;
pub mod rejected;
pub mod show;
pub mod sync;
pub mod trace;
pub mod watch;

use anyhow::{Context, Result};
use drift_core::EventStore;
use std::path::{Path, PathBuf};

pub fn prompts_dir(repo: &Path) -> PathBuf {
    repo.join(".prompts")
}
pub fn events_db_path(repo: &Path) -> PathBuf {
    prompts_dir(repo).join("events.db")
}
pub fn sessions_dir(repo: &Path) -> PathBuf {
    prompts_dir(repo).join("sessions")
}

pub fn open_store(repo: &Path) -> Result<EventStore> {
    let p = events_db_path(repo);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    EventStore::open(&p)
}
