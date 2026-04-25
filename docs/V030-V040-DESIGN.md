# Drift AI v0.3 + v0.4 Merged Design Proposal

**Status**: Phase 0 design proposal — awaiting reviewer approve to start Phase 1.
**Author**: ShellFans-Kirin
**Date**: 2026-04-25
**Target ship**: v0.4.0 (single merged release; no intermediate v0.3.0)

---

## 0. Why merge v0.3 and v0.4

Originally v0.3 (multi-LLM provider) and v0.4 (multi-agent connector) were
planned as separate releases. We merge them because:

1. **Engineering velocity in 2026**. With AI-assisted coding, the realistic
   delivery for both is ~3-5 working days — short enough to land as one
   release without inflating cycle time.
2. **Launch narrative concentration**. A single "Show HN: vendor-neutral
   AI coding handoff (Claude/GPT/Gemini/DeepSeek/local)" lead is sharper
   than two diluted v0.3 / v0.4 launches a week apart.
3. **Real vendor-neutrality only makes sense when both axes are in**.
   Multi-LLM without multi-agent (or vice versa) is half a story; the
   product claim is "any agent × any model" and shipping half is leakage.
4. **No blocking dependencies between the two**. Provider work and
   connector work touch disjoint modules (`compaction/` vs `connectors/`).
   Parallel-friendly, low merge-conflict risk.

We still **commit and push at every phase boundary** (Phase 1 / 2 / 3) so the
merged release is composed of reviewable chunks, not a wall of changes.

---

## 1. Multi-provider architecture (v0.3 part)

### 1.1 Trait design

The existing `CompactionProvider` trait (v0.2) is **unchanged**. We add four
native providers and one generic:

```rust
// Existing
pub struct AnthropicProvider { ... }      // v0.2 — unchanged path
// New
pub struct OpenAIProvider { ... }         // gpt-5 / gpt-4o / o1-series
pub struct GeminiProvider { ... }         // gemini-2.5-pro / -flash
pub struct OllamaProvider { ... }         // local LLM
pub struct OpenAICompatibleProvider {     // generic OpenAI-protocol client
    base_url: String,
    api_key: Option<String>,              // None for keyless local servers
    model: String,
    cost_per_1m_input_usd: Option<f64>,   // user-supplied (we don't track third-party prices)
    cost_per_1m_output_usd: Option<f64>,
}
```

`OpenAICompatibleProvider` is the wedge that absorbs the long tail
(DeepSeek, Groq, Mistral, Together AI, Fireworks, LM Studio, vLLM) without
us writing a per-vendor client. They all already speak the OpenAI HTTP
protocol — we just configure base URL, key, and model.

### 1.2 Config schema evolution

Default `[handoff].provider = "anthropic"` is preserved (backwards-compat
for every v0.2 user). Multi-provider is **opt-in**:

```toml
[handoff]
provider = "anthropic"  # default unchanged

# --- Native providers (uncomment one to switch) ---

# [handoff.providers.openai]
# model = "gpt-5"
# api_key_env = "OPENAI_API_KEY"

# [handoff.providers.gemini]
# model = "gemini-2.5-pro"
# api_key_env = "GEMINI_API_KEY"

# [handoff.providers.ollama]
# base_url = "http://localhost:11434"
# model = "llama3.3:70b"

# --- OpenAI-compatible providers (uncomment one + set env) ---

# [handoff.providers.deepseek]
# type = "openai_compatible"
# base_url = "https://api.deepseek.com"
# model = "deepseek-chat"
# api_key_env = "DEEPSEEK_API_KEY"
# cost_per_1m_input_usd = 0.27
# cost_per_1m_output_usd = 1.10

# [handoff.providers.groq]
# type = "openai_compatible"
# base_url = "https://api.groq.com/openai/v1"
# model = "llama-3.3-70b-versatile"
# api_key_env = "GROQ_API_KEY"
# cost_per_1m_input_usd = 0.59
# cost_per_1m_output_usd = 0.79

# [handoff.providers.mistral]
# type = "openai_compatible"
# base_url = "https://api.mistral.ai/v1"
# model = "mistral-large-latest"
# api_key_env = "MISTRAL_API_KEY"

# [handoff.providers.together]
# type = "openai_compatible"
# base_url = "https://api.together.xyz/v1"
# model = "meta-llama/Llama-3.3-70B-Instruct-Turbo"
# api_key_env = "TOGETHER_API_KEY"

# [handoff.providers.lmstudio]
# type = "openai_compatible"
# base_url = "http://localhost:1234/v1"
# model = "local-model"
# # api_key_env not set — LM Studio is keyless

# [handoff.providers.vllm]
# type = "openai_compatible"
# base_url = "http://localhost:8000/v1"  # user fills in
# model = "your-model"
```

Switching is a single `provider = "deepseek"` (or `"openai"`, etc) edit + an
env var. No code change.

### 1.3 Same mechanism applied to `[compaction]`

Section parallel to `[handoff]`:

```toml
[compaction]
provider = "anthropic"  # default

# [compaction.providers.deepseek]
# type = "openai_compatible"
# ... same shape as handoff
```

`drift capture` and `drift handoff` resolve their providers independently.
You can run capture on cheap Haiku and handoff on Opus, or vice versa.

### 1.4 Cost tracking

`compaction_calls` table schema **unchanged** (the `model` column is already
free-text). Each provider returns `(input_tokens, output_tokens, cost_usd)`
on completion:

| Provider | Token source | Cost source |
|---|---|---|
| Anthropic | `usage` block in `message_stop` SSE event | built-in price table |
| OpenAI | `usage` block in final SSE chunk; reasoning tokens (o1/o3) folded into output | built-in price table |
| Gemini | `usageMetadata` in final response | built-in price table |
| Ollama | `eval_count` / `prompt_eval_count` in NDJSON final event | always $0 |
| OpenAICompatible | `usage` block (OpenAI shape) | **user-supplied** in config |

We deliberately **do not** ship a third-party price table for OpenAI-compatible
providers. DeepSeek / Groq / Mistral / Together change pricing more often than
we ship; the user's config is the source of truth. If pricing fields are
absent, `cost_usd = 0.0` and `drift cost` shows `(unpriced)`.

### 1.5 Streaming

| Provider | Wire format | Parser |
|---|---|---|
| Anthropic | SSE (`text/event-stream`) | existing `streaming::sse_parser` |
| OpenAI | SSE | shared `streaming::sse_parser` (compatible shape) |
| Gemini | SSE (different event names) | new `streaming::gemini_sse_parser` |
| Ollama | NDJSON (one JSON per line) | new `streaming::ndjson_parser` |
| OpenAICompatible | SSE (OpenAI-shape mandate) | shared `streaming::sse_parser` |

### 1.6 Error mapping

Each provider maps wire errors to the existing `CompactionError` enum (no
enum changes — guarantees v0.2 callers keep working):

| Wire | `CompactionError` |
|---|---|
| 401 | `AuthInvalid` |
| 429 | `RateLimited { retry_after: Option<Duration> }` |
| 404 model | `ModelNotFound` |
| 400 context length | `ContextTooLong` |
| 5xx / network / timeout | `TransientNetwork` |
| stream parse failure | `Stream` |
| anything else | `Other(String)` |

Retry policy (5× for 429 honouring `Retry-After`, 4× exp-backoff for 5xx)
moves into `compaction/retry.rs` shared by all providers.

---

## 2. Cursor connector (v0.4 part)

### 2.1 Where Cursor stores sessions

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/Cursor/User/workspaceStorage/<hash>/` |
| Linux | `~/.config/Cursor/User/workspaceStorage/<hash>/` |
| Windows | `%APPDATA%\Cursor\User\workspaceStorage\<hash>\` |

Each `<hash>` is one workspace. Inside:
- `state.vscdb` — SQLite, holds chat / composer / edits
- supplementary JSON files

### 2.2 SQLite schema (reverse-engineered, undocumented)

The single relevant table:

```
CREATE TABLE cursorDiskKV (
  key   TEXT PRIMARY KEY,
  value BLOB
);
```

Keys use reverse-domain notation. The ones we care about:

| Key prefix | Holds |
|---|---|
| `composerData:<id>` | composer chat session (messages + edits) |
| `aiService.prompts` | prompt history |
| `cursorPanelView:<id>` | sidebar chat sessions |

Each `value` is JSON. Composer JSON shape (typed from real fixtures):

```json
{
  "composerId": "...",
  "messages": [
    { "role": "user", "content": "...", "timestamp": ... },
    { "role": "assistant", "content": "...", "edits": [...] }
  ],
  "edits": [
    { "filePath": "...", "before": "...", "after": "...", "status": "accepted|rejected" }
  ],
  "createdAt": ..., "updatedAt": ...
}
```

### 2.3 Parse strategy

```rust
let conn = rusqlite::Connection::open_with_flags(
    state_vscdb,
    OpenFlags::SQLITE_OPEN_READ_ONLY,
)?;
let mut stmt = conn.prepare("SELECT key, value FROM cursorDiskKV WHERE key LIKE 'composerData:%'")?;
for row in stmt.query_map([], ...) {
    let (key, value): (String, Vec<u8>) = ...;
    let session: CursorSession = serde_json::from_slice(&value)?;
    yield normalize(session);
}
```

### 2.4 Known limitations (documented in `[BEST-EFFORT]` doc string)

- Schema is **undocumented**; reverse-engineered from current Cursor
  stable (early 2026). Future Cursor versions may rename keys or change
  JSON shape.
- We tag the connector `[BEST-EFFORT]` and emit a warning on parse error
  rather than failing the whole `drift capture` run.
- v0.4 covers **chat history (composer + sidebar)**. The richer "agent
  mode" composer state (multi-step tool calls) is partial — we extract
  the messages but not the intermediate state machine. Full agent-mode
  support is deferred to v0.5.
- No write-back to Cursor DB. Read-only forever; we won't risk corrupting
  the user's chat history.

### 2.5 Agent slug + first-class status

`agent_slug = "cursor"` joins `claude-code` / `codex` as a first-class
connector (default-enabled, exposed in `--agent` filter, listed in MCP
tool responses).

After Phase 3 completes, the first-class roster is:
`claude-code` / `codex` / `cursor` / `aider` (4 first-class).

---

## 3. Aider full connector (v0.4 part)

### 3.1 Aider session format

Aider stores conversation history as plain markdown:

- `<repo>/.aider.chat.history.md` — turn-by-turn user/assistant log
- `<repo>/.aider.input.history` — raw CLI prompts (we ignore)

Aider also commits directly to git with subject prefix `aider:` (e.g.
`aider: add OAuth login`), so we can correlate commits ↔ sessions via
commit message + timestamp.

### 3.2 Parse strategy

1. Walk `<repo>` for `.aider.chat.history.md`.
2. Markdown parse into turns using Aider's heading pattern (`# user` /
   `# assistant`) — or fall back to a regex if heading style varies.
3. For each assistant turn, find ` ```diff ` fenced blocks → extract
   `+/-` hunks → produce `CodeEvent`.
4. Correlate to git commits by walking `git log --grep="^aider:"` and
   matching by timestamp window (assistant turn time ± 60s of commit).

### 3.3 Limitations (documented in `[BEST-EFFORT]` doc string)

- Aider has no `tool_call` / `tool_result` structure — every assistant
  turn is just markdown text. We can't reliably distinguish "this diff
  was applied" from "this diff was suggested but rejected".
  → **All Aider events default to `rejected = false`** (i.e. assumed
  accepted). The SHA-256 ladder will catch divergences and re-attribute
  to `human`.
- Aider session has no stable session ID. We synthesise:
  `session_id = sha256(file_path + first_turn_timestamp)`.
- Multi-repo `.aider.chat.history.md` would conflict on the synthesised
  ID; v0.4 documents this and hashes file path absolutely.

### 3.4 CONTRIBUTING.md walkthrough

The aider stub-to-full PR diff becomes the worked example in the
"Adding a new connector" section of `CONTRIBUTING.md`. Replaces the
current placeholder.

---

## 4. Per-`--to` agent footer upgrades

`drift handoff --to <target>` already varies the brief footer per target.
v0.4 adds two new targets:

| `--to` | Footer style |
|---|---|
| `claude-code` (existing) | "paste this to claude" + `claude` CLI hint |
| `codex` (existing) | codex-style resume prompt |
| `generic` (existing) | no footer |
| `cursor` (new) | "paste into Cursor composer; Ctrl/Cmd+I to apply" |
| `aider` (new) | "paste at aider prompt; aider will read referenced files" |

Body content is identical across targets — only the footer changes.
Per-vendor body translation (tool-call schema adapter) is explicitly v0.5+.

---

## 5. Testing strategy

### 5.1 Unit tests

| Module | Min new tests |
|---|---|
| `compaction/openai.rs` | 5 (SSE parse, retry-on-429, error map, cost calc, stream-cut) |
| `compaction/gemini.rs` | 5 |
| `compaction/ollama.rs` | 5 (NDJSON parse) |
| `compaction/openai_compat.rs` | 5 (incl. user-supplied cost) |
| `connectors/cursor.rs` | 8 (fixture SQLite, multi-workspace, missing schema, parse failure recovery) |
| `connectors/aider.rs` | 6 (fixture markdown, diff extraction, git commit correlation) |
| Integration | 5 (provider+connector round-trip via mock servers) |

Target: **v0.2 had 67 tests → v0.4 ~120 tests** (+50ish).

### 5.2 Real smoke (gated by env vars)

| Provider | Gate | Action if missing |
|---|---|---|
| Anthropic | `[ -n "$ANTHROPIC_API_KEY" ]` | **fail** (must run) |
| OpenAI | `[ -n "$OPENAI_API_KEY" ]` | skip + report |
| Gemini | `[ -n "$GEMINI_API_KEY" ]` | skip + report |
| DeepSeek | `[ -n "$DEEPSEEK_API_KEY" ]` | skip + report |
| Ollama | `curl -sf http://localhost:11434/api/tags >/dev/null` | skip + report |

All smoke results captured in `docs/V030-V040-SMOKE.md` (cost, latency,
sample output for cross-provider quality eyeball).

### 5.3 Cursor / Aider connectors

- Unit tests use **synthetic fixtures** (we do not check in real Cursor
  state.vscdb because it contains user data).
- Integration: `tests/fixtures/cursor/build_fixture.rs` is a helper that
  produces a minimal SQLite at runtime; tests then read it.
- Aider fixture: `tests/fixtures/aider/.aider.chat.history.md` (curated,
  redacted, public-safe sample).

---

## 6. Demo strategy (merged launch)

Three demo GIFs under `docs/demo/`:

| GIF | Story | Length |
|---|---|---|
| `v040-handoff-bidirectional.gif` | Codex stalls → Claude resumes; Claude rate-limited → Codex picks up | ≤ 30s |
| `v040-multi-llm-comparison.gif` | Same scope brief generated by Claude / GPT / Gemini / DeepSeek side-by-side, with cost overlay (DeepSeek ~30× cheaper) | ≤ 45s |
| `v040-cursor-handoff.gif` | Real Cursor session → `drift capture` → `drift handoff --to claude-code` → paste into Claude Code → continue | ≤ 30s |

All recorded with asciinema → agg → optimised GIF. No fake mockups.

---

## 7. Launch positioning

### 7.1 Show HN title — three candidates

**A** (pain): `Show HN: drift – when your AI coding agent stalls, hand off to another in 10s`

**B** (multi-LLM): `Show HN: drift – vendor-neutral AI coding handoff (Claude/GPT/Gemini/DeepSeek/local)` ← **recommended**

**C** (scope): `Show HN: drift – cross-agent + cross-LLM handoff for AI coding tasks`

**Recommendation: B**. Strongest hook because (1) "vendor-neutral" is the
sharpest differentiator, (2) the parenthesised vendor list does the demo
in the headline, (3) including DeepSeek triggers Chinese-language tech
circles to reshare (zh-Hans / zh-Hant). A is too narrow on a single
pain; C is accurate but doesn't trigger curiosity.

### 7.2 Tagline

> Vendor-neutral handoff for AI coding tasks.
> Works with Claude, GPT, Gemini, DeepSeek, and any local LLM.
> Reads sessions from Claude Code, Codex, Cursor, and Aider.

### 7.3 Asia-language launch articles (drafts staged, not posted)

| Locale | Channel | Angle |
|---|---|---|
| zh-Hans | 掘金 (juejin.cn) | DeepSeek native + cost demo (30× cheaper) |
| zh-Hans | V2EX | Local-first + Ollama for sensitive code |
| zh-Hant | iThome 鐵人賽 / Medium | Vendor-neutral + handoff pain |
| ja | Qiita | 本地 Ollama + privacy + Claude Code 連携 |

Files end up at `docs/launch/v040-{zh-Hans-juejin,zh-Hans-v2ex,zh-Hant-ithome,ja-qiita}.md`.

---

## 8. Out of scope (explicit)

Items deliberately **not** in v0.4:

- Cursor agent-mode composer state (full multi-step tool-call replay).
  → v0.5.
- Per-vendor body translation (tool-call schema adapter for handoff body).
  Footer-only differentiation stays. → v0.5.
- Bedrock / Vertex AI / Azure OpenAI native wrappers (enterprise).
  → v0.5+.
- Ollama auto-detection + provider recommendation. → v0.5.
- Team handoff (notes sync UX). → v0.5+.
- Secret redaction in capture. Still relies on user discipline +
  pre-commit hook. → v0.5+.
- Windows native testing. macOS + Linux only in v0.4 CI; Windows path
  code lands but isn't gated.

---

## 9. Risks & open questions for reviewer

### Q1. Gemini API surface — AI Studio or Vertex AI?

**Answer**: AI Studio.
- Simpler auth (URL query API key vs Google Cloud auth flow).
- Individual-developer friendly (free tier exists).
- Vertex AI is the enterprise variant; defer to v0.5 when we add Bedrock /
  Azure OpenAI (the trio that maps to "enterprise managed model APIs").
- Wire-format compatibility: most Gemini features ship to AI Studio first
  anyway; Vertex is mostly auth + quota wrapping.

### Q2. OpenAI o1 / o3 reasoning tokens — how do we count them?

**Answer**: Fold reasoning_tokens into output_tokens for billing; surface
separately in `drift cost --by model` for transparency.
- Rationale: reasoning tokens *are* billed at output rate; from the
  user's wallet perspective they're indistinguishable.
- We add a single `reasoning_tokens` column to `compaction_calls`
  schema? **No** — schema is frozen. Stash inside metadata JSON column.

### Q3. DeepSeek default — V3 (chat) or R1 (reasoning)?

**Answer**: Default config template uses `deepseek-chat` (V3). R1 is
listed in the template but commented out with a "reasoning model, slower
+ pricier" note.
- Rationale: handoff briefs are narrative summarization, not deep
  reasoning. V3 is the right cost/quality sweet spot. R1 is for users
  who specifically want chain-of-thought.

### Q4. Cursor schema versioning — which Cursor versions do we test?

**Answer**: Latest stable as of merge time (early 2026). Doc string
includes version range last verified. Connector emits a warning + degrades
gracefully if schema diverges; user files an issue with `cursor --version`.
- We do not maintain a schema-version detection matrix in v0.4. That's
  a maintenance burden that grows with every Cursor release.

### Q5. Ollama default model recommendation — what do we put in the template?

**Answer**: `llama3.3:70b` for quality; comment that 8b is the lower-RAM
alternative.
- Rationale: handoff briefs benefit from a larger model's narrative
  coherence. 70b is a reasonable default for a developer with 64GB RAM
  + a recent Mac / 4090. The 8b note keeps low-RAM users from being
  stranded.

---

## 10. Stability guarantees carried from v0.1 / v0.2

Hard-frozen — touching any of these triggers self-validation failure:

- **`events.db` schema** is unchanged.
- **MCP tool list** (`drift_blame` / `drift_trace` / `drift_rejected` /
  `drift_log` / `drift_show_event`) is unchanged.
- **`SessionConnector` trait signature** is unchanged. New default-impl
  methods may be added; required methods stay identical.
- **`CompactionProvider` trait** is unchanged. Implementors are added,
  the trait itself isn't widened.
- **`CompactionError` enum** is unchanged. New error categories piggyback
  on `Other(String)`.
- The v0.1.2 first-run privacy notice still fires unchanged.

Upgrading from v0.2.x to v0.4 is a binary swap. No migration.

---

## 11. Phase plan + estimated effort

| Phase | Scope | Estimate | Checkpoint commit |
|---|---|---|---|
| 0 (this) | Design proposal + draft PR | 0.25 days | `docs(v0.3+v0.4): merged design proposal` |
| 1 | Multi-provider compaction | 1.5–2 days | `feat(v0.3): multi-provider compaction (anthropic/openai/gemini/ollama/openai-compatible)` |
| 2 | Cursor connector | 1–1.5 days | `feat(v0.4): cursor connector via SQLite parsing` |
| 3 | Aider full + CONTRIBUTING walkthrough | 0.5 days | `feat(v0.4): aider connector full implementation` |
| 4 | Self-validation gate (fmt/clippy/test/smoke/install) | 0.25 days | `chore(v0.4): pass full self-validation` |
| 5 | 3× demo GIFs + README hero rebuild + 4× launch drafts | 0.5 days | `docs(v0.4): demo + launch artifacts` |
| 6 | Release pipeline (tag, release.yml, crates.io ×4, brew via Tailscale) | 0.5 days | `release v0.4.0` (squash merge into public main) |

**Total: ~4.5–5.5 working days end-to-end.**

Phase boundaries are commit boundaries — every phase pushes to dev_only
even if the merged release is the only public artifact.

---

## 12. Approval gate

This document is the entire Phase 0 deliverable. After reviewer approve,
Phase 1–6 run without further interruption per the merged-task spec.

If the reviewer wants to adjust scope:

- Want to defer one of Cursor / Aider? → still ships v0.4.0 but smaller
  connector list; flag the deferred one as v0.5 work.
- Want to add a vendor not on the list (e.g. xAI Grok, Qwen, Cohere)? →
  add to OpenAICompatible config templates if they speak OpenAI protocol;
  otherwise assess for v0.5.
- Want a separate v0.3.0 ship before v0.4.0? → de-merge the launch,
  ship v0.3.0 with provider matrix + cost demo, hold connectors for
  v0.4.0 a week later. Costs ~2 launch events instead of 1.

Default: proceed with single merged v0.4.0 ship as specified.
