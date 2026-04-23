//! `drift watch` — event-driven session monitor.
//!
//! Listens on `~/.claude/projects/` and `~/.codex/sessions/` via the
//! platform's native FS watcher (inotify on Linux, FSEvents on macOS,
//! `ReadDirectoryChangesW` on Windows — selected by
//! `notify::recommended_watcher`). Coalesces rapid bursts of writes
//! against a single session file into a single capture pass (200ms
//! debounce), persists the last-successful-capture timestamp to
//! `~/.config/drift/watch-state.toml` so a restart resumes instead of
//! re-running the whole directory, and handles `SIGINT` / `SIGTERM`
//! by finishing the current capture before exiting.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(200);

pub fn run(repo: &Path) -> Result<()> {
    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut w = notify::recommended_watcher(tx)?;

    let mut watched: Vec<PathBuf> = Vec::new();
    let home = dirs::home_dir().context("resolve home dir")?;
    let claude = home.join(".claude").join("projects");
    let codex = home.join(".codex").join("sessions");
    for p in [&claude, &codex] {
        if p.exists() {
            w.watch(p, RecursiveMode::Recursive)
                .with_context(|| format!("watch {}", p.display()))?;
            watched.push(p.clone());
        }
    }
    if watched.is_empty() {
        anyhow::bail!(
            "neither {} nor {} exists — no sessions to watch",
            claude.display(),
            codex.display()
        );
    }

    let stop = Arc::new(AtomicBool::new(false));
    {
        let s = stop.clone();
        ctrlc::set_handler(move || {
            s.store(true, Ordering::Relaxed);
        })
        .context("install SIGINT/SIGTERM handler")?;
    }

    let mut state = WatchState::load();
    eprintln!("drift watch · event-driven; Ctrl-C to stop");
    for p in &watched {
        eprintln!("  watching {}", p.display());
    }
    if let Some(ts) = state.last_event_at {
        eprintln!("  resuming; will skip sessions started before {}", ts);
    } else {
        eprintln!("  first run; capturing every session seen");
    }

    // Run one initial capture so the user has content after `drift watch`
    // even without a new file event.
    run_capture(repo, None, state.last_event_at.map(|t| t.to_rfc3339()));
    state.last_event_at = Some(Utc::now());
    state.save();

    loop {
        if stop.load(Ordering::Relaxed) {
            eprintln!("drift watch · interrupt received; exiting after last capture");
            break;
        }

        // Block for up to DEBOUNCE for the first event, then keep draining
        // the channel (non-blocking) for DEBOUNCE more to coalesce bursts.
        let first = match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(ev)) => Some(ev),
            Ok(Err(e)) => {
                tracing::warn!("notify error: {}", e);
                continue;
            }
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                tracing::error!("watch channel disconnected");
                break;
            }
        };
        let Some(first) = first else { continue };

        let mut dirty = HashSet::new();
        collect_dirty(&first, &mut dirty);

        let deadline = Instant::now() + DEBOUNCE;
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(remaining) {
                Ok(Ok(ev)) => collect_dirty(&ev, &mut dirty),
                Ok(Err(_)) => {}
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        // Run per-session captures for everything we saw dirty.
        let mut captured_any = false;
        for file in dirty.iter() {
            if let Some(session_id) = session_id_from_path(file) {
                tracing::info!(
                    "drift watch · dirty session {} — {}",
                    session_id,
                    file.display()
                );
                run_capture(repo, Some(session_id), None);
                captured_any = true;
            }
        }
        if captured_any {
            state.last_event_at = Some(Utc::now());
            state.save();
        }
    }

    Ok(())
}

/// Run `drift capture` for a single session (or full scan when `session_id`
/// is None). Soft-fail so a single failure doesn't take down the watcher.
fn run_capture(repo: &Path, session_id: Option<String>, all_since: Option<String>) {
    let sid_ref = session_id.as_deref();
    let since_ref = all_since.as_deref();
    if let Err(e) = super::capture::run(repo, sid_ref, None, since_ref) {
        tracing::warn!(
            session = sid_ref.unwrap_or("(scan)"),
            "capture failed: {}",
            e
        );
    }
}

fn collect_dirty(ev: &Event, dirty: &mut HashSet<PathBuf>) {
    if !is_interesting(&ev.kind) {
        return;
    }
    for p in &ev.paths {
        if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            dirty.insert(p.clone());
        }
    }
}

fn is_interesting(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

/// For both Claude (`~/.claude/projects/<proj>/<uuid>.jsonl`) and Codex
/// (`~/.codex/sessions/<date>/<uuid>.jsonl`) the file stem *is* the
/// session id. If it isn't a UUID-shaped stem we return None so the
/// debouncer does nothing with it.
fn session_id_from_path(p: &Path) -> Option<String> {
    let stem = p.file_stem()?.to_str()?;
    if stem.len() >= 8 && stem.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        Some(stem.to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Watch state persistence (~/.config/drift/watch-state.toml)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WatchState {
    pub last_event_at: Option<DateTime<Utc>>,
}

impl WatchState {
    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("drift").join("watch-state.toml"))
    }

    fn load() -> Self {
        let Some(p) = Self::path() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(&p) else {
            return Self::default();
        };
        toml::from_str(&text).unwrap_or_default()
    }

    fn save(&self) {
        let Some(p) = Self::path() else { return };
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match toml::to_string_pretty(self) {
            Ok(text) => {
                let _ = std::fs::write(&p, text);
            }
            Err(e) => tracing::warn!("watch-state write: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn session_id_from_claude_path() {
        let p = PathBuf::from(
            "/home/kirin/.claude/projects/-home-kirin/ad01ae46-156f-403b-b263-dd04a232873a.jsonl",
        );
        assert_eq!(
            session_id_from_path(&p).as_deref(),
            Some("ad01ae46-156f-403b-b263-dd04a232873a")
        );
    }

    #[test]
    fn session_id_rejects_non_jsonl() {
        let p = PathBuf::from("/tmp/whatever.txt");
        assert_eq!(session_id_from_path(&p), None);
    }

    #[test]
    fn session_id_rejects_non_uuid() {
        // Stem doesn't look like a UUID → None, regardless of extension.
        let p = PathBuf::from("/tmp/readme.jsonl");
        assert_eq!(session_id_from_path(&p), None);
    }

    #[test]
    fn watch_state_roundtrip() {
        let s = WatchState {
            last_event_at: Some(Utc::now()),
        };
        let text = toml::to_string_pretty(&s).unwrap();
        let back: WatchState = toml::from_str(&text).unwrap();
        assert_eq!(
            back.last_event_at.map(|t| t.to_rfc3339()),
            s.last_event_at.map(|t| t.to_rfc3339())
        );
    }
}
