# Changelog

All notable changes to drift_ai are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [SemVer](https://semver.org/spec/v2.0.0.html).

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
- Compaction engine with `MockProvider` (default, tagged `[MOCK]`) and
  `AnthropicProvider` skeleton (activates when `ANTHROPIC_API_KEY` is
  set; HTTP integration point marked in the source).
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
