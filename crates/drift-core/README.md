# drift-core

Core data model, attribution engine, and compaction provider trait for
[**Drift AI**](https://github.com/ShellFans-Kirin/drift_ai) — the AI-native
blame CLI.

This crate contains the library code that both the `drift` CLI binary
(published as [`drift-ai`](https://crates.io/crates/drift-ai)) and the
stdio MCP server ([`drift-mcp`](https://crates.io/crates/drift-mcp)) are
built on:

- **`NormalizedSession` + `CodeEvent`** — agent-agnostic session and
  per-file event models.
- **`EventStore`** — WAL-mode SQLite storage with `code_events`,
  `sessions`, `file_shas`, and (v0.1.1+) `compaction_calls` tables.
- **`CompactionProvider` trait** — `MockProvider` for offline/test runs,
  `AnthropicProvider` for live `POST /v1/messages?stream=true` with
  retry + context-window handling.
- **Attribution engine** — SHA-256 ladder for human-edit detection;
  `commit_drafts` upserts events with parent chains.

See the [top-level README](https://github.com/ShellFans-Kirin/drift_ai)
for install, quickstart, and the full feature tour.

Licensed under [Apache-2.0](https://github.com/ShellFans-Kirin/drift_ai/blob/main/LICENSE).
