> 🌐 [English](../../COMPARISON.md) · [日本語](../ja/COMPARISON.md) · **简体中文** · [繁體中文](../zh-Hant/COMPARISON.md)

# `drift` 跟其他工具怎么比

这是一份 *功能性* 比较 — 工具存什么、存在哪、支持什么查询。这不是「谁更好」
的判决；这些工具解的问题重叠但不相同。**`drift` 不取代任何一个。`drift` 读的是
这些工具写出来的内容，再在上面叠一层 attribution。**

| Tool | 存 AI session？ | 存在哪 | 行级 blame？ | 多 agent？ | Local-first？ |
|---|---|---|---|---|---|
| Cursor history | ✓ | 云端（Cursor servers） | ✗ | ✗（只 Cursor） | ✗ |
| GitHub Copilot chat history | ✓ | 云端（GitHub） | ✗ | ✗（只 Copilot） | ✗ |
| Cody (Sourcegraph) | ✓ | 云端（Sourcegraph） | ✗ | ✗（只 Cody） | ✗ |
| `git blame` | — | 本地 repo | 只到 commit 级 | —（纯 code） | ✓ |
| **`drift`** | ✓ | **repo 内的本地 `.prompts/`** | **✓ per line** | **✓ Claude + Codex + human + 可扩展** | **✓** |

## 各工具实际在回答什么

- **Cursor history / Copilot chat history / Cody**：「我跟我的 agent 聊过什么」。
  按日期或对话线索引。绑在单一 vendor 的 UI。取消订阅或换工具就消失。

- **`git blame`**：「谁、在哪个 commit、引入了这行」。回答 commit message 跟
  committer email。对 agent 跟 prompt 一无所知。git 自带，任何 repo 不用设置
  就能用。

- **`drift`**：「谁 — *哪个 agent 在哪个 prompt 上，或哪个 commit 之后的人类
  编辑* — 引入了这行，*而且他们产生了什么 diff*？」可按文件 + 行、按 session、
  按 commit、按 agent、按 rejected 状态查询。内置一份在 repo 里的 SQLite store
  跟 MCP server，让其他 AI 工具能反过来查 attribution。

## `drift` 在哪里延伸了它们

- 如果你用 Cursor，可以对着 `SessionConnector`（在 `crates/drift-connectors/`）
  写一个 `cursor` connector，`drift` 就会把 Cursor 本机的 session JSONL 跟
  Claude Code / Codex 一起索引。见 [`CONTRIBUTING.md`](../../../CONTRIBUTING.md) — Aider
  的 stub 是现成的示例。

- 如果你用 Copilot chat，同样的 — 把 connector 对到 Copilot 本机 cache 目录，
  你就拿到统一 blame。

- 如果你只用 `git blame` 而没在用任何 AI agent，`drift` 加的是「人类编辑检测」
  那一层（`AgentSlug::Human` 是「没有 AI session 产生这个」，不是作者主张）—
  但 win 很小，`git blame` 已经满足你的需求。

Thesis：当你的工作牵涉到多个 agent *加上* 手改 *加上* 你用的 agent 每六个月
就换一轮时，「session 存我家云端」那种模式就破了。`drift` 把 source of truth
留在你的 git repo，这样 attribution 在任何一次 vendor 迁移之后都还在。

## `drift` 不是什么

- 不是 chat client。`drift` 不取代 Cursor 的 chat UI 或 Claude Code 的 REPL。
- 不是 code review 工具。它记录发生了什么，不判断 AI 的建议对不对。
- 不是隐私产品。默认 config 会把 `events.db` commit 进你的 repo — trade-off
  详见 [`SECURITY.md`](SECURITY.md)。
