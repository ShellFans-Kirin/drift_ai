//! `drift handoff` — package an in-progress task into a markdown brief
//! another agent can pick up cold.
//!
//! New in v0.2.0. The shape is:
//!
//!   1. [`build_handoff`] orchestrates four small collectors against the
//!      `EventStore` + working tree:
//!      - [`collect_sessions`]  filters sessions by scope (branch / since / single)
//!      - [`collect_events`]    pulls per-file `CodeEvent` rows
//!      - [`collect_rejected`]  pre-extracts rejected approaches
//!      - [`extract_file_snippets`] reads the *current* working-tree file
//!        and excerpts modified ranges with surrounding context
//!   2. An LLM second pass — `AnthropicProvider::complete` — produces the
//!      "What I'm working on" summary plus progress / decisions / next-steps
//!      in JSON, parsed into [`HandoffBrief`].
//!   3. [`render_brief`] is pure string formatting that turns a `HandoffBrief`
//!      + a [`TargetAgent`] into the final markdown.
//!
//! `events.db` schema is unchanged; this module is read-only against the
//! v0.1 store.

use crate::compaction::{AnthropicProvider, CompactionError, CompactionRes, LlmCompletion};
use crate::model::AgentSlug;
use crate::store::{EventStore, SessionRow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct HandoffOptions {
    pub repo: PathBuf,
    pub scope: HandoffScope,
    pub target_agent: TargetAgent,
}

#[derive(Debug, Clone)]
pub enum HandoffScope {
    Branch(String),
    Since(DateTime<Utc>),
    Session(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAgent {
    ClaudeCode,
    Codex,
    Generic,
}

impl TargetAgent {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude-code" | "claude_code" | "claude" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            "generic" | "any" => Some(Self::Generic),
            _ => None,
        }
    }
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSlim {
    pub session_id: String,
    pub agent_slug: AgentSlug,
    pub model: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub turn_count: u32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnippet {
    pub path: String,
    pub created: bool,
    pub additions: u32,
    pub deletions: u32,
    pub content_excerpt: String,
    pub excerpt_kind: ExcerptKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExcerptKind {
    Full,
    AroundModified,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedApproach {
    pub session_id: String,
    pub agent_slug: AgentSlug,
    pub turn_id: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandoffBrief {
    pub branch: Option<String>,
    pub repo_full_name: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub source_sessions: Vec<SessionSlim>,
    pub files_in_scope: Vec<FileSnippet>,
    pub rejected_approaches: Vec<RejectedApproach>,
    pub target_agent: String,
    pub llm_summary: String,
    pub progress_items: Vec<ProgressItem>,
    pub key_decisions: Vec<Decision>,
    pub open_questions: Vec<String>,
    pub next_steps: Vec<String>,
    pub usage: Option<LlmCompletion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressItem {
    pub status: ProgressStatus,
    pub item: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressStatus {
    Done,
    InProgress,
    NotStarted,
}

impl ProgressStatus {
    pub fn emoji(self) -> &'static str {
        match self {
            Self::Done => "✅",
            Self::InProgress => "⏳",
            Self::NotStarted => "⏸",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub text: String,
    pub citation: Option<String>,
}

// ---------------------------------------------------------------------------
// Orchestrator
// ---------------------------------------------------------------------------

pub fn build_handoff(
    store: &EventStore,
    provider: Option<&AnthropicProvider>,
    opts: &HandoffOptions,
) -> CompactionRes<HandoffBrief> {
    let sessions = collect_sessions(store, &opts.scope, &opts.repo)?;
    if sessions.is_empty() {
        return Err(CompactionError::Other(anyhow::anyhow!(
            "no sessions matched scope {:?} (repo={})",
            opts.scope,
            opts.repo.display()
        )));
    }

    let events_by_file = collect_events(store, &sessions)?;
    let rejected = collect_rejected(store, &sessions)?;
    let snippets = extract_file_snippets(&opts.repo, &events_by_file);

    let branch = if let HandoffScope::Branch(b) = &opts.scope {
        Some(b.clone())
    } else {
        None
    };
    let repo_full_name = detect_repo_full_name(&opts.repo);

    let llm = match provider {
        Some(p) => llm_second_pass(p, &sessions, &snippets, &rejected, branch.as_deref())?,
        None => deterministic_second_pass(&sessions, &snippets, &rejected),
    };

    Ok(HandoffBrief {
        branch,
        repo_full_name,
        generated_at: Utc::now(),
        source_sessions: sessions,
        files_in_scope: snippets,
        rejected_approaches: rejected,
        target_agent: opts.target_agent.as_slug().to_string(),
        llm_summary: llm.summary,
        progress_items: llm.progress,
        key_decisions: llm.decisions,
        open_questions: llm.open_questions,
        next_steps: llm.next_steps,
        usage: llm.usage,
    })
}

// ---------------------------------------------------------------------------
// 4 collectors
// ---------------------------------------------------------------------------

pub fn collect_sessions(
    store: &EventStore,
    scope: &HandoffScope,
    repo: &Path,
) -> CompactionRes<Vec<SessionSlim>> {
    match scope {
        HandoffScope::Session(sid) => {
            // Direct lookup. Use sessions_in_range with a wide window
            // and filter; cheaper than adding a new query method.
            let all = store
                .sessions_in_range(
                    Utc::now() - chrono::Duration::days(365 * 5),
                    Utc::now() + chrono::Duration::days(1),
                )
                .map_err(|e| CompactionError::Other(e))?;
            Ok(all
                .into_iter()
                .filter(|s| &s.session_id == sid)
                .map(slim_from_row)
                .collect())
        }
        HandoffScope::Since(ts) => {
            let rows = store
                .sessions_in_range(*ts, Utc::now() + chrono::Duration::days(1))
                .map_err(|e| CompactionError::Other(e))?;
            Ok(rows.into_iter().map(slim_from_row).collect())
        }
        HandoffScope::Branch(branch_name) => {
            // Best-effort: ask git for the earliest commit that is on this
            // branch but not on main; use that commit's authored date as the
            // lower bound. Fall back to last 14 days if git is unavailable
            // or the branch can't be resolved.
            let since = branch_earliest_timestamp(repo, branch_name)
                .unwrap_or_else(|| Utc::now() - chrono::Duration::days(14));
            let rows = store
                .sessions_in_range(since, Utc::now() + chrono::Duration::days(1))
                .map_err(|e| CompactionError::Other(e))?;
            Ok(rows.into_iter().map(slim_from_row).collect())
        }
    }
}

fn slim_from_row(r: SessionRow) -> SessionSlim {
    SessionSlim {
        session_id: r.session_id,
        agent_slug: r.agent_slug,
        model: r.model,
        started_at: r.started_at,
        ended_at: r.ended_at,
        turn_count: r.turn_count,
        summary: r.summary,
    }
}

/// Group `CodeEvent` rows by `file_path`. Reads from `events_for_session()`
/// per session — keeps `EventStore` API surface unchanged.
pub fn collect_events(
    store: &EventStore,
    sessions: &[SessionSlim],
) -> CompactionRes<BTreeMap<String, Vec<crate::CodeEvent>>> {
    let mut by_file: BTreeMap<String, Vec<crate::CodeEvent>> = BTreeMap::new();
    for s in sessions {
        let evs = store
            .events_for_session(&s.session_id)
            .map_err(|e| CompactionError::Other(e))?;
        for e in evs {
            by_file.entry(e.file_path.clone()).or_default().push(e);
        }
    }
    // Stable order: by timestamp.
    for v in by_file.values_mut() {
        v.sort_by_key(|e| e.timestamp);
    }
    Ok(by_file)
}

pub fn collect_rejected(
    store: &EventStore,
    sessions: &[SessionSlim],
) -> CompactionRes<Vec<RejectedApproach>> {
    let mut out = Vec::new();
    for s in sessions {
        let evs = store
            .events_for_session(&s.session_id)
            .map_err(|e| CompactionError::Other(e))?;
        for e in evs {
            if !e.rejected {
                continue;
            }
            // Pull a human note from metadata.detected_via or from the
            // diff_hunks first line; fall back to file_path.
            let note = e
                .metadata
                .get("error_message")
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| {
                    e.diff_hunks
                        .lines()
                        .next()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                })
                .unwrap_or_else(|| format!("rejected change to {}", e.file_path));
            out.push(RejectedApproach {
                session_id: s.session_id.clone(),
                agent_slug: s.agent_slug,
                turn_id: e.turn_id.clone(),
                note,
            });
        }
    }
    Ok(out)
}

const FULL_FILE_LINE_THRESHOLD: usize = 50;
const CONTEXT_LINES: usize = 5;

pub fn extract_file_snippets(
    repo: &Path,
    events_by_file: &BTreeMap<String, Vec<crate::CodeEvent>>,
) -> Vec<FileSnippet> {
    let mut out = Vec::new();
    for (path, evs) in events_by_file {
        let abs = repo.join(path);
        let created = evs.iter().any(|e| e.operation == crate::Operation::Create);

        // Sum +/- lines from diff_hunks across all events.
        let (mut adds, mut dels) = (0u32, 0u32);
        for e in evs {
            for line in e.diff_hunks.lines() {
                if line.starts_with("+++") || line.starts_with("---") {
                    continue;
                }
                if line.starts_with('+') {
                    adds += 1;
                } else if line.starts_with('-') {
                    dels += 1;
                }
            }
        }

        // Modified ranges, merged.
        let mut ranges: Vec<(u32, u32)> = evs
            .iter()
            .flat_map(|e| e.line_ranges_after.iter().copied())
            .collect();
        ranges.sort();
        let merged = merge_overlapping(&ranges);

        let (excerpt, kind) = match std::fs::read_to_string(&abs) {
            Ok(text) => render_excerpt(&text, &merged),
            Err(_) => (
                format!("(could not read working-tree file at {})", path),
                ExcerptKind::Missing,
            ),
        };

        out.push(FileSnippet {
            path: path.clone(),
            created,
            additions: adds,
            deletions: dels,
            content_excerpt: excerpt,
            excerpt_kind: kind,
        });
    }
    // Larger diffs first (more user-visible).
    out.sort_by_key(|s| std::cmp::Reverse(s.additions + s.deletions));
    out
}

fn merge_overlapping(ranges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for r in ranges {
        if let Some(last) = out.last_mut() {
            if r.0 <= last.1.saturating_add(1) {
                last.1 = last.1.max(r.1);
                continue;
            }
        }
        out.push(*r);
    }
    out
}

/// Returns (excerpt_text, kind). For files ≤ `FULL_FILE_LINE_THRESHOLD`
/// returns the full file; otherwise returns the modified ranges expanded
/// by [`CONTEXT_LINES`] and joined with `... <gap> ...` ellipses.
pub fn render_excerpt(text: &str, modified_ranges: &[(u32, u32)]) -> (String, ExcerptKind) {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= FULL_FILE_LINE_THRESHOLD {
        return (text.to_string(), ExcerptKind::Full);
    }
    if modified_ranges.is_empty() {
        let head = lines
            .iter()
            .take(40)
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        return (
            format!(
                "{}\n... ({} more lines)\n",
                head,
                lines.len().saturating_sub(40)
            ),
            ExcerptKind::AroundModified,
        );
    }

    // Expand each modified range by CONTEXT_LINES, clamp to file bounds,
    // then merge overlapping ranges in the expanded form.
    let total = lines.len() as u32;
    let mut expanded: Vec<(u32, u32)> = modified_ranges
        .iter()
        .map(|(a, b)| {
            let lo = a.saturating_sub(CONTEXT_LINES as u32).max(1);
            let hi = b.saturating_add(CONTEXT_LINES as u32).min(total);
            (lo, hi)
        })
        .collect();
    expanded.sort();
    let merged = merge_overlapping(&expanded);

    let mut out = String::new();
    for (i, (lo, hi)) in merged.iter().enumerate() {
        if i > 0 {
            out.push_str("...\n");
        }
        for ln in *lo..=*hi {
            let idx = (ln as usize).saturating_sub(1);
            if let Some(l) = lines.get(idx) {
                out.push_str(&format!("{:>4}  {}\n", ln, l));
            }
        }
    }
    if let Some((_, last_hi)) = merged.last() {
        if (*last_hi as usize) < lines.len() {
            out.push_str("...\n");
        }
    }
    (out, ExcerptKind::AroundModified)
}

fn detect_repo_full_name(repo: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // Common shapes: git@github.com:owner/name.git / https://github.com/owner/name(.git)?
    let trimmed = url
        .trim_end_matches(".git")
        .replace("git@github.com:", "")
        .replace("https://github.com/", "");
    if trimmed.contains('/') && !trimmed.is_empty() {
        Some(trimmed)
    } else {
        None
    }
}

fn branch_earliest_timestamp(repo: &Path, branch: &str) -> Option<DateTime<Utc>> {
    // git log <branch> --not main --format=%aI | tail -1
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", branch, "--not", "main", "--format=%aI"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last()?.trim();
    DateTime::parse_from_rfc3339(last)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

// ---------------------------------------------------------------------------
// LLM second pass
// ---------------------------------------------------------------------------

const HANDOFF_TEMPLATE: &str = include_str!("../templates/handoff.md");
const HANDOFF_SYSTEM_PROMPT: &str = "\
You produce concise, structured handoff briefs for AI-assisted coding tasks. \
Output ONLY a single JSON object as specified. No prose, no fenced code block.";

struct LlmOutput {
    summary: String,
    progress: Vec<ProgressItem>,
    decisions: Vec<Decision>,
    open_questions: Vec<String>,
    next_steps: Vec<String>,
    usage: Option<LlmCompletion>,
}

fn llm_second_pass(
    provider: &AnthropicProvider,
    sessions: &[SessionSlim],
    snippets: &[FileSnippet],
    rejected: &[RejectedApproach],
    branch: Option<&str>,
) -> CompactionRes<LlmOutput> {
    let prompt = build_handoff_prompt(sessions, snippets, rejected, branch);
    let completion = provider.complete(HANDOFF_SYSTEM_PROMPT, &prompt)?;
    let parsed = parse_llm_json(&completion.text).unwrap_or_else(|err| {
        // Don't fail the whole handoff if the LLM returned non-JSON; degrade
        // gracefully with a deterministic fallback containing the raw text
        // as the summary so the user at least has something.
        tracing::warn!("LLM returned non-JSON handoff payload: {}", err);
        LlmJson {
            summary: completion.text.clone(),
            progress: vec![],
            key_decisions: vec![],
            open_questions: vec![],
            next_steps: vec![],
        }
    });
    Ok(LlmOutput {
        summary: parsed.summary,
        progress: parsed.progress,
        decisions: parsed.key_decisions,
        open_questions: parsed.open_questions,
        next_steps: parsed.next_steps,
        usage: Some(completion),
    })
}

fn deterministic_second_pass(
    sessions: &[SessionSlim],
    snippets: &[FileSnippet],
    _rejected: &[RejectedApproach],
) -> LlmOutput {
    let summary = if sessions.is_empty() {
        "(no sessions in scope)".to_string()
    } else {
        let agents: Vec<&str> = sessions
            .iter()
            .map(|s| s.agent_slug.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let total_turns: u32 = sessions.iter().map(|s| s.turn_count).sum();
        format!(
            "[MOCK] {} session(s) across {} agent(s) — {} total turns. {} file(s) in scope.",
            sessions.len(),
            agents.len(),
            total_turns,
            snippets.len()
        )
    };
    LlmOutput {
        summary,
        progress: snippets
            .iter()
            .take(5)
            .map(|s| ProgressItem {
                status: if s.created {
                    ProgressStatus::Done
                } else {
                    ProgressStatus::InProgress
                },
                item: format!("{} (+{} -{})", s.path, s.additions, s.deletions),
            })
            .collect(),
        decisions: vec![],
        open_questions: vec![],
        next_steps: vec!["[MOCK] resume work where the last session left off".to_string()],
        usage: None,
    }
}

fn build_handoff_prompt(
    sessions: &[SessionSlim],
    snippets: &[FileSnippet],
    rejected: &[RejectedApproach],
    branch: Option<&str>,
) -> String {
    let session_metas = sessions
        .iter()
        .map(|s| {
            format!(
                "- {} · agent={} · turns={} · {} → {}\n  prior summary: {}",
                short(&s.session_id),
                s.agent_slug.as_str(),
                s.turn_count,
                s.started_at.format("%Y-%m-%d %H:%M"),
                s.ended_at.format("%H:%M"),
                if s.summary.len() > 200 {
                    format!("{}…", &s.summary[..200])
                } else {
                    s.summary.clone()
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let file_summaries = snippets
        .iter()
        .map(|s| {
            format!(
                "- `{}` ({}, +{} -{})\n```\n{}\n```",
                s.path,
                if s.created { "created" } else { "modified" },
                s.additions,
                s.deletions,
                truncate(&s.content_excerpt, 1500)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let rejected_block = if rejected.is_empty() {
        "(none)".to_string()
    } else {
        rejected
            .iter()
            .take(8)
            .map(|r| {
                format!(
                    "- {}/{} · turn {} · {}",
                    r.agent_slug.as_str(),
                    short(&r.session_id),
                    r.turn_id.as_deref().unwrap_or("?"),
                    truncate(&r.note, 200)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Recent turn excerpts: prior session summaries are the cheapest
    // approximation, since we don't pull raw transcripts here. The LLM
    // gets these as additional context.
    let recent = sessions
        .iter()
        .rev()
        .take(3)
        .map(|s| {
            format!(
                "### session {} ({})\n{}",
                short(&s.session_id),
                s.agent_slug.as_str(),
                truncate(&s.summary, 800)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    HANDOFF_TEMPLATE
        .replace(
            "{{branch}}",
            branch.unwrap_or("(no branch / time-range scope)"),
        )
        .replace("{{session_metas}}", &session_metas)
        .replace("{{file_summaries}}", &file_summaries)
        .replace("{{rejected_approaches}}", &rejected_block)
        .replace("{{recent_turn_excerpts}}", &recent)
}

#[derive(Debug, Default, Deserialize)]
struct LlmJson {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    progress: Vec<ProgressItem>,
    #[serde(default)]
    key_decisions: Vec<Decision>,
    #[serde(default)]
    open_questions: Vec<String>,
    #[serde(default)]
    next_steps: Vec<String>,
}

fn parse_llm_json(s: &str) -> Result<LlmJson, String> {
    // Strip a possible code-fence wrapper if the model ignored instructions.
    let trimmed = s.trim();
    let inner = if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.trim_end_matches("```").trim()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.trim_end_matches("```").trim()
    } else {
        trimmed
    };
    // Find the outermost JSON object.
    let start = inner
        .find('{')
        .ok_or_else(|| "no { in LLM output".to_string())?;
    let end = inner
        .rfind('}')
        .ok_or_else(|| "no } in LLM output".to_string())?;
    if end < start {
        return Err("malformed JSON braces".into());
    }
    let candidate = &inner[start..=end];
    serde_json::from_str(candidate).map_err(|e| format!("serde_json: {}", e))
}

// ---------------------------------------------------------------------------
// Renderer (pure)
// ---------------------------------------------------------------------------

/// Render a [`HandoffBrief`] as a markdown string. Pure (no I/O).
pub fn render_brief(brief: &HandoffBrief, target: TargetAgent) -> String {
    let mut s = String::with_capacity(4096);
    let branch_label = brief.branch.as_deref().unwrap_or("(no branch)");
    let _ = std::fmt::Write::write_fmt(
        &mut s,
        format_args!("# Handoff Brief — `{}`\n\n", branch_label),
    );

    // Header table
    let agent_summary = summarise_agents(&brief.source_sessions);
    let total_turns: u32 = brief.source_sessions.iter().map(|s| s.turn_count).sum();
    let total_addsdel: (u32, u32) = brief
        .files_in_scope
        .iter()
        .fold((0, 0), |a, f| (a.0 + f.additions, a.1 + f.deletions));
    s.push_str("| Field      | Value |\n");
    s.push_str("|------------|-------|\n");
    s.push_str(&format!(
        "| From       | {} ({} sessions, {} turns) |\n",
        agent_summary,
        brief.source_sessions.len(),
        total_turns
    ));
    s.push_str(&format!("| To         | {} |\n", target.as_slug()));
    s.push_str(&format!(
        "| Generated  | {} |\n",
        brief.generated_at.format("%Y-%m-%d %H:%M UTC")
    ));
    if let Some(repo) = &brief.repo_full_name {
        s.push_str(&format!("| Repo       | {} @ {} |\n", repo, branch_label));
    }
    s.push_str(&format!(
        "| Branch dif | +{} / -{} across {} files |\n\n",
        total_addsdel.0,
        total_addsdel.1,
        brief.files_in_scope.len()
    ));

    // What I'm working on
    s.push_str("## What I'm working on\n\n");
    s.push_str(brief.llm_summary.trim());
    s.push_str("\n\n");

    // Progress
    s.push_str("## Progress so far\n\n");
    if brief.progress_items.is_empty() {
        s.push_str("_(no progress items recorded)_\n\n");
    } else {
        for p in &brief.progress_items {
            s.push_str(&format!("- {} {}\n", p.status.emoji(), p.item));
        }
        s.push('\n');
    }

    // Files in scope
    s.push_str("## Files in scope\n\n");
    if brief.files_in_scope.is_empty() {
        s.push_str("_(no file changes)_\n\n");
    } else {
        for f in &brief.files_in_scope {
            let kind = if f.created { "created" } else { "modified" };
            s.push_str(&format!(
                "### `{}` ({}, +{} / -{})\n\n",
                f.path, kind, f.additions, f.deletions
            ));
            s.push_str("```\n");
            s.push_str(f.content_excerpt.trim_end());
            s.push_str("\n```\n\n");
        }
    }

    // Key decisions
    s.push_str("## Key decisions made\n\n");
    if brief.key_decisions.is_empty() {
        s.push_str("_(none recorded)_\n\n");
    } else {
        for d in &brief.key_decisions {
            match &d.citation {
                Some(c) => s.push_str(&format!("- **{}** _({})_\n", d.text, c)),
                None => s.push_str(&format!("- **{}**\n", d.text)),
            }
        }
        s.push('\n');
    }

    // Rejected approaches
    s.push_str("## Approaches tried but rejected\n\n");
    if brief.rejected_approaches.is_empty() {
        s.push_str("_(none)_\n\n");
    } else {
        for r in brief.rejected_approaches.iter().take(8) {
            s.push_str(&format!(
                "- _{}/{}_ · turn {} · {}\n",
                r.agent_slug.as_str(),
                short(&r.session_id),
                r.turn_id.as_deref().unwrap_or("?"),
                r.note
            ));
        }
        s.push('\n');
    }

    // Open questions
    s.push_str("## Open questions / blockers\n\n");
    if brief.open_questions.is_empty() {
        s.push_str("_(none)_\n\n");
    } else {
        for (i, q) in brief.open_questions.iter().enumerate() {
            s.push_str(&format!("{}. {}\n", i + 1, q));
        }
        s.push('\n');
    }

    // Next steps
    s.push_str("## Next steps (suggested)\n\n");
    if brief.next_steps.is_empty() {
        s.push_str("_(none)_\n\n");
    } else {
        for (i, n) in brief.next_steps.iter().enumerate() {
            s.push_str(&format!("{}. {}\n", i + 1, n));
        }
        s.push('\n');
    }

    s.push_str("---\n\n");
    s.push_str(&format!(
        "*Generated by `drift handoff` against {} on {}.*\n",
        brief
            .repo_full_name
            .clone()
            .unwrap_or_else(|| "(local repo)".to_string()),
        brief.generated_at.format("%Y-%m-%d %H:%M UTC")
    ));

    // Per-target footer
    match target {
        TargetAgent::ClaudeCode => {
            s.push_str(
                "\n## How to continue (paste this to claude-code)\n\n\
                 > I'm picking up an in-progress task documented in the handoff brief above. \
                 Read it end-to-end, then resume from \"Next steps #1\". \
                 The codebase is at the current working directory",
            );
            if let Some(b) = &brief.branch {
                s.push_str(&format!(", branch `{}`", b));
            }
            s.push_str(
                ". Don't revisit decisions in \"Key decisions made\"; treat them as settled.\n",
            );
        }
        TargetAgent::Codex => {
            s.push_str(
                "\n## How to continue (paste this to codex)\n\n\
                 > Resume the task documented in the handoff brief above. \
                 Read it end-to-end, then start with \"Next steps\" item 1. \
                 Working directory has the relevant codebase",
            );
            if let Some(b) = &brief.branch {
                s.push_str(&format!(" on branch `{}`", b));
            }
            s.push_str(". Decisions in \"Key decisions made\" are settled — do not revisit.\n");
        }
        TargetAgent::Generic => {
            // Pure brief, no per-agent footer.
        }
    }

    s
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn summarise_agents(sessions: &[SessionSlim]) -> String {
    let mut counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    for s in sessions {
        *counts.entry(s.agent_slug.as_str()).or_default() += 1;
    }
    counts
        .iter()
        .map(|(a, n)| format!("{} ×{}", a, n))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn short(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if i >= n {
                break;
            }
            out.push(c);
        }
        out.push('…');
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_agent_parse_aliases() {
        assert_eq!(
            TargetAgent::parse("claude-code"),
            Some(TargetAgent::ClaudeCode)
        );
        assert_eq!(
            TargetAgent::parse("Claude-Code"),
            Some(TargetAgent::ClaudeCode)
        );
        assert_eq!(
            TargetAgent::parse("claude_code"),
            Some(TargetAgent::ClaudeCode)
        );
        assert_eq!(TargetAgent::parse("claude"), Some(TargetAgent::ClaudeCode));
        assert_eq!(TargetAgent::parse("codex"), Some(TargetAgent::Codex));
        assert_eq!(TargetAgent::parse("generic"), Some(TargetAgent::Generic));
        assert_eq!(TargetAgent::parse("any"), Some(TargetAgent::Generic));
        assert_eq!(TargetAgent::parse("nope"), None);
    }

    #[test]
    fn merge_overlapping_basic() {
        assert_eq!(
            merge_overlapping(&[(1, 3), (2, 5), (8, 9)]),
            vec![(1, 5), (8, 9)]
        );
        assert_eq!(merge_overlapping(&[(1, 3), (4, 5)]), vec![(1, 5)]);
        assert_eq!(
            merge_overlapping(&[(1, 3), (10, 12)]),
            vec![(1, 3), (10, 12)]
        );
        assert!(merge_overlapping(&[]).is_empty());
    }

    #[test]
    fn render_excerpt_short_file_returns_full() {
        let text = (1..=10)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let (out, kind) = render_excerpt(&text, &[(1, 3)]);
        assert_eq!(kind, ExcerptKind::Full);
        assert!(out.contains("line1"));
        assert!(out.contains("line10"));
    }

    #[test]
    fn render_excerpt_long_file_extracts_around_modified() {
        let text = (1..=200)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        // Modified ranges around lines 50-52 and 150-151
        let (out, kind) = render_excerpt(&text, &[(50, 52), (150, 151)]);
        assert_eq!(kind, ExcerptKind::AroundModified);
        // Should include line 50 (in first window with 5 ctx → 45..=57)
        assert!(out.contains("line50"));
        assert!(out.contains("line150"));
        // Should NOT include line 100 (between the two ranges)
        assert!(!out.contains("line100"));
        // Should have a separating "..." between the two ranges.
        assert!(out.contains("..."));
    }

    #[test]
    fn render_excerpt_long_file_no_modified_ranges_falls_back_to_head() {
        let text = (1..=200)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let (out, kind) = render_excerpt(&text, &[]);
        assert_eq!(kind, ExcerptKind::AroundModified);
        assert!(out.contains("line1"));
        assert!(out.contains("more lines"));
    }

    #[test]
    fn parse_llm_json_strict_object() {
        let s = r#"{"summary":"hi","progress":[],"key_decisions":[],"open_questions":[],"next_steps":["s1","s2"]}"#;
        let p = parse_llm_json(s).unwrap();
        assert_eq!(p.summary, "hi");
        assert_eq!(p.next_steps.len(), 2);
    }

    #[test]
    fn parse_llm_json_strips_code_fence() {
        let s = "```json\n{\"summary\":\"hi\",\"progress\":[],\"key_decisions\":[],\"open_questions\":[],\"next_steps\":[]}\n```";
        let p = parse_llm_json(s).unwrap();
        assert_eq!(p.summary, "hi");
    }

    #[test]
    fn parse_llm_json_extracts_inner_object_with_surrounding_prose() {
        let s = "Sure, here is the JSON: {\"summary\":\"x\",\"progress\":[],\"key_decisions\":[],\"open_questions\":[],\"next_steps\":[]} hope that helps";
        let p = parse_llm_json(s).unwrap();
        assert_eq!(p.summary, "x");
    }

    #[test]
    fn parse_llm_json_rejects_non_json() {
        assert!(parse_llm_json("definitely not json").is_err());
    }

    fn make_brief() -> HandoffBrief {
        HandoffBrief {
            branch: Some("feature/oauth".into()),
            repo_full_name: Some("owner/repo".into()),
            generated_at: chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 4, 25, 15, 30, 0).unwrap(),
            source_sessions: vec![SessionSlim {
                session_id: "abc12345-xxx".into(),
                agent_slug: AgentSlug::Codex,
                model: Some("claude-haiku-4-5".into()),
                started_at: chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 4, 25, 14, 0, 0)
                    .unwrap(),
                ended_at: chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 4, 25, 14, 30, 0).unwrap(),
                turn_count: 12,
                summary: "prior summary".into(),
            }],
            files_in_scope: vec![FileSnippet {
                path: "src/auth/login.ts".into(),
                created: false,
                additions: 47,
                deletions: 3,
                content_excerpt: "// snippet".into(),
                excerpt_kind: ExcerptKind::AroundModified,
            }],
            rejected_approaches: vec![],
            target_agent: "claude-code".into(),
            llm_summary: "Doing OAuth.".into(),
            progress_items: vec![ProgressItem {
                status: ProgressStatus::Done,
                item: "Wired NextAuth".into(),
            }],
            key_decisions: vec![Decision {
                text: "Chose NextAuth.".into(),
                citation: Some("codex abc12345, turn 4".into()),
            }],
            open_questions: vec!["Token refresh edge case?".into()],
            next_steps: vec!["Resume callback.ts:L40-65".into()],
            usage: None,
        }
    }

    #[test]
    fn render_brief_to_claude_code_includes_continue_footer() {
        let b = make_brief();
        let md = render_brief(&b, TargetAgent::ClaudeCode);
        assert!(md.contains("# Handoff Brief — `feature/oauth`"));
        assert!(md.contains("## How to continue (paste this to claude-code)"));
        assert!(md.contains("Doing OAuth."));
        assert!(md.contains("`src/auth/login.ts`"));
        assert!(md.contains("Wired NextAuth"));
        assert!(md.contains("Chose NextAuth."));
        assert!(md.contains("Token refresh edge case?"));
        assert!(md.contains("Resume callback.ts:L40-65"));
    }

    #[test]
    fn render_brief_to_codex_uses_codex_footer() {
        let md = render_brief(&make_brief(), TargetAgent::Codex);
        assert!(md.contains("## How to continue (paste this to codex)"));
        assert!(!md.contains("paste this to claude-code"));
    }

    #[test]
    fn render_brief_to_generic_omits_footer() {
        let md = render_brief(&make_brief(), TargetAgent::Generic);
        assert!(!md.contains("## How to continue"));
        // Body should still be there.
        assert!(md.contains("# Handoff Brief"));
        assert!(md.contains("Doing OAuth."));
    }

    #[test]
    fn render_brief_table_header_has_branch_and_repo() {
        let md = render_brief(&make_brief(), TargetAgent::Generic);
        assert!(md.contains("| Repo       | owner/repo @ feature/oauth |"));
        assert!(md.contains("| Branch dif | +47 / -3 across 1 files |"));
    }

    #[test]
    fn truncate_emits_ellipsis_only_when_needed() {
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abcdefgh", 4), "abcd…");
    }

    #[test]
    fn deterministic_second_pass_produces_mock_summary() {
        let sessions = vec![SessionSlim {
            session_id: "x".into(),
            agent_slug: AgentSlug::Codex,
            model: None,
            started_at: Utc::now(),
            ended_at: Utc::now(),
            turn_count: 3,
            summary: "".into(),
        }];
        let snips = vec![];
        let r = vec![];
        let out = deterministic_second_pass(&sessions, &snips, &r);
        assert!(out.summary.starts_with("[MOCK]"));
        assert!(out.usage.is_none());
    }
}
