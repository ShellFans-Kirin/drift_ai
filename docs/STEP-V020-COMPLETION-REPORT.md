# Drift AI v0.2.0 — Ship Report

**Date**: 2026-04-25
**Theme**: "you don't get locked into one LLM vendor" — `drift handoff` ships.
**Behavioural change vs v0.1.2**: only one new sync code path
(`AnthropicProvider::complete`) and one new CLI command (`drift handoff`).
The compaction / attribution / MCP / connector layers are unchanged from
v0.1.x.

---

## 交付連結

| 項目 | URL |
|---|---|
| GitHub Release | <https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.2.0> |
| Release assets | 4 × tarball + 4 × .sha256 (aarch64/x86_64 × Linux/macOS) |
| `drift-core` | <https://crates.io/crates/drift-core> · `max_version=0.2.0` |
| `drift-connectors` | <https://crates.io/crates/drift-connectors> · `max_version=0.2.0` |
| `drift-mcp` | <https://crates.io/crates/drift-mcp> · `max_version=0.2.0` |
| `drift-ai` | <https://crates.io/crates/drift-ai> · `max_version=0.2.0` |
| Homebrew Formula | <https://github.com/ShellFans-Kirin/homebrew-drift/blob/main/Formula/drift.rb> · `version "0.2.0"` |
| CHANGELOG | [`CHANGELOG.md` § 0.2.0](https://github.com/ShellFans-Kirin/drift_ai/blob/main/CHANGELOG.md#020--2026-04-25) |
| Design proposal | [`docs/V020-DESIGN.md`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V020-DESIGN.md) |
| Smoke output | [`docs/V020-SMOKE-OUTPUT.md`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V020-SMOKE-OUTPUT.md) |
| Demo cast | [`docs/demo/v020-handoff.cast`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/demo/v020-handoff.cast) — replay with `asciinema play` |
| Demo GIF | [`docs/demo/v020-handoff.gif`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/demo/v020-handoff.gif) — embedded in README |

---

## Phase summary

### Phase 0 — Design proposal (Phase-gated approval)

`docs/V020-DESIGN.md` (522 lines, 9 chapters) shipped as the only Phase 0
deliverable; reviewed and approved with one diff (default model
`claude-haiku-4-5` → `claude-opus-4-7`). Rationale: handoff briefs are
user-facing artifacts the next agent reads verbatim, so narrative
quality justifies the 30× cost premium for an infrequent operation.

### Phase 1 — Implementation

Code added (no `events.db` schema / MCP tool / SessionConnector trait
changes — those are frozen as committed):

- **`crates/drift-core/src/handoff.rs`** (1142 lines) — `build_handoff`
  orchestrates four small collectors, `render_brief` is pure-Rust string
  formatting. 22 new tests in this module + the CLI.
- **`crates/drift-core/templates/handoff.md`** (59 lines) — JSON-output
  prompt template.
- **`AnthropicProvider::complete_async` + `complete`** (236 lines added
  to `compaction.rs`) — generic system+user → text completion with the
  same retry / SSE machinery as `compact_async`. Returns `LlmCompletion`
  (text + tokens). Re-exported from `drift_core`.
- **`[handoff]` config section** (drift-core::config) — `model =
  "claude-opus-4-7"` default.
- **`crates/drift-cli/src/commands/handoff.rs`** (257 lines) — the
  `drift handoff` CLI: scope flags (`--branch` / `--since` / `--session`,
  mutually exclusive), `--to claude-code|codex|generic`, `--output` /
  `--print`, plus stderr `⚡` progress lines + final next-step hint.

### Phase 2 — Tests + real Anthropic smoke

- **67 unit + integration tests** all green (up from 45 in v0.1.2). 22 new
  tests for handoff + CLI scope resolution.
- `cargo fmt --all -- --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- **Real Opus smoke**: `drift handoff --session 3d132809...` against
  `/tmp/drift-smoke-v011/.prompts/events.db`, ~ \$0.05, 6 seconds wall
  time. Notable behaviour: Opus correctly identified the empty-task case
  and produced a "cold start" brief asking the next agent to ask the
  user for goal/branch/files instead of fabricating progress. Captured
  verbatim in `docs/V020-SMOKE-OUTPUT.md`.

### Phase 3 — Demo + README rework

- **`docs/demo/v020-handoff.cast`** — real asciinema recording of
  `drift handoff` against fixture data, real Anthropic Opus call,
  ~30 second playback. 4.4 KB.
- **`docs/demo/v020-handoff.gif`** — rendered via `agg` from the cast
  for inline GitHub README rendering. 120 KB, 688×490, monokai theme.
- **README pivot**: demo GIF in the hero spot above the pain copy
  (*"Your AI coding agent stalled — refused, rate-limited, or just got
  dumb..."*). Quickstart bumped from 5 to 6 commands. New `## Handoff`
  section before `## Live mode`. Blame / log content retained as
  supporting feature.
- **CHANGELOG v0.2.0** entry: 8 Added items + Stability guarantees
  re-affirming the v0.1.x freezes + Known limitations.

### Phase 4 — Release

| Step | State |
|---|---|
| Squash-merge v0.2.0 → main | ✅ commit `e6d6371` |
| Tag v0.2.0 + push | ✅ |
| `release.yml` 4-target build | ✅ run `24925365141` completed/success |
| Tap `repository_dispatch` auto-fired | ✅ tap run at `2026-04-25T07:11:02Z` |
| Tap `Formula/drift.rb` auto-updated to `version "0.2.0"` | ✅ all 4 sha256 filled |
| `cargo publish drift-core` (real) | ✅ |
| `cargo publish drift-connectors` (real) | ✅ |
| `cargo publish drift-mcp` (real) | ✅ |
| `cargo publish drift-ai` (real) | ✅ |
| All 4 crates `max_version=0.2.0` on crates.io | ✅ |
| `cargo install drift-ai --locked` from clean `/tmp` | ✅ 1m 56s, `drift 0.2.0` |
| Mac mini `brew install drift` (Tailscale SSH, Apple M4) | ✅ 7.6 s, `drift 0.2.0` |
| `drift handoff --help` shows full flag set on Mac | ✅ |
| `drift mcp` JSON-RPC initialize on Mac → `serverInfo.version=0.2.0` | ✅ |
| `brew test drift` on Mac | ✅ |
| `brew uninstall` + `brew untap` on Mac, residue check | ✅ no residue |

---

## v0.2 真實 Anthropic 用量

本次 v0.2 dev cycle 的 Opus 用量（design phase 0 LLM 沒呼叫；Phase 2
real smoke 為主；Phase 3 demo 錄製跑了一次 Opus）：

| Stage | Calls | Approx cost |
|---|---|---|
| Phase 2 Opus smoke (1 session) | 1 | ~\$0.05 |
| Phase 3 demo recording (1 session, Opus) | 1 | ~\$0.05 |
| **Total v0.2 dev** | **2** | **~\$0.10** |

Well under the design-doc cost cap of \$1.00 for the dev cycle.

累計（v0.1.0 → v0.2.0 全部 Anthropic smoke）：

| | Calls | Cost |
|---|---|---|
| v0.1.0 (design + Mock smoke) | 0 | \$0 |
| v0.1.1 (Phase A8 smoke + 2 demo runs) | 23 | ~\$3.07 |
| v0.1.2 (smoke + 1 dev run) | 13 | \$0.20 |
| **v0.2.0** (smoke + demo) | 2 | \$0.10 |
| **Total** | **38** | **~\$3.37** |

---

## Stability guarantees re-affirmed

These are unchanged from v0.1.x — upgrading from v0.1.2 → v0.2.0 is a
binary swap:

- `events.db` schema: **unchanged** since v0.1.1 (`compaction_calls`
  table added in v0.1.1; v0.2 only reads, never writes new schema).
- MCP tool list: **unchanged** since v0.1.0 (5 read-only tools).
- `SessionConnector` trait: **unchanged** since v0.1.0.
- v0.1.2 first-run privacy notice still fires on `drift capture`;
  nothing to re-acknowledge for handoff.

---

## Known limitations carried into v0.2

- `drift handoff --branch <name>` scoping is best-effort: the lower bound
  is the earliest commit on the branch divergent from `main`. Sessions
  on multiple parallel branches on the same day may bleed across — refine
  with `--since <iso>`.
- LLM cost at Opus default: ~\$0.10 per handoff. Heavy users
  should drop to Haiku via `[handoff].model = "claude-haiku-4-5"`
  (~30× reduction).
- No `drift handoff list` / `drift handoff show <id>` yet — generated
  briefs are markdown files in `.prompts/handoffs/`. `ls` and `cat` is
  the v0.2 query interface. v0.3+ if user demand exists.
- No cross-agent prompt-schema translation (tool-call adapters). The
  handoff body is identical across `--to` values; only the footer
  differs. v0.3+.

---

## Next steps (post-v0.2 ship)

1. **Phase 5 deliverables**: Show HN draft + Twitter thread + pre-launch
   checklist will be committed under `docs/launch/v020-*`. **No actual
   posting** until the user signs off on timing.
2. **Public visibility window**: same recommendation as v0.1.2 — Tue/Wed/Thu
   台北 evening 8-10 PM (= US Pacific morning) for HN.
3. **Awesome-claude-plugins PR**: existing draft to bump entry to v0.2.0
   + add handoff feature line.

---

## Ship gate — 一行結論

🟢 **drift v0.2.0 launch ready**. Code, docs, demo, release, crates.io,
Homebrew, Mac install verify all green. No outstanding blocker.
