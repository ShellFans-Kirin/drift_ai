# Changelog

All notable changes to drift_ai are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [SemVer](https://semver.org/spec/v2.0.0.html).

## [0.1.2] — 2026-04-25

Documentation + messaging patch on top of v0.1.1. The compaction /
attribution / MCP code paths are unchanged from v0.1.1; the only
behavioural change is a one-shot privacy notice the first time a
user runs `drift capture`.

### Added
- **`docs/SECURITY.md`** — threat model, current limitations, available
  mitigations (db_in_git toggle, manual review, gitleaks/trufflehog
  pre-commit), v0.2 roadmap (regex redaction pass, interactive review
  mode, `drift redact` post-hoc scrub), security-disclosure channel.
- **README `## Privacy & secrets` section** — explicit, non-soft-sold
  disclosure that `drift capture` mirrors session content into
  `.prompts/` and commits `events.db` to git by default.
- **`drift capture` first-run notice** — the first invocation prints a
  one-paragraph reminder of the privacy posture and waits on stdin.
  Bypass via `DRIFT_SKIP_FIRST_RUN=1` (CI-friendly). State is recorded
  at `~/.config/drift/state.toml::first_capture_shown`.
- **`docs/COMPARISON.md`** — functional comparison vs Cursor /
  Copilot chat / Cody / `git blame`. Linked from README.
- **README pain-statement opener** — one paragraph ("47 prompts to
  Claude + 3 Codex fills + 12 manual edits ...") above the
  technical description.
- **README `## About` section** — explicit declaration that drift is
  independent and not affiliated with Anthropic, OpenAI, or any other
  agent vendor.
- **README badges**: crates.io version + CI status (capped at two).
- **Provider-switching example** in `## Configuration` that names the
  v0.2 plan (ollama / vllm / openai-compatible).

### Tests
- `tests/first_run_notice.rs` covers `DRIFT_SKIP_FIRST_RUN=1` bypass
  and the state-file persistence path.

### Known limitations carried from v0.1.1
- Drift still does not actively redact secrets — that is v0.2 work.
- Cost pricing table is hardcoded; verify against Anthropic's public
  pricing before billing reports.

## [0.1.1] — 2026-04-23

### Added
- **Live Anthropic compaction.** `AnthropicProvider` now talks to
  `POST /v1/messages?stream=true` for real, consumes the SSE stream,
  echoes content deltas to stderr during CLI runs, and parses the
  `usage` block at `message_stop` for billing.
- **Typed compaction errors** (`CompactionError`): `AuthInvalid`,
  `RateLimited { retry_after }`, `ModelNotFound`, `ContextTooLong`,
  `TransientNetwork`, `Stream`, `Other`. Each variant maps to a
  distinct operator-visible message at the CLI.
- **Model switching via config**: `[compaction].model` accepts
  `claude-opus-4-7` (default), `claude-sonnet-4-6`, `claude-haiku-4-5`.
- **Retry policy**: 5× for 429 honouring `Retry-After`; 4× for 5xx with
  exponential backoff (1s → 2s → 4s → 8s); instant-fail for 401/404.
- **Context-window truncation**: char-based token estimate + 80%
  threshold; Strategy 1 keeps head(8) + tail(8) turns and elides the
  middle with an explicit marker.
- **`compaction_calls` table** (SQLite migration v2): per-call
  input/output/cache-creation/cache-read tokens and computed USD cost.
- **`drift cost`** CLI: `--since <iso>` / `--until <iso>` /
  `--model <name>` / `--by model|session|date`.
- **`drift watch` is event-driven**: `notify`-backed
  (FSEvents/inotify/ReadDirectoryChangesW), 200ms debounce,
  per-session capture by filename-derived session_id, state persisted
  to `~/.config/drift/watch-state.toml`, SIGINT/SIGTERM finish the
  current capture before exit.
- **Homebrew tap live**: `brew install ShellFans-Kirin/drift/drift`
  against the public [homebrew-drift](https://github.com/ShellFans-Kirin/homebrew-drift)
  tap; formula is auto-regenerated on every release via a
  `repository_dispatch` from `release.yml`.
- **Published to crates.io**: `drift-core`, `drift-connectors`,
  `drift-mcp`, `drift-ai`.

### Changed
- `CompactionProvider::compact` now returns `CompactionResult`
  (summary + optional usage) instead of `CompactedSummary` alone, so
  live providers can round-trip billing data.
- `drift init` is idempotent: re-running does not overwrite an
  existing `config.toml`.
- `drift capture` soft-fails on a single-session compaction error
  (logs + skips) so one oversized session doesn't abort the batch.
- `summary_to_markdown` now emits real section headings (`## Summary`,
  `## Key decisions`, `## Files touched`, `## Rejected approaches`,
  `## Open threads`) in place of the one-line `[MOCK]` blurb.

### Fixed
- Workspace internal deps pinned to 0.1.1 (previously 0.1.0) so
  `cargo publish` can resolve against crates.io.
- Accidentally checked-in `.prompts/events.db` from ship-time smoke
  now ignored; `.prompts/` added to `.gitignore` for fresh clones.

### Known limitations
- Context-window Strategy 2 (hierarchical summarization) is scaffolded
  but feature-flagged off. Default behaviour is Strategy 1.
- Cost totals use a hardcoded pricing table (checked against
  Anthropic's public pricing as of 2026-04-23); reconcile against
  <https://www.anthropic.com/pricing> before billing reports.

## [0.1.0] — 2026-04-22

### Added
- Cargo workspace with four crates: `drift-core`, `drift-connectors`,
  `drift-cli` (binary: `drift`), `drift-mcp`.
- First-class connectors for Claude Code + Codex; Aider stub behind a
  feature flag (`aider`).
- Attribution engine: `CodeEvent` rows persisted to SQLite at
  `.prompts/events.db`, SHA-256 ladder for human-edit detection,
  two-tier rename handling (session tool calls + git-log-follow
  fallback), MultiEdit intra-call parent chains.
- Compaction engine with `MockProvider` (default in v0.1.0, tagged
  `[MOCK]`) and an `AnthropicProvider` skeleton whose HTTP call was
  wired up in v0.1.1.
- CLI: `init`, `capture`, `watch`, `list`, `show`, `blame`, `trace`,
  `diff`, `rejected`, `log`, `bind`, `auto-bind`, `install-hook`,
  `sync push/pull`, `config get/set/list`, `mcp`.
- Git notes integration (`refs/notes/drift`): binding, auto-binding by
  timestamp, non-blocking post-commit hook.
- Stdio MCP server with five read-only tools: `drift_blame`,
  `drift_trace`, `drift_rejected`, `drift_log`, `drift_show_event`.
- Plugin skeletons (`plugins/claude-code/`, `plugins/codex/`) —
  unpublished in v0.1.0; targeting marketplaces in v0.2.
- CI (`.github/workflows/ci.yml`) and release (`release.yml`) matrices
  for Linux x86_64/aarch64 + macOS x86_64/aarch64.
- Apache-2.0 licensing, CONTRIBUTING walkthrough for adding connectors,
  code-of-conduct.

### Known limitations
- Anthropic compaction HTTP call is stubbed. Mock path is the shipping
  default; wire-up noted in
  `crates/drift-core/src/compaction.rs`.
- Human-edit detection is timeline-only — no authorship claim.
- Codex `reasoning` items are encrypted; we count but do not surface
  them.
- `drift watch` is a debounced polling daemon; v0.2 will move to fully
  event-driven.
- `cargo publish` not executed from this cut; Cargo.toml metadata is
  complete for `0.1.1`.
