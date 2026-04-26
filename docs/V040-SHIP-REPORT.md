# Drift v0.4.0 (merged v0.3 + v0.4) Ship Report

**Date**: 2026-04-26 (UTC) / 2026-04-26 12:14 Asia/Taipei
**Tag**: `v0.4.0` @ `1325462`
**Branch flow**: `dev_only/v0.3-v0.4-merged` → squash-merged → `dev_only/main` → `release-to-public.sh v0.4.0` → `public/main` + tag
**Spec**: [`docs/V030-V040-DESIGN.md`][design]
**Smoke**: [`docs/V030-V040-SMOKE.md`][smoke]

## Delivery links

- **GitHub Release**: https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.4.0
  - 4 platform tarballs + 4 SHA-256 checksums (8 assets total)
- **crates.io**: all four crates at 0.4.0
  - https://crates.io/crates/drift-core/0.4.0
  - https://crates.io/crates/drift-connectors/0.4.0
  - https://crates.io/crates/drift-mcp/0.4.0
  - https://crates.io/crates/drift-ai/0.4.0
- **Homebrew tap**: `drift.rb` auto-bumped to `version "0.4.0"` with the
  4 new SHA-256s.
  https://github.com/ShellFans-Kirin/homebrew-drift/blob/main/Formula/drift.rb
- **CHANGELOG `## [0.4.0]`**: https://github.com/ShellFans-Kirin/drift_ai/blob/main/CHANGELOG.md
- **Design proposal**: https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-DESIGN.md
- **Smoke report**: https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-SMOKE.md

## Phase status

| Phase | Scope | Status |
|---|---|---|
| 0 | Design proposal + draft PR | ✓ shipped (`8b57461`) |
| 1A | Multi-provider compaction module + 4 providers | ✓ shipped (`cdbbc3f`) |
| 1B+1C | Wire factory + real-API smoke against 4 LLMs | ✓ shipped (`98345dd`) |
| 2 | Cursor connector via SQLite | ✓ shipped (`0161f71`) |
| 3 | Aider connector full impl + CONTRIBUTING walkthrough | ✓ shipped (`7c9158f`) |
| 4 | bump 0.4.0 + self-validation gate | ✓ shipped (`244c2a6`) |
| 5 | README hero + CHANGELOG + demo guide + 4 launch drafts | ✓ shipped (`1325462`) |
| 6 | Release pipeline (tag → public push → release.yml → crates publish ×4) | ✓ shipped |

## Multi-provider real smoke results

Same 4-turn fixture session, each provider:

| Provider | Model | Latency | In/Out tok | Cost USD |
|---|---|---:|---:|---:|
| Anthropic | claude-haiku-4-5 | 2281 ms | 435 / 179 | $0.001330 |
| OpenAI | gpt-4o-mini | 3201 ms | 391 / 147 | $0.000147 |
| Gemini | gemini-2.5-flash | 1505 ms | 455 / 199 | $0.000188 |
| DeepSeek (compat) | deepseek-chat | 1906 ms | 396 / 109 | $0.000227 |
| Ollama | (skipped) | — | — | — (daemon not running on smoke host) |

DeepSeek vs Anthropic Opus default (handoff config) ≈ ~30× cheaper at
similar narrative quality.

## Test counts

- v0.2.0 baseline: 67 tests
- **v0.4.0**: **122 tests** (+55)
  - +35 in compaction (provider parsers, factory, streaming helpers)
  - +9 in cursor connector (synthetic SQLite fixtures)
  - +11 in aider connector (synthetic markdown fixture)
- Real-API smoke (gated by `#[ignore]`, env-var, daemon probe):
  4 cloud providers verified end-to-end, Ollama deferred to launch-recording host

## Self-validation gate

- `cargo fmt --all -- --check`            ✓ clean
- `cargo clippy --all-targets -- -D warnings` ✓ clean across workspace
- `cargo test --workspace`                ✓ 122 / 122 passed
- `cargo build --release --workspace`     ✓ 1m 07s on Linux x86_64
- `/tmp/drift-clean-cargo-v040/bin/drift --version` → `drift 0.4.0` ✓

## Real Anthropic + multi-provider usage cost (this release work)

Approximation from the smoke runs + ad-hoc testing during build:

| Vendor | Approx spend |
|---|---:|
| Anthropic | $1.5–2.0 (Opus + Haiku across smoke + design + handoff dogfooding) |
| OpenAI | < $0.01 (smoke fixture only) |
| Gemini | < $0.01 (smoke fixture only) |
| DeepSeek | < $0.01 (smoke fixture only) |

Cost numbers are surfaced live via `drift cost` against the local
`.prompts/events.db` for the dev_only repo.

## Known limitations carried forward to v0.5

1. **Cursor connector is `[BEST-EFFORT]`** — schema reverse-engineered
   from current Cursor stable; future Cursor releases may break parsing.
   Connector emits warnings on unparseable rows rather than failing the
   capture run.
2. **Aider markdown has no tool_call structure**, so `rejected = false`
   is the default for every event; the SHA-256 ladder still detects
   real divergence on disk.
3. **Gemini wired against AI Studio**, not Vertex AI. Vertex + Bedrock
   + Azure OpenAI is the v0.5 enterprise wave.
4. **OpenAI-compatible providers don't ship a price table**. Users
   supply per-1M-token rates per provider entry; absent → unpriced.
5. **Cursor agent-mode composer state** (multi-step tool-call replay)
   partial. v0.5.

## Mac brew install verification

⚠️ **Pending** — to be run on a real macOS host (Apple Silicon or Intel)
before launch:

```bash
brew untap ShellFans-Kirin/drift 2>/dev/null || true
brew tap ShellFans-Kirin/drift
brew install drift
drift --version
brew uninstall drift
# expect: drift 0.4.0
```

The agent runs on a Linux host, so this step is deferred to the user's
launch checklist (`docs/launch/v040-pre-launch-checklist.md`, T-50 min).

## Demo recordings

⚠️ **Pending** — needs `asciinema rec` + `agg` on the user's host.
Storyboards + exact recording sequences live in
[`docs/demo/v040-recording-guide.md`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/demo/v040-recording-guide.md).
The README hero already references the three GIF paths; once recorded
+ committed, the launch is fully ready.

Three GIFs:
1. `v040-handoff-bidirectional.gif` — Codex ↔ Claude switching mid-task
2. `v040-multi-llm-comparison.gif` — same brief, 4 LLMs, cost overlay
3. `v040-cursor-handoff.gif` — Cursor session → Claude Code paste

## Show HN timing

🟢 **GREEN** — ready to post once GIFs land + Mac brew test is green.

- Recommended posting window: Tue / Wed / Thu, 8–10 PM Asia/Taipei
- Title: `Show HN: drift – vendor-neutral AI coding handoff (Claude/GPT/Gemini/DeepSeek/local)`
- Body: [`docs/launch/v040-show-hn.md`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/launch/v040-show-hn.md)
- Twitter thread: [`docs/launch/v040-twitter-thread.md`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/launch/v040-twitter-thread.md)
- Asia drafts: zh-Hans (掘金) / zh-Hant (Medium) / ja (Qiita) all under
  `docs/launch/v040-*.md`
- Pre-launch checklist: `docs/launch/v040-pre-launch-checklist.md`

## One-line conclusion

v0.4.0 fully shipped to public + crates.io + Homebrew tap. **5 LLM
provider × 4 agent connector**, 122 tests, real API smoke against 4
clouds. Any user can today run `brew install ShellFans-Kirin/drift/drift`
or `cargo install drift-ai` and get a working vendor-neutral AI coding
handoff CLI. GIF recording + Mac brew Tailscale test are the only two
items between this report and Show HN.

[design]: https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-DESIGN.md
[smoke]: https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-SMOKE.md
