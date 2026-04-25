> 🌐 **English** · [日本語](README.ja.md) · [简体中文](README.zh-Hans.md) · [繁體中文](README.zh-Hant.md)

# drift_ai

[![crates.io](https://img.shields.io/crates/v/drift-ai.svg)](https://crates.io/crates/drift-ai)
[![CI](https://github.com/ShellFans-Kirin/drift_ai/actions/workflows/ci.yml/badge.svg)](https://github.com/ShellFans-Kirin/drift_ai/actions/workflows/ci.yml)

> Hand off your in-progress AI coding task between Claude, Codex, and
> whatever agent you switch to next. Local-first.

![drift handoff demo](docs/demo/v020-handoff.gif)

**The problem**: Your AI coding agent stalled — refused, rate-limited, or
just got dumb. Now you need to transfer 30 minutes of context to another
agent. Re-pasting a chat history doesn't work; the new agent doesn't
know which decisions are settled, which approaches you already rejected,
or which file you were halfway through.

**`drift handoff`** packages your in-progress task into a markdown brief
any LLM can absorb cold:

```bash
$ drift handoff --branch feature/oauth --to claude-code
⚡ scanning .prompts/events.db
⚡ extracting file snippets and rejected approaches
⚡ compacting brief via claude-opus-4-7
✅ written to .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md
```

The brief lists what you've decided, what you tried and rejected, what's
open, and where to resume. Paste it into the next agent and they pick up
mid-task without you re-explaining.

`drift` is built on top of a v0.1 attribution engine that watches the
local session logs of your AI coding agents (Claude Code, Codex,
Aider...), LLM-compacts each completed session, stores the result in
`.prompts/` inside your git repo, and binds each session to its matching
commit via `git notes`. The handoff feature is the new (v0.2) wedge; the
attribution engine is the (still-supported) `drift blame` / `drift log`
side that powers it.

After installation, `drift log` still shows multi-agent attribution per
commit:

```
commit abc1234 — Add OAuth login
   💭 [claude-code] 7 events accepted, 0 rejected
   💭 [codex]       3 events accepted, 1 rejected
   ✋ [human]       2 manual edits
```

…and `drift blame` still resolves any line back to its full timeline.
See [`docs/VISION.md`](docs/VISION.md) for the broader thesis.

## Why drift exists

AI coding stopped being a single-agent workflow. A real session today
looks more like this:

- You start a feature in Claude Code, hit a rate limit or fill the
  context window, and have to bail mid-task.
- You move to Codex (or Aider, or another model), but the new agent
  doesn't know which approaches you already tried, which decisions are
  settled, and which were *deliberately* rejected.
- You paste a chat transcript at the new agent. It's noisy, the agent
  re-litigates settled questions, and you spend ten minutes
  re-explaining what you wanted instead of moving forward.
- A week later you review the commit and can't tell which lines came
  from which agent, which were human edits on top of an AI suggestion,
  or *why* the code took the shape it took.
- Your teammate clones the repo and sees the code, but none of the
  reasoning that produced it — that history lived in someone else's
  Claude / Codex chat history, on someone else's laptop, and is now
  effectively gone.

`drift` turns that disposable AI trail into durable project memory:

- **Capture, locally**: `drift capture` (and `drift watch` for live
  mode) reads the session JSONL your agents already write under
  `~/.claude/projects/` and `~/.codex/sessions/`. Nothing leaves your
  machine except an optional Anthropic compaction call you can turn off.
- **Compact, into markdown**: each session becomes a small markdown
  summary in `.prompts/sessions/` — decisions kept, approaches
  rejected, files touched. Cheap to read, cheap to grep, survives any
  vendor migration because it's just text in your repo.
- **Bind, to commits**: `drift bind` / `drift auto-bind` attaches each
  session to the commit it produced via `git notes` (`refs/notes/drift`).
  The link travels with the repo; it does not pollute commit history.
- **Hand off, when you switch agents**: `drift handoff --branch <b> --to
  <agent>` produces a brief the next agent can absorb cold — what's
  done, what's open, what was already rejected, and where to resume.
- **Reverse-lookup, when you forget**: `drift blame <file> [--line N]`
  returns the full timeline behind a line of code: which session, which
  prompt, which agent, plus the human edits that landed on top.
- **Forward-lookup, when you remember the session but not the diff**:
  `drift trace <session-id>` lists every `CodeEvent` that session
  produced.
- **Audit, across a release**: `drift log` is `git log` with a per-agent
  summary under each commit — useful when you need to answer "how much
  of this release was AI vs. human" without trusting LOC ratios.

Net effect: multi-agent AI coding becomes something you can hand off,
review, and reconstruct months later — instead of a chat history that
disappears the next time you close a tab.

## Install

**Homebrew** (macOS arm64/x86_64, Linux arm64/x86_64):

```bash
brew install ShellFans-Kirin/drift/drift
```

**crates.io** (Rust 1.85+ toolchain required):

```bash
cargo install drift-ai
```

**Pre-built binaries** (GitHub Releases):

```bash
curl -sSfL https://github.com/ShellFans-Kirin/drift_ai/releases/latest/download/drift-v0.2.0-$(uname -m)-unknown-linux-gnu.tar.gz \
  | tar xz -C /tmp && sudo mv /tmp/drift /usr/local/bin/drift
drift --version
```

**From source**:

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo install --path crates/drift-cli
```

## Privacy & secrets

`drift` does **not** scrub session content. Anything you typed into your
Claude Code / Codex session — including secrets you may have pasted —
will be mirrored into `.prompts/` and, by default, committed to your repo.

Two knobs today:

1. Set `[attribution].db_in_git = false` in `.prompts/config.toml` to
   keep `events.db` local only.
2. Review `.prompts/sessions/` before `git add`.

`v0.2` will add a regex-based redaction pass. For full coverage today,
pair `drift` with [gitleaks](https://github.com/gitleaks/gitleaks) or
[trufflehog](https://github.com/trufflesecurity/trufflehog) as a
pre-commit hook.

> **If you routinely paste secrets into AI sessions, wait for `v0.2`
> before enabling `drift` on shared repos.**

The first time you run `drift capture`, you'll see a one-shot notice
restating the above; press Enter to acknowledge. Set
`DRIFT_SKIP_FIRST_RUN=1` to suppress in CI.

See [`docs/SECURITY.md`](docs/SECURITY.md) for the full threat model
and roadmap.

## Quickstart

Six commands, zero config:

```bash
cd your-git-repo
drift init                                          # scaffold .prompts/
drift capture                                       # pull sessions from ~/.claude + ~/.codex
drift handoff --branch feature/oauth --to claude   # NEW in v0.2 — task transfer
drift blame src/foo.rs                              # reverse lookup: who wrote what
drift trace <session-id>                            # forward lookup: session → events
drift install-hook                                  # auto-run after each commit
```

`drift handoff` is the v0.2 headline feature: package your in-progress
task into a brief the next agent can absorb cold. See [§Handoff](#handoff--cross-agent-task-transfer-v02)
for the full flow.

Verified from `/tmp` with zero prior state:

```bash
rm -rf /tmp/drift-smoke && mkdir -p /tmp/drift-smoke && cd /tmp/drift-smoke
git init -q && git config user.email ""x@y"" && git config user.name x
drift init && drift capture && drift list
```

## Handoff — cross-agent task transfer (v0.2)

`drift handoff` reads your local `events.db` (built up by `drift capture`
or `drift watch`), filters down to the sessions in the scope you ask
for, and produces a markdown brief structured for handoff:

- **What I'm working on** — 3-5 sentences of intent (LLM-compacted).
- **Progress so far** — done / in-progress / not-started bullets.
- **Files in scope** — modified ranges with ±5 lines of context.
- **Key decisions** — with session+turn citations.
- **Rejected approaches** — pre-extracted from session tool errors.
- **Open questions / blockers**.
- **Next steps**.
- **How to continue** — the prompt to paste into the target agent.

```bash
# scope by branch (recommended): all sessions whose commits land on this
# branch since it diverged from main
drift handoff --branch feature/oauth --to claude-code

# scope by time
drift handoff --since 2026-04-25T08:00:00Z --to codex

# single-session debug
drift handoff --session abc12345-xxx --print

# pipe to clipboard or another tool
drift handoff --branch feature/oauth --print | pbcopy
```

The default model is `claude-opus-4-7` — the brief is what the next
agent reads verbatim, so narrative quality matters more than for the
per-session compaction in v0.1. Each handoff costs ≈ \$0.10–\$0.30 USD
at Opus rates. To trade narrative for ~30× cost reduction, drop to
Haiku in `.prompts/config.toml`:

```toml
[handoff]
model = "claude-haiku-4-5"   # default is "claude-opus-4-7"
```

## Live mode — event-driven watcher

`drift watch` is an event-driven daemon backed by the platform's native
file-system notifications (FSEvents on macOS, inotify on Linux,
ReadDirectoryChangesW on Windows). It coalesces rapid writes against the
same session file inside a 200ms window, so a long running Claude Code
or Codex session produces one capture pass per idle moment, not one
per tool call. State is persisted to `~/.config/drift/watch-state.toml`
so a restart resumes rather than rescans. `Ctrl-C` finishes the
current capture and exits cleanly.

```bash
drift watch
# drift watch · event-driven; Ctrl-C to stop
#   watching /home/you/.claude/projects
#   watching /home/you/.codex/sessions
#   first run; capturing every session seen
# drift capture · provider=anthropic
# Captured 10 session(s), wrote 192 event(s) to .prompts/events.db
# ...
# drift watch · interrupt received; exiting after last capture
```

## Cost transparency

Every Anthropic compaction call is logged to `compaction_calls` in
`events.db` with input / output / cache token counts and a computed USD
cost (built-in pricing table per model; cross-check against
<https://www.anthropic.com/pricing> before trusting as invoice-ready).

```bash
drift cost
# drift cost — compaction billing
#   total calls      : 10
#   input tokens     : 120958
#   output tokens    : 6582
#   cache creation   : 0
#   cache read       : 0
#   total cost (USD) : $0.1539

drift cost --by model
# ── grouped by model (descending cost)
#   key                    calls   input_tok   output_tok     cost (USD)
#   claude-haiku-4-5          10      120958         6582        $0.1539

drift cost --by session
# ── grouped by session (descending cost)
#   key                                     calls   input_tok   output_tok     cost (USD)
#   4b1e2ba0-621c-4977-af3f-2a9df5ac45ec        2       51696         2448        $0.0564
#   ad01ae46-156f-403b-b263-dd04a232873a        1       33662         2390        $0.0456
#   ...
```

Filter with `--since <date>`, `--until <date>`, `--model <name>`.
Switching from Opus to Haiku on the same 10-session corpus takes
compaction from **$2.91 → $0.15** — a ~19× reduction at the cost of
slightly terser summaries.

## AI-native blame

`drift blame` is the reverse lookup. Given a line of code, it returns
the full timeline of who touched it (multi-agent + human edits), with
each entry linked back to the originating session and prompt.

See [`docs/VISION.md`](docs/VISION.md) for the three core scenarios:
**reverse** (`drift blame`), **forward** (`drift trace`), and **audit**
(`drift log`).

## MCP integration

Drift AI ships its own stdio MCP server (`drift mcp`). Any
MCP-compatible client can call the five read-only tools — `drift_blame`,
`drift_trace`, `drift_rejected`, `drift_log`, `drift_show_event` — to
query the attribution store without spawning a subshell.

**Claude Code** (one command):

```bash
claude mcp add drift -- drift mcp
```

**Codex**:

```bash
codex mcp add drift -- drift mcp
```

Tools are read-only by design — anything that mutates state
(`capture` / `bind` / `sync`) stays CLI-only.

## Commands

| Command | Purpose |
|---------|---------|
| `drift init` | scaffold `.prompts/` + project config |
| `drift capture` | one-shot: discover sessions, compact, attribute |
| `drift watch` | background daemon, debounced re-capture |
| `drift handoff [--branch B --to A --print --output P]` | **v0.2** — cross-agent task brief |
| `drift list [--agent A]` | list captured sessions |
| `drift show <id>` | render a compacted session |
| `drift blame <file> [--line N] [--range A-B]` | **reverse lookup** |
| `drift trace <session-id>` | **forward lookup** |
| `drift diff <event-id>` | single event's unified diff |
| `drift rejected [--since DATE]` | list rejected AI suggestions |
| `drift log [-- <git-args>]` | `git log` + per-agent session summaries |
| `drift bind <commit> <session>` | attach a session to a commit note |
| `drift auto-bind` | timestamp-pair every session to its closest commit |
| `drift install-hook` | install a non-blocking post-commit hook |
| `drift sync push\|pull <remote>` | push/pull `refs/notes/drift` |
| `drift config get\|set\|list` | global + project TOML merge |
| `drift mcp` | run the stdio MCP server |

## Configuration

Global: `~/.config/drift/config.toml`
Project (overrides): `<repo>/.prompts/config.toml`

```toml
[attribution]
db_in_git = true          # default — teams share blame via the repo

[connectors]
claude_code = true
codex = true
aider = false             # feature-gated stub

[compaction]
provider = "anthropic"      # default; switch to "mock" for offline / testing
model = "claude-haiku-4-5"  # or claude-sonnet-4-6 / claude-opus-4-7

[handoff]
model = "claude-opus-4-7"   # narrative-quality is the value; switch to
                            # "claude-haiku-4-5" for ~30x cost reduction
```

`ANTHROPIC_API_KEY`: required for the live API compaction path. When
it's unset, drift_ai transparently falls back to `MockProvider` and
every summary is labelled `[MOCK]` so you never mistake a fallback
run for a real one — nothing else in the pipeline changes.

How drift compares to Cursor / Copilot history, Cody, and `git blame`
itself: [`docs/COMPARISON.md`](docs/COMPARISON.md).

## Honest limitations (v0.2.0)

- Human-edit detection is SHA-ladder only — we do not claim authorship,
  the `human` slug means "no AI session produced this". See VISION.md.
- `Bash python -c "open(...).write(...)"` is best-effort; anything the
  shell lexer misses is caught by the SHA ladder and attributed to
  `human`.
- Codex `reasoning` items are encrypted; we count them, we do not
  surface them.
- Cost totals use a hardcoded pricing table — cross-check against
  <https://www.anthropic.com/pricing> before treating as invoice-ready.
- Context-window truncation is deterministic head+tail elision
  (Strategy 1); hierarchical summarization (Strategy 2) is stubbed
  behind a feature flag for v0.2.

## About

`drift` is an independent open-source project by
[@ShellFans-Kirin](https://github.com/ShellFans-Kirin)
([shellfans.dev](https://shellfans.dev)). It is **not** affiliated with
Anthropic, OpenAI, or any other vendor whose agents it integrates with —
`drift` is built *on top of* their session logs, not by them.

> Originally built for myself when I kept losing context between Codex
> stalls and Claude rate-limits. The v0.2 `drift handoff` feature is
> the part I personally use most.

## License

Apache 2.0 — see [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) — it walks through adding a new
connector using the Aider stub as the worked example.
