use super::{open_store, sessions_dir};
use anyhow::{Context, Result};
use chrono::DateTime;
use drift_connectors::default_connectors;
use drift_core::attribution::commit_drafts;
use drift_core::compaction::factory::make_provider;
use drift_core::compaction::{summary_to_markdown, CompactionProvider};
use drift_core::config;
use std::io::{BufRead as _, Write as _};
use std::path::{Path, PathBuf};

pub fn run(
    repo: &Path,
    session_filter: Option<&str>,
    agent_filter: Option<&str>,
    all_since: Option<&str>,
) -> Result<()> {
    maybe_show_first_run_notice()?;
    super::init::run(repo).ok(); // ensure .prompts/ exists
    let store = open_store(repo)?;
    let connectors = default_connectors();
    let cfg = config::load(repo).unwrap_or_default();
    let provider: Box<dyn CompactionProvider> = select_provider(&cfg);
    eprintln!(
        "drift capture · provider={} (set [compaction].provider=\"anthropic\" + ANTHROPIC_API_KEY for live compaction)",
        provider.name()
    );
    let sessions_dir_p = sessions_dir(repo);
    std::fs::create_dir_all(&sessions_dir_p)?;

    let since_ts = all_since
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.naive_utc());

    let mut n_sessions = 0;
    let mut n_events = 0;

    for c in connectors {
        if let Some(a) = agent_filter {
            if c.agent_slug() != a {
                continue;
            }
        }
        let refs = c.discover()?;
        for r in refs {
            let ns = match c.parse(&r) {
                Ok(ns) => ns,
                Err(e) => {
                    tracing::warn!("skip {:?}: {}", r.path, e);
                    continue;
                }
            };
            if let Some(sid) = session_filter {
                if ns.session_id != sid {
                    continue;
                }
            }
            if let Some(ts) = since_ts {
                if ns.started_at.naive_utc() < ts {
                    continue;
                }
            }
            let drafts = c.extract_code_events(&ns)?;
            let events = commit_drafts(&store, drafts)?;
            n_events += events.len();

            // Compact. Soft-fail on a single session: log + skip so a huge
            // session that blows the context window doesn't abort the batch.
            let res = match provider.compact(&ns) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        session = %ns.session_id,
                        provider = provider.name(),
                        "compaction failed, skipping session: {}",
                        e
                    );
                    eprintln!(
                        "warning: skipping session {} ({}): {}",
                        ns.session_id,
                        provider.name(),
                        e
                    );
                    continue;
                }
            };
            if let Some(ref usage) = res.usage {
                store.insert_compaction_call(usage).with_context(|| {
                    format!("record compaction_call for session {}", ns.session_id)
                })?;
            }
            let summary = res.summary;
            let md = summary_to_markdown(&summary);
            let date = ns.started_at.format("%Y-%m-%d");
            let short = ns.session_id.chars().take(8).collect::<String>();
            let filename = format!("{}-{}-{}.md", date, c.agent_slug(), short);
            let path = sessions_dir_p.join(&filename);
            std::fs::write(&path, md).with_context(|| format!("write {}", path.display()))?;

            store.insert_session_meta(
                &ns.session_id,
                ns.agent_slug,
                ns.model.as_deref(),
                ns.working_dir
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .as_deref(),
                ns.started_at,
                ns.ended_at,
                ns.turns.len() as u32,
                ns.thinking_blocks,
                Some(&filename),
                &summary.summary,
            )?;
            n_sessions += 1;
        }
    }

    println!(
        "Captured {} session(s), wrote {} event(s) to {}",
        n_sessions,
        n_events,
        super::events_db_path(repo).display()
    );
    Ok(())
}

/// Resolve the compaction provider at capture-time using the v0.3 routing
/// factory. Supports `anthropic` (default), `openai`, `gemini`, `ollama`,
/// `mock`, and any user-named entry under `[compaction.providers.<name>]`
/// with `type = "openai_compatible"`.
///
/// Falls back to MockProvider when the chosen provider's API key env var is
/// unset, so CI / dry-runs still work.
fn select_provider(cfg: &drift_core::config::DriftConfig) -> Box<dyn CompactionProvider> {
    let routing = cfg.compaction.to_routing();
    match make_provider(&routing) {
        Ok((p, _mock_fallback)) => p,
        Err(e) => {
            eprintln!("drift capture · provider config error: {} — using mock", e);
            Box::new(drift_core::compaction::MockProvider)
        }
    }
}

// ---------------------------------------------------------------------------
// First-run privacy notice (v0.1.2)
// ---------------------------------------------------------------------------
//
// drift mirrors raw session content (including anything the user may have
// pasted into Claude/Codex chat) into .prompts/ and, by default, commits
// events.db to git. The first time a user runs `drift capture`, we print a
// one-shot reminder of this and wait for an interactive Enter so they cannot
// claim they were not told.
//
// Bypass for CI / scripted callers: set DRIFT_SKIP_FIRST_RUN=1. Doing so
// does NOT mark the notice as shown, because a CI bypass should not silence
// the warning for a later interactive run on the same workstation.

const NOTICE_BODY: &str = "\
drift capture · first-run notice
  drift mirrors your AI session content (including anything you
  pasted) into .prompts/. events.db is committed to git by default.
  See docs/SECURITY.md for the full story.

  Press Enter to continue, Ctrl-C to abort.";

pub(crate) fn maybe_show_first_run_notice() -> Result<()> {
    if std::env::var("DRIFT_SKIP_FIRST_RUN").is_ok_and(|v| !v.is_empty()) {
        return Ok(());
    }
    let path = first_run_state_path();
    if first_run_already_shown(path.as_deref()) {
        return Ok(());
    }
    let mut stderr = std::io::stderr();
    writeln!(stderr, "{}", NOTICE_BODY).ok();
    stderr.flush().ok();
    let mut buf = String::new();
    let stdin = std::io::stdin();
    stdin
        .lock()
        .read_line(&mut buf)
        .context("read stdin for first-run notice acknowledgement")?;
    if let Some(p) = path {
        write_first_run_state(&p)?;
    }
    Ok(())
}

pub(crate) fn first_run_state_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("drift").join("state.toml"))
}

pub(crate) fn first_run_already_shown(path: Option<&Path>) -> bool {
    let Some(p) = path else { return false };
    let Ok(text) = std::fs::read_to_string(p) else {
        return false;
    };
    text.lines()
        .any(|l| l.trim_start().starts_with("first_capture_shown") && l.contains("true"))
}

pub(crate) fn write_first_run_state(p: &Path) -> Result<()> {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(p, "first_capture_shown = true\n")
        .with_context(|| format!("write first-run state to {}", p.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_env_var_bypasses_notice() {
        // With DRIFT_SKIP_FIRST_RUN=1 set, maybe_show_first_run_notice returns
        // Ok(()) without ever blocking on stdin. Run inside a tempdir to
        // isolate state.toml.
        let tmp = tempfile::tempdir().unwrap();
        let prior = std::env::var("DRIFT_SKIP_FIRST_RUN").ok();
        let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("DRIFT_SKIP_FIRST_RUN", "1");
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let res = maybe_show_first_run_notice();
        // restore env regardless of outcome
        match prior {
            Some(v) => std::env::set_var("DRIFT_SKIP_FIRST_RUN", v),
            None => std::env::remove_var("DRIFT_SKIP_FIRST_RUN"),
        }
        match prior_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        assert!(res.is_ok(), "skip env should bypass cleanly: {:?}", res);
        // And we did NOT mark first_capture_shown — a later interactive run
        // on the same machine still gets the notice.
        let state = tmp.path().join("drift").join("state.toml");
        assert!(
            !state.exists(),
            "DRIFT_SKIP_FIRST_RUN must not write state.toml (would silence later interactive runs)"
        );
    }

    #[test]
    fn already_shown_predicate_reads_state() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("state.toml");
        assert!(!first_run_already_shown(Some(&p)));
        write_first_run_state(&p).unwrap();
        assert!(first_run_already_shown(Some(&p)));
        let body = std::fs::read_to_string(&p).unwrap();
        assert!(body.contains("first_capture_shown = true"));
    }

    #[test]
    fn write_state_creates_parent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("nested").join("dir").join("state.toml");
        write_first_run_state(&p).unwrap();
        assert!(p.exists());
    }
}
