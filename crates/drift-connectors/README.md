# drift-connectors

Session connectors for [**Drift AI**](https://github.com/ShellFans-Kirin/drift_ai),
the AI-native blame CLI.

Each connector implements `SessionConnector` (from
[`drift-core`](https://crates.io/crates/drift-core)) — `discover()` +
`parse()` + `extract_code_events()` — against one agent's on-disk
session format:

- **`claude_code`** — `~/.claude/projects/<proj>/<uuid>.jsonl`; Claude
  Code's Messages-API-flavoured transcript with `Write`/`Edit`/`Bash`
  tool calls.
- **`codex`** — `~/.codex/sessions/<date>/<uuid>.jsonl`; OpenAI Codex
  CLI output with `apply_patch` semantics (add/update/delete/move).
- **`aider`** (feature-gated) — stub; see CONTRIBUTING in the main
  repo for the worked example of wiring a new connector.

Not usually depended on directly — the `drift` CLI picks the full set
via `default_connectors()` behind `default = ["claude-code", "codex"]`.

See the [top-level README](https://github.com/ShellFans-Kirin/drift_ai)
for the broader system.

Licensed under [Apache-2.0](https://github.com/ShellFans-Kirin/drift_ai/blob/main/LICENSE).
