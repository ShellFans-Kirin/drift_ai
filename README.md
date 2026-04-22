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

**Pre-built binaries** (macOS x86_64/arm64 + Linux x86_64/arm64):

```bash
# Check the latest release
curl -sSfL https://github.com/shellfans-dev/drift_ai/releases/latest
```

**From source** (requires Rust 1.85+):

```bash
git clone https://github.com/shellfans-dev/drift_ai.git
cd drift_ai
cargo install --path crates/drift-cli
```

**Homebrew tap** (planned for v0.1.x — see
[`docs/distribution/drift.rb.template`](docs/distribution/drift.rb.template)).

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
model = "claude-opus-4-7" # used when ANTHROPIC_API_KEY is set
# provider = "mock"       # force Mock (default when key is unset)
```

`ANTHROPIC_API_KEY`: only required for the real API compaction path.
Without it, drift_ai uses `MockProvider` and tags summaries `[MOCK]` —
nothing else is affected.

## Honest limitations (v0.1.0)

- Human-edit detection is SHA-ladder only — we do not claim authorship,
  the `human` slug means "no AI session produced this". See VISION.md.
- `Bash python -c "open(...).write(...)"` is best-effort; anything the
  shell lexer misses is caught by the SHA ladder and attributed to
  `human`.
- Codex `reasoning` items are encrypted; we count them, we do not
  surface them.
- `drift watch` is a debounced polling daemon, not a fully
  event-driven reactor (v0.2).
- Anthropic HTTP call is stubbed — Mock path is the shipping default.
  The integration point is marked in `crates/drift-core/src/compaction.rs`.

## License

Apache 2.0 — see [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) — it walks through adding a new
connector using the Aider stub as the worked example.
