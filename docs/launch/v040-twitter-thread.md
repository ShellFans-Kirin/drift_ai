# Twitter / X Thread — v0.4.0

7 tweets. ≤ 280 chars each. Replace `<gif-url>` with the GitHub asset
URLs after release.yml uploads them.

## 1/ Hook

> Codex stalled mid-task? Claude rate-limited? Hand off in 10 seconds
> and pick up where you left off — with any LLM, any agent.
>
> drift v0.4 is out. Vendor-neutral handoff for AI coding tasks.
> github.com/ShellFans-Kirin/drift_ai
>
> 🎬 <gif-url for v040-handoff-bidirectional.gif>

## 2/ Multi-agent capture

> Capture sessions from any of:
> · Claude Code
> · Codex
> · Cursor (new!)
> · Aider (now full)
>
> Hand off between any pair. Source × target = 16 combinations,
> all from one binary.
>
> 🎬 <gif-url for v040-cursor-handoff.gif>

## 3/ Multi-LLM cost

> Same handoff brief, four LLMs:
>
> · Anthropic Haiku    $0.00133
> · OpenAI gpt-4o-mini $0.00015
> · Gemini 2.5-flash   $0.00019
> · DeepSeek           $0.00023
>
> DeepSeek = ~30× cheaper than Opus at similar narrative quality.
>
> 🎬 <gif-url for v040-multi-llm-comparison.gif>

## 4/ Vendor-neutral by design

> Native: Anthropic / OpenAI / Gemini / Ollama
> Generic OpenAI-protocol: DeepSeek / Groq / Mistral / Together / vLLM
> / LM Studio
>
> Switch the LLM behind drift handoff by editing one line in
> .prompts/config.toml. No code change, no rebuild.

## 5/ Install

> brew install ShellFans-Kirin/drift/drift
>
> or
>
> cargo install drift-ai
>
> macOS arm64+x86_64, Linux arm64+x86_64. Rust 1.85+ if you build from
> source. v0.4.0 has 122 tests + real-API smoke against 4 cloud LLMs.

## 6/ Roadmap

> v0.5 — secret redaction in capture, Cursor agent-mode, Bedrock /
>        Vertex / Azure OpenAI wrappers
> v0.6 — team handoff sync, per-vendor brief body translation
> v1.0 — schema + format stability guarantees
>
> Apache 2.0. Independent. Not affiliated with any vendor.

## 7/ Ask

> If your AI coding workflow has agent-switching pain, try drift v0.4
> and tell me where the abstraction breaks.
>
> Issues + ideas: github.com/ShellFans-Kirin/drift_ai/issues
>
> Special thanks to the Anthropic / OpenAI / Google / DeepSeek API
> teams for stable streaming protocols.
> @AnthropicAI @OpenAIDevs @GoogleAI
