# Drift AI plugins (marketplace-facing)

This directory contains plugin manifests for the Claude Code and Codex
plugin marketplaces. **In v0.1.0 they are skeletons, not published.**

Both manifests register the exact same MCP server (`drift mcp`), so the
logic lives once — in `crates/drift-mcp/` — and these plugins are thin
shells that surface the same five tools
(`drift_blame` / `drift_trace` / `drift_rejected` / `drift_log` /
`drift_show_event`) through each host's plugin UI.

## v0.2 publish plan

| Target | Action |
|--------|--------|
| [ComposioHQ/awesome-claude-plugins](https://github.com/ComposioHQ/awesome-claude-plugins) | Send a PR adding `drift` to the index; body points here |
| Claude Code plugin store | Submit `plugins/claude-code/.claude-plugin/plugin.json` per Anthropic guidance |
| Codex marketplace | Submit `plugins/codex/marketplace.json` |

Until then, install the MCP server directly (see the README
"MCP integration" section):

```bash
# Claude Code
claude mcp add drift -- drift mcp

# Codex
codex mcp add drift -- drift mcp
```
