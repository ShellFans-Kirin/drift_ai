> 🌐 **English** · [日本語](CHANGELOG.ja.md) · [简体中文](CHANGELOG.zh-Hans.md) · [繁體中文](CHANGELOG.zh-Hant.md)

# Changelog

All notable changes to drift_ai are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [SemVer](https://semver.org/spec/v2.0.0.html).

## [0.4.1] — 2026-04-26

Doc-only patch on top of v0.4.0. The compaction / attribution / connector
code paths are unchanged from v0.4.0; only the CLI help text changes.

### Fixed

- `drift handoff --help` now lists `cursor` and `aider` as valid `--to`
  values. The runtime parser already accepted them in v0.4.0
  ([`TargetAgent::parse`](crates/drift-core/src/handoff.rs)), but the
  clap doc comment in `crates/drift-cli/src/main.rs` had been left at
  the v0.2 list of `claude-code | codex | generic`. No behaviour change
  for callers that knew the value already worked.

### Real Mac brew install verification (the actual reason this patch exists)

v0.4.0's ship gate was missing one entry: nobody had run
`brew install ShellFans-Kirin/drift/drift` on a real macOS host. After
v0.4.0 shipped, the verification was run via SSH to a Mac mini (Apple
M4 / macOS 26.3.1 / Homebrew 5.1.7); install + smoke + uninstall all
green. The smoke surfaced the `--help` regression above, which is what
this patch fixes.

Real Mac smoke results (v0.4.0):

| Step | Result |
|---|---|
| `brew tap ShellFans-Kirin/drift` | ✓ |
| `brew install drift` | ✓ 7.8 s, 7.1 MB at `/opt/homebrew/Cellar/drift/0.4.0` |
| `drift --version` | ✓ `drift 0.4.0` |
| `drift mcp` initialize handshake | ✓ `serverInfo.version=0.4.0` |
| Binary integrity | ✓ Mach-O thin arm64, ad-hoc codesigned |
| `drift handoff --to cursor --print` runtime | ✓ accepted (footer correct) |
| `drift handoff --to aider --print` runtime | ✓ accepted (footer correct) |
| Cleanup (`uninstall` + `untap`) | ✓ no residue |

This patch's release goes through the same gate end-to-end on Mac, so
v0.4.1 is the first version that's been *fully* verified on macOS at
the brew-install level.

## [0.4.0] — 2026-04-25

The "vendor-neutral" release. Merges what was originally planned as v0.3
(multi-LLM provider) and v0.4 (multi-agent connector) into a single ship,
because half the story isn't a story. After v0.4 you can hand off
between any combination of **{Claude Code, Codex, Cursor, Aider}** as
the *source* and any of **{Anthropic, OpenAI, Gemini, DeepSeek, local
Ollama, anything OpenAI-compatible}** as the *LLM doing the brief*.

### Added — Multi-LLM provider (v0.3 part)

- **`OpenAIProvider`**: gpt-5 / gpt-4o / o1 / o3 series. SSE streaming
  via the shared `compaction::streaming::for_each_sse_data`. Reasoning
  tokens (o1/o3) folded into output tokens for billing transparency.
  Built-in price table for current model SKUs.
- **`GeminiProvider`**: Google AI Studio
  (`generativelanguage.googleapis.com`). API key as URL query param,
  `systemInstruction` at top-level, `:streamGenerateContent?alt=sse`
  endpoint. Sends `thinkingConfig.thinkingBudget = 0` so 2.5-flash's
  hidden reasoning doesn't drain the visible output budget.
- **`OllamaProvider`**: local LLM at `http://localhost:11434/api/chat`.
  NDJSON streaming (one JSON per line, not SSE). Cost is always $0.
  Surfaces a friendly `Ollama is not running. Start it with: ollama serve`
  message instead of a raw connection error.
- **`OpenAICompatibleProvider`**: generic OpenAI-protocol client. Same
  wire path for **DeepSeek / Groq / Mistral / Together AI / Fireworks /
  LM Studio / vLLM** — any vendor that speaks `chat.completions`. Cost
  is user-supplied via `cost_per_1m_input_usd` / `cost_per_1m_output_usd`
  in config because we don't track third-party prices (they go stale).
- **`compaction::factory::make_provider` + `make_completer`**: build a
  `Box<dyn CompactionProvider>` / `Box<dyn LlmCompleter>` from a
  `RoutingConfig`. Used by both `drift capture` and `drift handoff`.
- **`LlmCompleter` trait**: parallel to `CompactionProvider`, lets
  handoff dispatch through any provider via dynamic dispatch.
- **Config schema gains `[handoff.providers.<name>]` and
  `[compaction.providers.<name>]`**. `provider = "anthropic"` remains
  the default; v0.2 configs upgrade losslessly.
- **`drift init` writes a richer config template** — every supported
  provider has a commented-out template the user uncomments to switch.
- **CRLF SSE delimiter support** in the shared streaming helper
  (`compaction::streaming::find_event_boundary`). Caught a Gemini
  parsing bug where `\r\n\r\n` event boundaries were being missed.
- **Real-API smoke harness** at `crates/drift-core/tests/v030_real_smoke.rs`,
  covering Anthropic + OpenAI + Gemini + DeepSeek + Ollama. Self-skips
  when env / daemon is missing. Results captured in `docs/V030-V040-SMOKE.md`.

### Added — Multi-agent connector (v0.4 part)

- **`CursorConnector`**: parses Cursor's per-workspace SQLite store
  (`state.vscdb`, `cursorDiskKV` table) for `composerData:*` /
  `cursorPanelView:*` rows. Maps composer messages → turns and
  `edits[]` → `CodeEvent`s with operation inferred from before/after
  presence. **Read-only**: never writes back to Cursor's DB.
- **`AiderConnector` full impl** (was a 27-line stub in v0.1). Parses
  `.aider.chat.history.md` markdown: `> `-prefixed lines = user, the
  rest = assistant. Extracts ` ```diff ` fenced blocks as tool calls
  keyed by file path.
- **`AgentSlug::Cursor`** as a first-class slug; Cursor + Aider both
  default-on in the connector feature set.
- **`TargetAgent::Cursor` + `TargetAgent::Aider`**: `drift handoff
  --to cursor|aider` now produces footers tailored to those agents'
  paste workflows.
- **9 unit tests for Cursor + 11 for Aider** with synthetic fixtures.
  No real user data is committed.
- **CONTRIBUTING.md walkthrough rewritten** to use both connectors as
  worked examples (markdown-style for Aider, SQLite-style for Cursor).

### Changed

- `compaction.rs` is now a Rust 2018 sibling-submodule parent — the
  v0.2 Anthropic + Mock implementations stay exactly as shipped, with
  the new providers in `compaction/{openai,gemini,ollama,openai_compat,
  factory,streaming}.rs`. Public API is preserved; v0.2 callers keep
  working.
- `Role` enum gains `Copy` (additive — no break).
- `cursor: false` and `aider: false` defaults in `[connectors]` are
  *opt-in* — set them to `true` to enable, matching the v0.1 stance for
  agents that may not be installed on every host.

### Stability guarantees carried from v0.1 / v0.2

- `events.db` schema is unchanged. Upgrading from v0.2.x is a binary
  swap; no migration.
- MCP tool list (`drift_blame` / `drift_trace` / `drift_rejected` /
  `drift_log` / `drift_show_event`) is unchanged.
- `SessionConnector` trait signature is unchanged.
- `CompactionProvider` trait is unchanged. New providers extend the
  set; the trait itself isn't widened.
- `CompactionError` enum is unchanged.
- v0.1.2 first-run privacy notice still fires unchanged.

### Known limitations (v0.4)

- **Cursor schema is reverse-engineered** and undocumented. The
  connector is `[BEST-EFFORT]` and may break with future Cursor
  versions — it emits warnings rather than failing the whole capture
  run when a row doesn't parse.
- **Aider has no tool_call structure**, so `rejected = false` is the
  default for every event; the SHA-256 ladder still detects when
  reality diverges.
- **`OpenAICompatibleProvider` doesn't ship a price table**. Users
  supply `cost_per_1m_*_usd` per provider entry; if absent, `drift cost`
  shows that provider as unpriced.
- **Cursor agent-mode composer state** (multi-step tool-call replay)
  is partial — chat messages are extracted, the inner state machine is
  not. Full agent-mode support is v0.5.
- **Gemini is wired against AI Studio**, not Vertex AI. Vertex (and
  Bedrock + Azure OpenAI) is the v0.5 enterprise wave.

### Test count

v0.2.0: 67 tests · v0.4.0: **122 tests** (+55) — 35 in compaction
(provider parsers + factory + streaming), 9 in cursor connector, 11 in
aider connector. All four cloud providers verified with real API smoke.

## [0.2.0] — 2026-04-25

The "you don't get locked into one LLM vendor" release. Adds
**`drift handoff`** — the new headline command — and a v0.2-style
README that puts task transfer at the front and demotes blame to a
supporting feature.

### Added

- **`drift handoff` CLI**. Packages an in-progress task (filtered by
  `--branch`, `--since`, or `--session`) into a markdown brief that
  another agent can absorb cold. Flags: `--to claude-code|codex|generic`,
  `--output <path>`, `--print`. Default output:
  `.prompts/handoffs/<YYYY-MM-DD-HHMM>-<branch>-to-<agent>.md`.
- **`crates/drift-core/src/handoff.rs`** — orchestrator + four small
  collectors (sessions, events-by-file, rejected approaches, file
  snippets) + LLM second pass + pure-Rust `render_brief`. New unit
  tests cover scope parsing, snippet extraction (full vs. modified-range
  excerpt), JSON-from-LLM parsing (with code-fence + surrounding-prose
  tolerance), and per-`--to` footer rendering. 15 new tests.
- **`crates/drift-core/templates/handoff.md`** — LLM prompt template
  for the second pass; instructs the model to emit JSON with
  `summary` / `progress` / `key_decisions` / `open_questions` /
  `next_steps`.
- **`AnthropicProvider::complete_async`** + sync `complete` — generic
  system+user → text completion that re-uses `compact_async`'s
  retry / streaming / token-usage machinery for callers that need an
  LLM call with a different prompt shape (used by handoff). Returns a
  new `LlmCompletion` struct (text + per-call token / cost).
- **`[handoff]` config section** in `.prompts/config.toml`. Default
  model `claude-opus-4-7`. The default is Opus — handoff briefs are
  user-facing artifacts the next agent reads verbatim, narrative quality
  is the value, and handoff frequency is low (a few per workday at
  most). Users can drop to Haiku for ~30× cost reduction.
- **30-second demo** at `docs/demo/v020-handoff.gif` (real recording
  of `drift handoff` against fixture data; cast file at
  `docs/demo/v020-handoff.cast`).
- **Real-Anthropic smoke output** captured at
  [`docs/V020-SMOKE-OUTPUT.md`](docs/V020-SMOKE-OUTPUT.md).
- **`docs/V020-DESIGN.md`** — Phase 0 design proposal kept in repo as
  reference for the `drift handoff` shape.

### Changed

- README first screen pivots to `drift handoff` as the headline,
  with the demo GIF in the hero spot. The blame / log feature is
  retained as a "supporting feature" reference in the same screen.
- Quickstart bumped from 5 commands to 6 (added `drift handoff`).
- About section adds a one-line dogfood-origin note.
- Pre-built binary install URL bumped to `drift-v0.2.0`.

### Stability guarantees carried from v0.1

- `events.db` schema is **unchanged**. Upgrading from v0.1.x is a
  pure binary swap; no migration required.
- MCP tool list is **unchanged**. Existing MCP clients keep working.
- `SessionConnector` trait is **unchanged**. Existing connectors
  keep working.
- The v0.1.2 first-run privacy notice still fires on first
  `drift capture`; nothing to re-acknowledge for handoff.

### Known limitations (v0.2)

- `--branch <name>` scoping is best-effort: it asks `git log <branch>
  --not main --format=%aI` for the earliest divergent commit and uses
  that as a lower-bound filter. Sessions on multiple parallel branches
  on the same day may bleed across — refine via `--since`.
- The handoff LLM call has the cost profile of any Opus call (~$0.10
  per brief). Heavy users should set `[handoff].model =
  "claude-haiku-4-5"`.
- No `drift handoff list` / `drift handoff show <id>` yet — generated
  briefs are just markdown files in `.prompts/handoffs/`. `ls` and
  `cat` is the v0.2 query interface.

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
