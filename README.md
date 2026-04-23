# drift_ai

> AI-native blame for the post-prompt era. Local-first.

`drift` watches the local session logs of your AI coding agents
(Claude Code, Codex, Aider...), LLM-compacts each completed session,
stores the result in `.prompts/` inside your git repo, **builds a
line-level attribution layer that links every code event back to its
originating prompt**, and binds each session to its matching commit
via `git notes`.

After installation, `drift log` shows multi-agent attribution per
commit:

```
commit abc1234 — Add OAuth login
   💭 [claude-code] 7 events accepted, 0 rejected
   💭 [codex]       3 events accepted, 1 rejected
   ✋ [human]       2 manual edits
```

…and `drift blame` resolves any line back to its full timeline:

```
src/auth/login.ts
├─ 2026-04-15 14:03  💭 [claude-code] session abc12345
│   --- a/src/auth/login.ts
│   +++ b/src/auth/login.ts
│   @@
│   +if (attempts > 5) throw new RateLimitError()
├─ 2026-04-15 15:20  ✋ [human]       post-commit manual edit
│   -  if (attempts > 5)
│   +  if (attempts > MAX_ATTEMPTS)
└─ 2026-04-16 09:12  💭 [codex]       session def45678
    +const MAX_ATTEMPTS = 5
```

The thesis: commit granularity is too coarse to be the source of truth
in the AI era. drift_ai keeps prompts and the code they produce
stitched together at the line level — across multiple agents, across
human edits, across renames, and including the suggestions you
rejected. See [`docs/VISION.md`](docs/VISION.md).

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
curl -sSfL https://github.com/ShellFans-Kirin/drift_ai/releases/latest/download/drift-v0.1.1-$(uname -m)-unknown-linux-gnu.tar.gz \
  | tar xz -C /tmp && sudo mv /tmp/drift /usr/local/bin/drift
drift --version
```

**From source**:

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo install --path crates/drift-cli
```

## Quickstart

Five commands, zero config:

```bash
cd your-git-repo
drift init                               # scaffold .prompts/
drift capture                            # pull sessions from ~/.claude + ~/.codex
drift blame src/foo.rs                   # reverse lookup: who wrote what
drift trace <session-id>                 # forward lookup: session → events
drift install-hook                       # auto-run after each commit
```

Verified from `/tmp` with zero prior state:

```bash
rm -rf /tmp/drift-smoke && mkdir -p /tmp/drift-smoke && cd /tmp/drift-smoke
git init -q && git config user.email ""x@y"" && git config user.name x
drift init && drift capture && drift list
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
provider = "anthropic"    # default; set to "mock" to run offline
model = "claude-opus-4-7" # or claude-sonnet-4-6 / claude-haiku-4-5
```

`ANTHROPIC_API_KEY`: required for the live API compaction path. When
it's unset, drift_ai transparently falls back to `MockProvider` and
every summary is labelled `[MOCK]` so you never mistake a fallback
run for a real one — nothing else in the pipeline changes.

## Honest limitations (v0.1.1)

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

## License

Apache 2.0 — see [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) — it walks through adding a new
connector using the Aider stub as the worked example.
