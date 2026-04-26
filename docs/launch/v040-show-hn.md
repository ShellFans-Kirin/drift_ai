# Show HN draft — v0.4.0 (merged v0.3 + v0.4)

**Status**: draft, not posted. Post timing recommendation in
`v040-pre-launch-checklist.md`.

## Title

`Show HN: drift – vendor-neutral AI coding handoff (Claude/GPT/Gemini/DeepSeek/local)`

(Two backups, in case the title doesn't land:)
- `Show HN: drift – when your AI coding agent stalls, hand off to another in 10s`
- `Show HN: drift – cross-agent + cross-LLM handoff for AI coding tasks`

## Body

I built **drift** because Codex kept stalling on me mid-task and pasting
chat history into Claude was a mess — the new agent always re-litigated
decisions I'd already made. v0.4 ships today and it's the first version
that's *actually* vendor-neutral.

`drift handoff` packages an in-progress AI coding session into a
markdown brief any LLM can absorb cold:

```bash
$ drift handoff --branch feature/oauth --to claude-code
⚡ scanning .prompts/events.db
⚡ extracting file snippets and rejected approaches
⚡ compacting brief via claude-opus-4-7
✅ written to .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md
```

The brief lists what you've decided, what you tried and rejected,
what's still open, and where to resume. Paste it into the next agent
and they pick up mid-task without you re-explaining.

### What's new in v0.4 (merged v0.3 + v0.4 release)

**Multi-LLM provider** — the LLM doing the brief is now configurable:
- Native: Anthropic, OpenAI (gpt-5/4o/o1/o3), Gemini (AI Studio),
  Ollama (local)
- Generic OpenAI-protocol: DeepSeek, Groq, Mistral, Together AI,
  LM Studio, vLLM — anything that speaks `chat.completions`

Cost spread on the same fixture session:

| Provider | Model | Cost (USD) |
|---|---|---:|
| Anthropic | claude-haiku-4-5 | $0.00133 |
| OpenAI | gpt-4o-mini | $0.00015 |
| Gemini | gemini-2.5-flash | $0.00019 |
| DeepSeek (compat) | deepseek-chat | $0.00023 |

DeepSeek is **~30× cheaper than Opus** at similar narrative quality.
Real-API smoke results in [`docs/V030-V040-SMOKE.md`][smoke].

**Multi-agent connector** — drift now reads sessions from:
- Claude Code (since v0.1)
- Codex (since v0.1)
- **Cursor** (new) — parses Cursor's per-workspace SQLite
- **Aider** (new) — parses `.aider.chat.history.md`, including
  ` ```diff ` fenced blocks

So you can capture a Cursor session and hand off to Claude Code, or
capture a Claude Code session and hand off to Aider. Any source × any
target.

### What's it really doing?

drift is built on top of a v0.1 attribution engine that watches the
local session JSONL your AI coding agents already write under
`~/.claude/projects/`, `~/.codex/sessions/`,
`~/Library/Application Support/Cursor/`, and `<repo>/.aider.chat.history.md`.

It LLM-compacts each session into a small markdown record, stores the
result in `.prompts/` inside your git repo, and binds each session to
its matching commit via `git notes`. So:

- `drift blame src/auth.ts --line 42` → which session/agent/prompt
  produced this line, plus the human edits on top.
- `drift trace <session-id>` → forward lookup: this session changed
  these files in this order.
- `drift log` → `git log` with per-agent attribution under each commit.
- `drift handoff` → cross-agent task transfer (the headline of v0.4).

Local-first by design. The only network egress is the optional
Anthropic / OpenAI / Gemini / DeepSeek compaction call — turn it off
with `[compaction].provider = "mock"` or `"ollama"` if you don't want
LLM summarisation in the loop.

### What it's not

- Not a chat client. Doesn't replace Cursor's UI or Claude Code's REPL.
- Not a privacy product. Default config commits `events.db` to git for
  team-shared blame; flip `[attribution].db_in_git = false` to keep it
  local. v0.5 ships a regex-based redaction pass.
- Not affiliated with Anthropic, OpenAI, Google, DeepSeek, or any
  other vendor it integrates with. Apache 2.0, [code on GitHub][repo].

### Install

```bash
# Homebrew (mac arm64/x86_64, Linux arm64/x86_64)
brew install ShellFans-Kirin/drift/drift

# crates.io (Rust 1.85+)
cargo install drift-ai

# pre-built binary
curl -sSfL https://github.com/ShellFans-Kirin/drift_ai/releases/latest/download/drift-v0.4.0-$(uname -m)-unknown-linux-gnu.tar.gz \
  | tar xz -C /tmp && sudo mv /tmp/drift /usr/local/bin/drift
```

### Roadmap (rough)

- **v0.5**: Regex-based secret redaction in `drift capture`. Cursor
  agent-mode (multi-step tool-call replay). Bedrock / Vertex AI / Azure
  OpenAI native wrappers.
- **v0.6**: Team-handoff (notes sync UX). Per-vendor body translation
  (tool-call schema adapter for handoff).
- **v1.0**: Stable schema + format guarantees beyond just `events.db`.

### Why open-source

I dogfood drift every day — the v0.4 work itself was a Codex × Claude
Code handoff loop with drift catching the context. The data model
matters more than any one tool's UI; keeping it open + Apache means
nobody owns your prompt history but you.

Code: https://github.com/ShellFans-Kirin/drift_ai
Vision: https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/VISION.md
Smoke: https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-SMOKE.md

Issues + ideas welcome. Roast me on the bits where the abstraction
doesn't yet pay off.

[smoke]: https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-SMOKE.md
[repo]: https://github.com/ShellFans-Kirin/drift_ai
