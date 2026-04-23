# Show HN draft — drift_ai

**Title**: Show HN: Drift AI — git blame for the AI coding era (Rust, local-first, Claude Code + Codex on day one)

**URL**: https://github.com/ShellFans-Kirin/drift_ai

**Body**:

Hi HN — I got tired of `git blame` returning `someone committed 400
lines last Tuesday` when I knew full well that commit was 80% Claude
Code, 15% Codex, and a few manual touch-ups at the end. Commits are
the wrong granularity for AI-era source control.

Drift AI captures each AI coding session locally (Claude Code from
`~/.claude/projects/`, Codex from `~/.codex/sessions/`), compacts it
into a ~1 KB markdown note, writes it to `.prompts/` in the repo, and
— the actual novel bit — builds a line-level attribution layer so
`drift blame path/to/file.rs --line 42` returns the full multi-agent
+ human-edit timeline for that exact line, complete with the
originating prompt and any rejected alternatives from the same
session.

Technical choices worth calling out:

- **Local-first, zero cloud.** Everything lives in the repo: compacted
  sessions under `.prompts/sessions/`, events in SQLite under
  `.prompts/events.db` (configurable — set `db_in_git = false` for
  privacy-sensitive repos), binding via `refs/notes/drift`.
- **Cross-agent abstraction proven on two materially different
  schemas.** Claude Code emits `tool_use` with `Write` / `Edit` /
  `MultiEdit`; Codex emits `custom_tool_call` with an `apply_patch`
  envelope (`*** Begin Patch / Add File / Update File / Move File`).
  Building the `SessionConnector` trait against both from day one
  keeps the attribution layer honest.
- **Honest about what we can and can't claim.** The `human` slug means
  "no AI session produced this change", not "this human wrote it". SHA
  drift is the only reliable signal we have — we don't fake authorship.
- **MCP-first.** `drift mcp` is a stdio JSON-RPC server exposing five
  read-only tools (`drift_blame`, `drift_trace`, `drift_rejected`,
  `drift_log`, `drift_show_event`). Claude Code and Codex plugin
  skeletons in `plugins/` register this same server — logic lives once.

Limits I'm open about in the README: the Anthropic HTTP call is
stubbed (Mock provider is the default, wire-up is a ~30-line change);
human-edit detection is a SHA ladder with intentionally no authorship
inference; `drift watch` is debounced polling rather than fully
event-driven.

Written in Rust (edition 2021, MSRV 1.85). Pre-compiled binaries for
Linux + macOS on GitHub Releases. Apache-2.0.

Looking for: folks to beat on the data model for the four cases I've
tried hardest to honour — **multi-origin** / **human edit** /
**rejected suggestion** / **rename lineage** — and tell me where it
falls over.
