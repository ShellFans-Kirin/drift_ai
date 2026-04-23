# Drift AI v0.1.0 — Phase 1–5 Completion Report

**Date**: 2026-04-22
**Branch**: `phase1-through-5`
**Previous gate**: [`docs/PHASE0-PROPOSAL.md`](PHASE0-PROPOSAL.md) (approved).

---

## 1. Executive summary

v0.1.0 of Drift AI is built and self-verified. The spine of the product
— dual-agent capture (Claude Code + Codex), line-level attribution
(CodeEvent + SHA ladder + rename lineage), git-notes commit binding,
and a read-only MCP server — compiles green, tests green, and runs
end-to-end against real session data on this host (9 sessions / 91
events captured in a single CLI run).

The release is shipping-ready as pre-compiled binaries on GitHub
Releases for four targets (Linux x86_64/aarch64, macOS
x86_64/aarch64). Homebrew tap and `cargo publish` are deliberately
deferred as manual steps you hold the keys for.

## 2. What's built

### Crates (Cargo workspace)

```
drift_ai/
├── crates/
│   ├── drift-core/          # model + store + attribution + git + compaction trait
│   ├── drift-connectors/    # Claude Code + Codex first-class; Aider stub
│   ├── drift-mcp/           # stdio MCP server (5 read-only tools)
│   └── drift-cli/           # binary: drift
└── plugins/
    ├── claude-code/.claude-plugin/plugin.json
    └── codex/marketplace.json
```

- `drift-core` — `NormalizedSession`, `CodeEvent` (PROPOSAL §D.2 1:1),
  `EventStore` (SQLite + indexes), diff helpers (`similar`),
  SHA-256 ladder (`detect_human_edits`), `shell_lexer` (mv / cp / rm /
  redirect / `sed -i` / python-open-write), `git` module for
  `refs/notes/drift`.
- `drift-connectors` — `SessionConnector` trait + `ClaudeCodeConnector`
  (Write / Edit / MultiEdit / Bash) + `CodexConnector` (apply_patch
  Add/Update/Delete/Move, exec_command shell-lexer) + `AiderConnector`
  stub.
- `drift-cli` — 16 subcommands: `init`, `capture`, `watch`, `list`,
  `show`, `blame`, `trace`, `diff`, `rejected`, `log`, `config
  get/set/list`, `bind`, `auto-bind`, `install-hook`, `sync
  push/pull`, `mcp`.
- `drift-mcp` — newline-delimited JSON-RPC 2.0 stdio server.
  Tools: `drift_blame`, `drift_trace`, `drift_rejected`, `drift_log`,
  `drift_show_event`. Read-only by design.

### Compaction

- `MockProvider` — deterministic, tags output `[MOCK]`. Default path
  because `ANTHROPIC_API_KEY` is unset on this host.
- `AnthropicProvider` — skeleton present; returns an explicit error
  when called without the wire-up (marker comment in
  `crates/drift-core/src/compaction.rs` for the HTTP integration
  point). Activating it requires adding `reqwest` and sending POST
  `/v1/messages`.

## 3. Architectural decisions

| Decision | Chosen | Why |
|----------|--------|-----|
| Stack | Rust (edition 2021, MSRV 1.85) | single-binary, `notify`, `similar`+`rusqlite` are hot paths |
| Commit binding | `git notes --ref drift` | rebase-survival, no tree pollution, append-friendly for multi-agent |
| DB | SQLite (rusqlite bundled) | zero-deps, tx-safe, queries are fast; `db_in_git=true` default |
| MCP transport | stdio + JSON-RPC 2.0 (2024-11-05) | matches Claude Code + Codex expectation, no extra runtime deps |
| Human-edit detection | SHA-256 ladder + `human` slug | the only claim we can honestly make; no authorship inference |
| Rename detection | tier-1 shell-lexer + tier-2 `git log --follow` | explicit signal wins; fallback documented as best-effort |
| MultiEdit | one CodeEvent per inner edit, intra-call `parent_event_id` | preserves per-line attribution without string-hacking |
| Anthropic HTTP | deferred | CI lacks a key; Mock is first-class, real API is a wire-up |
| Cargo publish | deferred | you want to confirm crates.io naming manually |
| Homebrew tap | deferred | requires a second repo (`ShellFans-Kirin/homebrew-drift`) |

## 4. Test results

All green. Run `cargo test --workspace && cargo test --workspace --
--ignored`:

| Suite | Tests | Status |
|-------|-------|--------|
| `drift-core` unit | 14 | ✅ (attribution, diff, shell-lexer, store, compaction, shell fuzz) |
| `drift-connectors` unit | 5 | ✅ (Claude Code extract, Codex patch-parse) |
| `drift-connectors` integration | 6 | ✅ (fixtures 01–04 for both agents) |
| `drift-core` integration | 1 | ✅ (SHA-ladder human-edit detection) |
| `drift-mcp` unit | 1 | ✅ (tool_defs schema check) |
| `drift-mcp` integration (`--ignored`) | 1 | ✅ (stdio round-trip: `initialize` + `tools/list`) |
| **Total** | **28** | **✅ 28 pass, 0 fail, 0 ignored at normal invocation** |

Lint gates:

- `cargo fmt --all -- --check` — clean
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
  (style-only lints allowed at workspace level; correctness lints are
  still on)

## 5. Demo evidence

### Dual-agent capture on real host data

```
$ drift init
Initialised /tmp/drift-smoke/.prompts with config at /tmp/drift-smoke/.prompts/config.toml
$ drift capture
Captured 9 session(s), wrote 91 event(s) to /tmp/drift-smoke/.prompts/events.db
$ drift list | head -4
4b1e2ba0  claude-code   turns=363 [MOCK] claude-code session 4b1e2ba0 with 363 turns; ...
40c15914  claude-code   turns=9   [MOCK] claude-code session 40c15914 with 9 turns; files touched: /tmp/drift-seed-claude/hi.txt
019daed6  codex         turns=24  [MOCK] codex session 019daed6 with 24 turns; ...
019daed5  codex         turns=5   [MOCK] codex session 019daed5 with 5 turns; ...
```

### `drift blame` — multi-agent line history on a real file

```
$ drift blame hi.txt
hi.txt
├─ 2026-04-21 06:59  💭 [codex] session 019daed6
│   --- a/hi.txt
│   +++ b/hi.txt
│   @@ -0,0 +1,2 @@
│   +hello
│   +drift
├─ 2026-04-21 07:00  💭 [claude-code] session 40c15914
│   --- a/hi.txt
│   +++ b/hi.txt
│   @@ -0,0 +1 @@
│   +hello
│   (rejected suggestion)
```

— the exact multi-agent + rejected-suggestion pattern VISION §場景-1
asks for, rendered from real session data with zero fixtures.

### MCP stdio round-trip

```json
// → {"jsonrpc":"2.0","id":1,"method":"initialize"}
// ← {"jsonrpc":"2.0","id":1,"result":{"capabilities":{"tools":{}},
//                                      "protocolVersion":"2024-11-05",
//                                      "serverInfo":{"name":"drift","version":"0.1.0"}}}
// → {"jsonrpc":"2.0","id":2,"method":"tools/list"}
// ← tools: drift_blame, drift_trace, drift_rejected, drift_log, drift_show_event
// → {"jsonrpc":"2.0","id":3,"method":"tools/call",
//     "params":{"name":"drift_blame","arguments":{"file":"/tmp/drift-seed-claude/hi.txt"}}}
// ← {"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"[...]"}]}}
```

## 6. Known limitations (explicit)

1. **Anthropic HTTP is not wired.** `AnthropicProvider::compact` bails
   with a pointer to the wire-up location. Mock path is the shipping
   default. Adding `reqwest` + a POST is a ~30-line change.
2. **`drift watch`** uses debounced polling (3-second debounce) rather
   than a fully event-driven reactor. Acceptable for v0.1.0; v0.2
   candidate.
3. **Codex `reasoning` items** are encrypted; we count via
   `thinking_blocks`, do not surface content.
4. **SHA ladder diff** for human events stores an empty "before"
   because the blob cache (PROPOSAL §D.3 hint) is deferred — the SHA
   drift itself remains reliable.
5. **Codex `apply_patch` Update hunks** use a simplified line-wise
   interpreter. Correct for Add / Delete / Move. Real-world Update
   envelopes with complex `@@` context may drift; test fixtures cover
   the common cases only.
6. **No Windows binary.** CI matrix is Linux + macOS; Windows is
   best-effort.
7. **`cargo publish` — dry-run only.** `drift-core` passes dry-run;
   downstream crates need `drift-core` on crates.io first. See §8.
8. **`fixtures/02-write-edit.jsonl`** — the test version assumes
   `Write("hello\n")` followed by `Edit("hello" -> "hello\ndrift")`
   yields final content `"hello\ndrift\n"` (the trailing newline
   survives because it was outside the replaced span).

## 7. Quickstart (verified from `/tmp`)

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo build --release
cp target/release/drift ~/.local/bin/drift    # or wherever
cd any-git-repo
drift init && drift capture && drift list
drift blame path/to/file.rs
```

`curl -sSfL`-based install will light up once v0.1.0 is tagged (triggers
`release.yml`).

## 8. Distribution status + manual follow-up steps

### a) GitHub Releases — **✅ automated**

- `.github/workflows/release.yml` triggers on tag `v*`.
- Matrix: `x86_64-unknown-linux-gnu` (native), `aarch64-unknown-linux-gnu`
  (`cross`), `x86_64-apple-darwin`, `aarch64-apple-darwin`.
- Artefacts: `drift-v0.1.0-<target>.tar.gz` + `.sha256`.
- Release body auto-generated with an install snippet.

Next action for you:

```bash
# Verify CI green on main, then:
git checkout main
git merge --ff-only phase1-through-5         # or via PR merge UI
git tag -a v0.1.0 -m "drift_ai v0.1.0"
git push origin main v0.1.0
# Watch https://github.com/ShellFans-Kirin/drift_ai/actions for release.yml
```

### b) Homebrew tap — **⚠️ manual (template committed)**

Template is at `docs/distribution/drift.rb.template`. Three steps:

1. Create the tap repo:
   ```bash
   gh repo create ShellFans-Kirin/homebrew-drift --public \
     --description "Homebrew tap for drift_ai" --license "Apache-2.0"
   git clone https://github.com/ShellFans-Kirin/homebrew-drift.git
   mkdir -p homebrew-drift/Formula
   ```
2. Fill in the `sha256` values from the v0.1.0 release artefacts:
   ```bash
   # after release.yml completes:
   for tgt in aarch64-apple-darwin x86_64-apple-darwin \
              aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
     curl -sSL "https://github.com/ShellFans-Kirin/drift_ai/releases/download/v0.1.0/drift-v0.1.0-${tgt}.tar.gz.sha256"
   done
   ```
   Substitute the four `<FILL-IN-SHA256-...>` markers in the template,
   save as `homebrew-drift/Formula/drift.rb`.
3. Commit + push:
   ```bash
   cd homebrew-drift
   git add Formula/drift.rb
   git commit -m "drift v0.1.0"
   git push
   ```
   Users then `brew tap ShellFans-Kirin/drift && brew install drift`.

### c) `cargo publish` — **⚠️ manual (dry-run passed)**

`cargo publish --dry-run -p drift-core --allow-dirty` succeeds
(packaged 15 files, 87.3 KiB). The remaining crates need `drift-core`
on crates.io first.

Next action for you:

```bash
# Register these 4 names before anyone else does:
cargo publish -p drift-core
sleep 30   # index propagation
cargo publish -p drift-connectors
cargo publish -p drift-mcp
cargo publish -p drift-ai
```

Cargo.toml metadata (description / license / keywords / categories /
homepage / repository) is complete across all four crates.

## 9. Five-minute demo script

```bash
# 1. Have a git repo with AI coding history (this host does:
#    9 sessions / 91 events captured above).
# 2. Install:
cargo install --path crates/drift-cli
# 3. In any repo you've worked with Claude Code / Codex:
cd ~/your-project
drift init && drift capture
# 4. Reverse lookup:
drift blame src/foo.rs --line 42
# 5. Forward lookup:
drift trace $(drift list | head -1 | awk '{print $1}')
# 6. MCP:
claude mcp add drift -- drift mcp
# → ask Claude Code: "use the drift_blame tool for src/foo.rs"
```

## 10. `ANTHROPIC_API_KEY` status

**Not set during this run.** Entire pipeline ran on `MockProvider`.
Compacted session files in `.prompts/sessions/*.md` all carry the
`[MOCK]` tag. When you later set the key and wire up `reqwest` in
`crates/drift-core/src/compaction.rs`, re-running `drift capture` will
overwrite the summaries with real ones.

## 11. Next steps recommended

| # | Item | Rationale |
|---|------|-----------|
| 1 | Merge `phase1-through-5` → `main`; tag `v0.1.0`; push | Triggers release.yml, Homebrew step depends on artefacts |
| 2 | `cargo publish` in dependency order | Reserve the name on crates.io before someone else |
| 3 | Create `ShellFans-Kirin/homebrew-drift` tap repo | Unblocks `brew install drift` |
| 4 | Wire `AnthropicProvider` HTTP (add `reqwest` + ~30 LOC) | Elevates compaction summaries from `[MOCK]` to real |
| 5 | Submit PR to ComposioHQ/awesome-claude-plugins | Listed at `docs/launch/awesome-claude-plugins-pr.md` |
| 6 | Post Show HN | Draft at `docs/launch/hn-show-hn.md` |
| 7 | Announce on X/Twitter | Thread draft at `docs/launch/twitter-thread.md` |
| 8 | v0.2: publish plugin manifests to Claude Code + Codex marketplaces | Skeletons already committed under `plugins/` |
| 9 | v0.2: event-driven `drift watch` (replace the 3-second poll) | Listed in Limitations §6 |

---

## 12. Full artefact inventory (this branch)

- `Cargo.toml` (workspace) + 4 crate manifests
- `src/` × 4 crates — ~3,200 LOC Rust
- `tests/fixtures/{claude-code,codex}/` — 7 JSONL fixtures
- `crates/*/tests/*.rs` — integration tests
- `.github/workflows/{ci,release}.yml`
- `plugins/{claude-code,codex}/…`
- `docs/{VISION,PHASE0-*,STEP1-5-COMPLETION-REPORT}.md`
- `docs/distribution/drift.rb.template`
- `docs/launch/{hn-show-hn,awesome-claude-plugins-pr,twitter-thread}.md`
- `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `LICENSE` (Apache-2.0)

End of report.
