use anyhow::Result;
use notify::{RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn run(repo: &Path) -> Result<()> {
    let (tx, rx) = channel();
    let mut w = notify::recommended_watcher(tx)?;

    let home = dirs::home_dir();
    if let Some(h) = &home {
        let claude = h.join(".claude").join("projects");
        if claude.exists() {
            w.watch(&claude, RecursiveMode::Recursive)?;
        }
        let codex = h.join(".codex").join("sessions");
        if codex.exists() {
            w.watch(&codex, RecursiveMode::Recursive)?;
        }
    }

    println!("drift watch: monitoring Claude Code + Codex session dirs. Ctrl+C to stop.");

    let mut last_run = std::time::Instant::now() - Duration::from_secs(60);
    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(_event) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(e) => {
                tracing::warn!("watch channel closed: {}", e);
                break;
            }
        }
        // Debounce: run at most every 3 seconds.
        if last_run.elapsed() >= Duration::from_secs(3) {
            if let Err(e) = super::capture::run(repo, None, None, None) {
                tracing::warn!("capture failed: {}", e);
            }
            last_run = std::time::Instant::now();
        }
    }
    Ok(())
}
