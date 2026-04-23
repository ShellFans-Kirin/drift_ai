# drift-mcp

Read-only stdio [Model Context Protocol](https://modelcontextprotocol.io/)
server for [**Drift AI**](https://github.com/ShellFans-Kirin/drift_ai).

`drift-mcp` exposes five read-only tools over stdio JSON-RPC so any
MCP-compatible client (Claude Code, Codex, Claude Desktop, and others)
can query the drift attribution store without shelling out:

- `drift_blame` — reverse lookup: file → timeline of who touched each line
- `drift_trace` — forward lookup: session → all events it produced
- `drift_rejected` — list suggestions the agent dropped (tool_result errors)
- `drift_log` — `git log`-style view with per-commit session summaries
- `drift_show_event` — fetch a single `CodeEvent` by id

Launch directly from your MCP client:

```bash
claude mcp add drift -- drift mcp
codex  mcp add drift -- drift mcp
```

Anything that mutates state (`capture`, `bind`, `sync`) stays CLI-only.

See the [top-level README](https://github.com/ShellFans-Kirin/drift_ai)
for the wider system.

Licensed under [Apache-2.0](https://github.com/ShellFans-Kirin/drift_ai/blob/main/LICENSE).
