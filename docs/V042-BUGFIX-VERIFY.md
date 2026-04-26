# v0.4.2 Bug Fix Verification

**Date**: 2026-04-26
**Commit**: `fcc3454` (originally) → re-verified on `v0.4.2` branch
**Tests added**: 2 regression tests (config + openai-compat)

## Bug 1 — `[handoff]` config overlay was ignored

**Symptom**: setting `[handoff].provider = "deepseek"` in
`.prompts/config.toml` had no effect; `drift handoff` always called
the global default Anthropic provider.

**Root cause**: `crates/drift-core/src/config.rs::load()` overlaid
`attribution / connectors / compaction / sync` from the project file
but skipped `handoff`. One missing line.

**Regression test**:
`config::tests::handoff_config_project_overlay_is_applied` —
constructs a project config with `provider = "deepseek"` + named
provider entry, calls `load()`, asserts `cfg.handoff.provider == "deepseek"`
and the providers map propagated.

```
test config::tests::handoff_config_project_overlay_is_applied ... ok
```

## Bug 2 — `OpenAICompatibleProvider::complete_async` returned cost = 0

**Symptom**: `drift handoff` against any OpenAI-protocol provider
(DeepSeek / Groq / Mistral / vLLM / LM Studio) reported `cost=$0.0000`
no matter how many tokens were used.

**Root cause**: `OpenAICompatibleProvider::complete_async` (used by
the handoff second-pass) delegated to the inner `OpenAIProvider`
without re-stamping the user-supplied per-1M-token rates. The inner
client used OpenAI's price table, missed the third-party model name,
fell to `0.0`. The `compact_async` path was already correct.

**Regression test**:
`compaction::openai_compat::tests::handoff_complete_re_stamps_user_pricing` —
builds a mock OpenAI server returning 1000 input + 200 output tokens,
provider configured with `cost_per_1m_input_usd = 0.27`,
`cost_per_1m_output_usd = 1.10`, asserts the returned `LlmCompletion.cost_usd`
equals `1000/1M * 0.27 + 200/1M * 1.10 = $0.00049`.

```
test compaction::openai_compat::tests::handoff_complete_re_stamps_user_pricing ... ok
```

## Real-API smoke (4 providers, same fixture)

Identical 4-turn fixture session, each provider configured via
`[handoff]` block in `.prompts/config.toml`:

| Model | Cost line (stderr from `drift handoff`) |
|---|---|
| claude-haiku-4-5 | `· model=claude-haiku-4-5 · in=1154 out=337 · cost=$0.0028` |
| gpt-4o-mini | `· model=gpt-4o-mini · in=1026 out=196 · cost=$0.0003` |
| gemini-2.5-flash | `· model=gemini-2.5-flash · in=1158 out=258 · cost=$0.0003` |
| deepseek-chat | `· model=deepseek-chat · in=1033 out=178 · cost=$0.0005` |

All four:
- Reported the *correct* model name (Bug 1 fix — provider switching takes effect)
- Reported a *non-zero* cost (Bug 2 fix — OpenAICompatible re-stamps DeepSeek pricing)

DeepSeek vs Anthropic Opus default for handoff (`claude-opus-4-7` at
$15/$75 per 1M tokens) is roughly 30× cheaper at similar narrative
quality.

## Status

🟢 Both v0.4.2 ship-blocker bugs verified fixed via unit test + real
API smoke.
