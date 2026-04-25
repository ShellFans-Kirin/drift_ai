# X / Twitter thread — drift v0.2.0

**Draft. Do not post.** Sequence below; tweak punctuation closer to
launch day so it doesn't read like AI output.

---

## 1/ — Hook

> Your AI coding agent stalled. Refused, rate-limited, got dumb.
> 
> You have 30 minutes of context to transfer to another agent.
> 
> drift handoff packages it into a brief any LLM picks up cold.
>
> [GIF: docs/demo/v020-handoff.gif]

---

## 2/ — What it produces

> Run `drift handoff --branch feature/oauth --to claude-code`. drift
> reads your local Claude/Codex session log, pulls out:
> 
> • what you've decided
> • what you tried and rejected
> • files in scope (with line-level diff context)
> • next steps
> 
> One markdown file. Paste it into the new agent.

---

## 3/ — Why this is hard without drift

> Re-pasting chat history doesn't work. The new agent doesn't know
> which decisions are settled, which approaches you already rejected,
> or which file you're halfway through.
> 
> "Continue from where I left off" means giving them STRUCTURE, not raw
> transcript.

---

## 4/ — The pieces

> Built in Rust on a v0.1 attribution layer:
> 
> • SQLite events.db inside your repo (local-first, no server)
> • watches Claude Code + Codex session logs
> • LLM-compacted summaries via Anthropic API
> • single-binary install via brew or cargo
> • Apache-2.0
> 
> The handoff command is the new (v0.2) wedge.

---

## 5/ — Install

> brew install ShellFans-Kirin/drift/drift
> 
> or 
> 
> cargo install drift-ai
> 
> Then:
> 
> drift init
> drift capture            # pulls existing sessions
> drift handoff --branch feat-x --to claude
> 
> Done.

---

## 6/ — Roadmap (v0.3+)

> • Ollama / vLLM for fully-local handoff
> • Cursor + Cline + custom-agent connector PRs welcome
> • Cross-agent prompt-schema translation (tool-call adapters)
> • Team handoff (sanitised cross-developer brief)
> • Regex-based secret redaction in capture
> 
> Ranking by you, not me — what'd unblock you most?

---

## 7/ — Independent project

> drift is built by me, not Anthropic / not OpenAI / not any agent
> vendor. It runs on top of THEIR session logs, but it's not part of
> their products.
> 
> If you've ever lost 20 minutes re-explaining context to a new agent,
> this is the tool I wished existed.
> 
> https://github.com/ShellFans-Kirin/drift_ai
