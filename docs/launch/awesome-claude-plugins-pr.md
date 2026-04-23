# Draft PR — add drift_ai to ComposioHQ/awesome-claude-plugins

**Target repo**: https://github.com/ComposioHQ/awesome-claude-plugins
**Branch (in your fork)**: `add-drift-ai`

---

## PR title

`Add drift_ai — local-first AI-native blame`

## PR body

Adds `drift_ai` to the index. It's an Apache-2.0 CLI that captures
Claude Code (and Codex) sessions, compacts them, and binds each
session to the matching commit via `git notes`, so `drift blame <file>
--line N` resolves any line back to its originating prompt (plus
later human edits, plus any rejected suggestions).

Ships an MCP server (`drift mcp`) with five read-only tools —
`drift_blame`, `drift_trace`, `drift_rejected`, `drift_log`,
`drift_show_event` — installable in one command:

```bash
claude mcp add drift -- drift mcp
```

Plugin manifest included at
`plugins/claude-code/.claude-plugin/plugin.json` in the repo.

## README entry diff

Add under the appropriate section (e.g. "Developer Tools" or
"Productivity"):

```markdown
- **[drift_ai](https://github.com/ShellFans-Kirin/drift_ai)** —
  Local-first AI-native `git blame`: captures Claude Code sessions,
  line-level attribution, MCP server for reverse/forward lookup.
  Apache-2.0, Rust.
```

## Checklist

- [x] Project has a permissive OSS license (Apache-2.0)
- [x] Working MCP server (stdio, JSON-RPC 2.0, 2024-11-05 handshake)
- [x] Pre-compiled install route (GitHub Releases; Homebrew tap in
      flight)
- [x] Demonstrable use case in the README (reverse blame + forward
      trace demos)
- [x] No shady telemetry — runs 100% local; opts in to Anthropic API
      only when `ANTHROPIC_API_KEY` is present
