//! `drift handoff` — package an in-progress task into a markdown brief
//! that another agent can pick up cold.

use super::open_store;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use drift_core::config;
use drift_core::{
    build_handoff, make_completer, render_brief, HandoffOptions, HandoffScope, LlmCompleter,
    TargetAgent,
};
use std::io::{IsTerminal, Write as _};
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
pub fn run(
    repo: &Path,
    branch: Option<&str>,
    since: Option<&str>,
    session: Option<&str>,
    to: Option<&str>,
    output: Option<&Path>,
    print: bool,
) -> Result<()> {
    if print && output.is_some() {
        return Err(anyhow!(
            "--print and --output are mutually exclusive (use one or the other)"
        ));
    }

    let scope = resolve_scope(branch, since, session)?;
    let target = match to {
        Some(s) => TargetAgent::parse(s).ok_or_else(|| {
            anyhow!(
                "--to expects one of claude-code | codex | generic, got `{}`",
                s
            )
        })?,
        None => TargetAgent::ClaudeCode,
    };

    let cfg = config::load(repo).unwrap_or_default();
    // Build an LlmCompleter from config — anthropic by default, with optional
    // [handoff.providers.<name>] entries for openai / gemini / ollama / openai_compatible.
    let routing = cfg.handoff.to_routing();
    let (completer, mock_fallback) = match make_completer(&routing) {
        Ok(pair) => pair,
        Err(e) => {
            return Err(anyhow!("handoff: failed to build provider: {}", e));
        }
    };
    if mock_fallback {
        eprintln!(
            "drift handoff · provider `{}` unavailable (env var unset?) — falling back to deterministic mock summary",
            routing.provider.as_deref().unwrap_or("anthropic")
        );
    }
    let completer_name = completer
        .as_ref()
        .map(|p| <dyn LlmCompleter>::name(p.as_ref()))
        .unwrap_or("mock");

    let opts = HandoffOptions {
        repo: repo.to_path_buf(),
        scope,
        target_agent: target,
    };

    progress(true, "⚡ scanning .prompts/events.db");
    let store = open_store(repo)?;

    progress(true, "⚡ extracting file snippets and rejected approaches");
    progress(true, &format!("⚡ compacting brief via {}", completer_name));

    let brief = build_handoff(&store, completer.as_deref(), &opts).context("build_handoff")?;

    let body = render_brief(&brief, target);

    if print {
        // Direct to stdout, no progress noise on the user's pipe sink.
        std::io::stdout()
            .write_all(body.as_bytes())
            .context("write handoff brief to stdout")?;
        return Ok(());
    }

    let out_path = match output {
        Some(p) => p.to_path_buf(),
        None => default_output_path(repo, &brief, target),
    };
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(&out_path, &body).with_context(|| format!("write {}", out_path.display()))?;

    progress(true, &format!("✅ written to {}", out_path.display()));

    if let Some(usage) = brief.usage.as_ref() {
        eprintln!(
            "  · model={} · in={} out={} · cost=${:.4}",
            usage.model, usage.input_tokens, usage.output_tokens, usage.cost_usd
        );
    }

    print_next_steps(target, &out_path);
    Ok(())
}

fn resolve_scope(
    branch: Option<&str>,
    since: Option<&str>,
    session: Option<&str>,
) -> Result<HandoffScope> {
    let n = branch.is_some() as u8 + since.is_some() as u8 + session.is_some() as u8;
    if n == 0 {
        return Err(anyhow!(
            "drift handoff needs one of --branch <name>, --since <iso>, or --session <id>"
        ));
    }
    if n > 1 {
        return Err(anyhow!(
            "--branch / --since / --session are mutually exclusive (got {} of them)",
            n
        ));
    }
    if let Some(b) = branch {
        return Ok(HandoffScope::Branch(b.to_string()));
    }
    if let Some(s) = since {
        let dt = DateTime::parse_from_rfc3339(s)
            .with_context(|| format!("--since expects RFC3339 timestamp, got `{}`", s))?
            .with_timezone(&Utc);
        return Ok(HandoffScope::Since(dt));
    }
    if let Some(sid) = session {
        return Ok(HandoffScope::Session(sid.to_string()));
    }
    unreachable!()
}

fn default_output_path(
    repo: &Path,
    brief: &drift_core::HandoffBrief,
    target: TargetAgent,
) -> PathBuf {
    let dir = repo.join(".prompts").join("handoffs");
    let ts = brief.generated_at.format("%Y-%m-%d-%H%M");
    let branch_slug = brief
        .branch
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| "scope".to_string());
    let name = format!("{}-{}-to-{}.md", ts, branch_slug, target.as_slug());
    dir.join(name)
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn progress(enabled: bool, msg: &str) {
    if !enabled {
        return;
    }
    if std::env::var("DRIFT_HANDOFF_QUIET").is_ok() {
        return;
    }
    let _ = writeln!(std::io::stderr(), "{}", msg);
}

fn print_next_steps(target: TargetAgent, out_path: &Path) {
    let path_str = out_path.display();
    match target {
        TargetAgent::ClaudeCode => {
            eprintln!();
            eprintln!("next:");
            eprintln!("  claude");
            eprintln!("  # then paste:");
            eprintln!("  \"I'm continuing this task. Read the handoff brief and resume from 'Next steps' #1:\"");
            eprintln!("  \"$(cat {})\"", path_str);
        }
        TargetAgent::Codex => {
            eprintln!();
            eprintln!("next:");
            eprintln!("  codex");
            eprintln!("  # then paste:");
            eprintln!("  \"Resume the task documented in this brief. Start at 'Next steps' #1.\"");
            eprintln!("  \"$(cat {})\"", path_str);
        }
        TargetAgent::Generic => {
            eprintln!();
            eprintln!("next: paste {} into your target agent.", path_str);
        }
    }
    // Suppress colour in non-interactive contexts (no-op for now since we
    // don't emit colour codes; this is a placeholder for future styling).
    let _ = std::io::stderr().is_terminal();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_scope_branch() {
        match resolve_scope(Some("feature/x"), None, None).unwrap() {
            HandoffScope::Branch(b) => assert_eq!(b, "feature/x"),
            _ => panic!("wrong scope"),
        }
    }

    #[test]
    fn resolve_scope_session() {
        match resolve_scope(None, None, Some("abc")).unwrap() {
            HandoffScope::Session(s) => assert_eq!(s, "abc"),
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_scope_since_iso() {
        match resolve_scope(None, Some("2026-04-25T12:00:00Z"), None).unwrap() {
            HandoffScope::Since(_) => {}
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_scope_rejects_bad_iso() {
        assert!(resolve_scope(None, Some("yesterday"), None).is_err());
    }

    #[test]
    fn resolve_scope_rejects_zero() {
        assert!(resolve_scope(None, None, None).is_err());
    }

    #[test]
    fn resolve_scope_rejects_more_than_one() {
        assert!(resolve_scope(Some("a"), Some("2026-04-25T00:00:00Z"), None).is_err());
        assert!(resolve_scope(Some("a"), None, Some("b")).is_err());
        assert!(resolve_scope(None, Some("2026-04-25T00:00:00Z"), Some("b")).is_err());
    }

    #[test]
    fn slugify_cleans_branch_separators() {
        assert_eq!(slugify("feature/oauth"), "feature-oauth");
        assert_eq!(slugify("user/joe@dev/wip"), "user-joe-dev-wip");
        assert_eq!(slugify("plain-branch"), "plain-branch");
    }
}
