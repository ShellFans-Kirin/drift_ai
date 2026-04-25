> 🌐 **English** · [日本語](i18n/ja/COMPARISON.md) · [简体中文](i18n/zh-Hans/COMPARISON.md) · [繁體中文](i18n/zh-Hant/COMPARISON.md)

# How `drift` compares

This is a *functional* comparison — what a tool stores, where, and what
queries it supports. It is not a verdict on which is "better"; the tools
solve overlapping but distinct problems. **`drift` does not replace any
of these. `drift` reads what these tools write and adds an attribution
layer on top.**

| Tool | Stores AI sessions? | Where | Line-level blame? | Multi-agent? | Local-first? |
|---|---|---|---|---|---|
| Cursor history | ✓ | Cloud (Cursor servers) | ✗ | ✗ (Cursor only) | ✗ |
| GitHub Copilot chat history | ✓ | Cloud (GitHub) | ✗ | ✗ (Copilot only) | ✗ |
| Cody (Sourcegraph) | ✓ | Cloud (Sourcegraph) | ✗ | ✗ (Cody only) | ✗ |
| `git blame` | — | Local repo | Commit-level only | — (just code) | ✓ |
| **`drift`** | ✓ | **Local `.prompts/` in your repo** | **✓ per line** | **✓ Claude + Codex + human + extensible** | **✓** |

## What each tool actually answers

- **Cursor history / Copilot chat history / Cody**: "Show me the
  conversations I had with my agent." Indexed by date or by chat
  thread. Tied to one vendor's UI. Disappears if you cancel your
  subscription or switch tools.

- **`git blame`**: "Who introduced this line, in what commit?" Commit
  message and committer email. No knowledge of agents or prompts.
  Bundled with git; works on any repo without setup.

- **`drift`**: "Who — *which agent on which prompt, or which human edit
  after which commit* — introduced this line, *and what diff did they
  produce*?" Indexed by file + line, by session, by commit, by agent,
  and by rejection state. Ships its own SQLite store inside the repo
  and an MCP server so other AI tools can query attribution back.

## Where `drift` extends each one

- If you use Cursor, you can write a `cursor` connector against
  `SessionConnector` (in `crates/drift-connectors/`) and `drift` will
  index Cursor's local session JSONL alongside Claude Code / Codex.
  See [`CONTRIBUTING.md`](../CONTRIBUTING.md) — Aider's stub is the
  worked example.

- If you use Copilot chat, the same — point a connector at the local
  cache directory Copilot maintains and you get unified blame.

- If you use `git blame` and don't use any AI agent, `drift` adds the
  human-edit detection layer (`AgentSlug::Human` is "no AI session
  produced this", not an authorship claim) but the win is small —
  `git blame` already handles your case.

The thesis: when your work involves multiple agents *and* manual edits
*and* the agents you use change every six months, the "store sessions
in our cloud" model breaks. `drift` keeps the source of truth in your
git repo so the attribution survives any vendor migration.

## What `drift` is not

- Not a chat client. `drift` doesn't replace Cursor's chat UI or
  Claude Code's REPL.
- Not a code review tool. It records what happened; it does not
  judge whether the AI's suggestion was correct.
- Not a privacy product. The default config commits `events.db` to
  your repo — see [`SECURITY.md`](SECURITY.md) for the trade-offs.
