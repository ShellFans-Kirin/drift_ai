# drift v0.4.2 Ship Report

**Date**: 2026-04-26
**Branch**: `v0.4.2` (squash-merged into `main` via FF; PAT lacked PR-write
on dev_only, same path as prior releases).
**Tag**: `v0.4.2` @ `f5a0b02`

## Two bug fixes (the actual reason for this release)

| Bug | Before | After |
|---|---|---|
| `[handoff]` config overlay was ignored | Setting `provider = "deepseek"` in `.prompts/config.toml` had **no effect** | `cfg.handoff = proj.handoff` now in `config.rs::load`; `handoff_config_project_overlay_is_applied` regression test locks it in |
| `OpenAICompatibleProvider::complete_async` cost = $0 | Every DeepSeek / Groq / Mistral / vLLM handoff brief reported `cost=$0.0000` | `complete_async` now re-stamps cost with user-supplied per-1M rates; `handoff_complete_re_stamps_user_pricing` regression test locks it in |

Both fixes shipped in commit `fcc3454` (carried into this release with
new regression tests in commit `848dfda`).

## README launch polish (5 required + 3 optimisations, all done)

| # | Change | Status |
|---:|---|---|
| Required 1 | Unify version numbering — drop stale `(v0.2)` headers / `v0.2 will add` claims; bump "Honest limitations" to `(v0.4.2)` | ✓ |
| Required 2 | Rewrite first "Why drift exists" bullet (was non-idiomatic English with duplicate clause) | ✓ |
| Required 3 | Privacy section: clarify `db_in_git = true` default + link to Configuration; "wait for v0.2" → "wait for the redaction pass" | ✓ |
| Required 4 | Quickstart `""x@y""` → `"x@y"` in 4 README locales | ✓ |
| Required 5 | 3-GIF distribution: hero shows GIF 1 only; GIF 3 below pain points; GIF 2 in new cost section | ✓ |
| Optimisation 1 | Tagline 4 lines → 2 lines | ✓ |
| Optimisation 2 | "Why drift exists" 6 wandering bullets → 3 tight ones | ✓ |
| Optimisation 3 | New "Cost across providers (v0.4 multi-LLM)" section + GIF 2 | ✓ |

**Translations (`README.ja.md` / `README.zh-Hans.md` / `README.zh-Hant.md`)**:
tagline rewritten, Privacy clarification ported, install URL bumped to
v0.4.2, broken `v020-handoff.gif` reference replaced with the real
`v040-handoff-bidirectional.gif`. The `Why drift exists` 6→3
condensation and the new multi-LLM cost section are English-only for
v0.4.2; translations will catch up in a follow-up i18n sync (flagged
in CHANGELOG `[0.4.2]` Changed section).

## GIF 2 re-recording

`docs/demo/v042-multi-llm-comparison.gif` (79 KB) — re-recorded with
the v0.4.2 binary so the DeepSeek cost is non-zero ($0.0005 on the
fixture session, vs $0 in the v0.4.0/v0.4.1-recorded version). README
references updated.

## Release pipeline

| Step | Result |
|---|---|
| Workspace `Cargo.toml` 0.4.1 → 0.4.2 + 4 path-deps | ✓ |
| `cargo fmt --all -- --check` | ✓ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✓ |
| `cargo test --workspace` | ✓ **124 passed** (was 122 in v0.4.1; +2 regression tests) |
| `cargo build --release --workspace` | ✓ |
| FF-merge `v0.4.2` → local `main` | ✓ |
| `git push dev_only main + tag v0.4.2` | ✓ |
| `./scripts/release-to-public.sh v0.4.2` | ✓ — public/main + tag pushed |
| `release.yml` 4-target build (Linux/macOS × arm64/x86_64) | ✓ — completed in ~3 min |
| GitHub Release v0.4.2 (8 assets: 4 tarballs + 4 sha256) | ✓ |
| Homebrew Formula auto-update to `version "0.4.2"` + 4 new sha256 | ✓ |
| `cargo publish` 4 crates (drift-core → drift-connectors → drift-mcp → drift-ai) | ✓ all published |
| Linux: `CARGO_HOME=/tmp cargo install drift-ai --locked` → `drift 0.4.2` | ✓ |
| **macOS: brew tap + install + smoke + uninstall + untap** (M4 / 26.3.1, via SSH) | ✓ — 7.5 s install, `drift 0.4.2`, `--to` shows cursor + aider, `mcp serverInfo.version=0.4.2`, `brew test drift` green, Cellar gone |

## Verification artefacts

- [`docs/V042-BUGFIX-VERIFY.md`](docs/V042-BUGFIX-VERIFY.md) — 2 unit
  tests + a real-API 4-provider smoke (Anthropic Haiku / OpenAI
  gpt-4o-mini / Gemini 2.5-flash / DeepSeek-via-OpenAI-compatible),
  all reporting non-zero costs and the correct model name.
- [`docs/V030-V040-SMOKE.md`](docs/V030-V040-SMOKE.md) — original
  v0.3+v0.4 multi-provider smoke (still valid).

## Delivery URLs

- **GitHub Release**: https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.4.2
- **crates.io 4 crates**:
  - https://crates.io/crates/drift-core/0.4.2
  - https://crates.io/crates/drift-connectors/0.4.2
  - https://crates.io/crates/drift-mcp/0.4.2
  - https://crates.io/crates/drift-ai/0.4.2
- **Homebrew Formula**: https://github.com/ShellFans-Kirin/homebrew-drift/blob/main/Formula/drift.rb (`version "0.4.2"`)
- **README** (English): https://github.com/ShellFans-Kirin/drift_ai/blob/main/README.md
- **CHANGELOG `[0.4.2]`**: https://github.com/ShellFans-Kirin/drift_ai/blob/main/CHANGELOG.md
- **Translations**: [ja](https://github.com/ShellFans-Kirin/drift_ai/blob/main/README.ja.md) · [zh-Hans](https://github.com/ShellFans-Kirin/drift_ai/blob/main/README.zh-Hans.md) · [zh-Hant](https://github.com/ShellFans-Kirin/drift_ai/blob/main/README.zh-Hant.md)

## Real-API usage (v0.4.2 work, including bug-fix smoke + GIF re-record)

Approximation across this session's runs:

| Vendor | Approx spend |
|---|---:|
| Anthropic | < $0.30 (Haiku for handoff briefs, Opus during GIF 1 recording) |
| OpenAI | < $0.01 |
| Gemini | < $0.01 |
| DeepSeek | < $0.01 |

Surfaced live via `drift cost` against the local dev `.prompts/events.db`
(once that path's recording starts being captured, which is a v0.4 known
gap — the handoff cost line goes to stderr but isn't stored in
`compaction_calls` yet).

## Mac brew install verification (full transcript)

```
$ brew tap ShellFans-Kirin/drift          ✓ Tapped
$ brew cat drift | grep version           ✓ version "0.4.2"
$ brew install drift                      ✓ 7.5 s, 7.2 MB at /opt/homebrew/Cellar/drift/0.4.2
$ drift --version                         ✓ drift 0.4.2
$ drift handoff --help                    ✓ --to claude-code | codex | cursor | aider | generic
$ drift mcp <init JSON>                   ✓ serverInfo.version=0.4.2
$ brew test drift                         ✓ test pass
$ brew uninstall drift && brew untap ...  ✓ Cellar gone, no PATH residue
```

v0.4.2 is the third release fully verified at the brew-install level
on real macOS (continuing the practice started at v0.4.1).

## Show HN time-to-launch

🟢 **GREEN** — all launch blockers cleared:

| Blocker | Status |
|---|---|
| Multi-LLM headline feature actually works (Bug 1) | ✓ fixed + regression tested |
| OpenAI-compatible cost reporting accurate (Bug 2) | ✓ fixed + regression tested |
| README front-page sharp + accurate | ✓ 5 required + 3 optimisations done |
| 3 demo GIFs (real, not fakes) | ✓ all three in repo |
| Mac brew install green | ✓ |
| Linux cargo install green | ✓ |
| crates.io 4 crates published | ✓ |
| Homebrew tap auto-update | ✓ |

Recommended posting window: Tue/Wed/Thu, 8–10 PM Asia/Taipei. Body +
title in [`docs/launch/v040-show-hn.md`](docs/launch/v040-show-hn.md)
(tweak title/body line to mention v0.4.2 instead of v0.4.0 before
posting).

## One-line conclusion

v0.4.2 ship complete. Two real bugs in the multi-LLM headline feature
fixed and locked with regression tests. README launch-polished. Binary
verified on Linux + macOS. crates.io + Homebrew tap both at 0.4.2. The
launch is unblocked.
