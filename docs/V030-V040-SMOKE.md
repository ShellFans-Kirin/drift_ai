# Drift AI v0.3+v0.4 Real-API Smoke Report

**Date**: 2026-04-25
**Branch**: `v0.3-v0.4-merged`
**Phase**: 1C (real-API verification of multi-provider compaction)
**Harness**: `crates/drift-core/tests/v030_real_smoke.rs` — gated `#[ignore]`,
run with `cargo test --test v030_real_smoke -- --ignored --nocapture`.

## Fixture session

A 4-turn AI-coding session: user asks for a rate limiter, assistant implements
sliding-window approach (rejecting per-IP token bucket with rationale), user
asks for tests, assistant adds them. ~440 prompt tokens, expected ~150–200
output tokens.

## Provider × model results

| Provider | Model | Latency | In tok | Out tok | Cost USD | Output preview |
|---|---|---:|---:|---:|---:|---|
| anthropic | claude-haiku-4-5 | 2281 ms | 435 | 179 | $0.001330 | Added a sliding window rate limiter to the login route with a 5 attempts per min… |
| openai | gpt-4o-mini | 3201 ms | 391 | 147 | $0.000147 | Implemented a sliding window rate limiter for the login route in `src/auth/login… |
| gemini | gemini-2.5-flash | 1505 ms | 455 | 199 | $0.000188 | The assistant implemented a sliding window rate limiter on the login route (`src… |
| deepseek (compat) | deepseek-chat | 1906 ms | 396 | 109 | $0.000227 | # Summary Added a sliding-window rate limiter to the login route (5 attempts/min… |
| ollama | (skipped) | — | — | — | — | daemon not running on this host |

**All four cloud providers returned coherent markdown summaries reconstructing
the session faithfully, including the rejected approach.** Token counts are
within the expected envelope; cost numbers match the built-in / user-supplied
price tables.

## Cost spread observation (v0.4 launch narrative)

For the same fixture session at roughly the same output length:

| Provider | Cost (USD) | × cheaper than Haiku |
|---|---:|---:|
| anthropic claude-haiku-4-5 | $0.001330 | 1× (baseline) |
| openai gpt-4o-mini | $0.000147 | ~9× cheaper |
| gemini gemini-2.5-flash | $0.000188 | ~7× cheaper |
| deepseek deepseek-chat (via OpenAI-compatible) | $0.000227 | ~6× cheaper |

The bigger spread comes when comparing the **default** Anthropic model
`claude-opus-4-7` (used by `drift handoff`) against the cheap alternatives —
roughly 30× ratio at the Opus end. We don't run Opus in this smoke (cost +
rate limit on free dev keys); the multiplier is documented in
`docs/V030-V040-DESIGN.md` §1.3 and validated by the in-table prices.

## Notable issues + fixes shipped during smoke

### CRLF-style SSE delimiters (Gemini)

First Gemini smoke returned 0 tokens despite HTTP 200 + valid response. Root
cause: `streaming::for_each_sse_data` only matched `\n\n` event boundaries.
Gemini's `streamGenerateContent` endpoint emits `\r\n\r\n`. Fixed in
`compaction/streaming.rs::find_event_boundary` to handle both. Anthropic /
OpenAI / DeepSeek stayed working throughout.

### Gemini 2.5 hidden reasoning tokens

Initial probe of `gemini-2.5-flash` returned `"thoughtsTokenCount": 18` with
zero visible text — the model spent its entire output budget on hidden
reasoning when `thinkingConfig` was unspecified. Fixed by passing
`thinkingConfig: { thinkingBudget: 0 }` in `GeminiProvider::complete_async`,
forcing all output tokens to land in the visible `parts[0].text`. Documented
in the provider docstring.

### Gemini 2.5-pro free-tier rate limits

`gemini-2.5-pro` rate-limited (`429 RateLimited`) on the dev key. Smoke
target switched to `gemini-2.5-flash`. Pro stays available as a config
option; users with paid keys get the upgrade for free.

## Skipped: Ollama

`curl -sf http://localhost:11434/api/tags` failed (no daemon running). The
`OllamaProvider::compact_async` path is verified by the `tokio_test` mocks
in `compaction/ollama.rs` — see the `happy_path_ndjson` /
`handles_large_content_split_across_chunks` cases. Real-host smoke deferred
to launch-time recording (`docs/demo/v040-multi-llm-comparison.gif` will
include Ollama if available).

## Reproducibility

```bash
source ~/.drift-env.sh   # exports ANTHROPIC_API_KEY / OPENAI_API_KEY /
                         # GEMINI_API_KEY / DEEPSEEK_API_KEY
cargo test --test v030_real_smoke -- --ignored --nocapture --test-threads=1
```

Each test self-skips when its env var is absent, so partial coverage works
(DeepSeek-only run is fine for verifying the OpenAI-compatible plumbing).

## Conclusion

🟢 Phase 1 multi-provider implementation **verified end-to-end against four
real cloud LLM APIs** (Anthropic / OpenAI / Gemini / DeepSeek). Cost
calculation correct in all four cases. The OpenAI-compatible generic provider
proves out: DeepSeek runs via the same `OpenAICompatibleProvider` path as
the future Groq / Mistral / Together / vLLM users will use, with no new
per-vendor code.

Phase 1 ready to commit + checkpoint to `dev_only`.
