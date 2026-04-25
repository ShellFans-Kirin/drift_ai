> 🌐 [English](../../COMPARISON.md) · [日本語](../ja/COMPARISON.md) · [简体中文](../zh-Hans/COMPARISON.md) · **繁體中文**

# `drift` 跟其他工具怎麼比

這是一份 *功能性* 比較 — 工具存什麼、存在哪、支援什麼查詢。這不是「誰比較好」
的判決；這些工具解的問題重疊但不相同。**`drift` 不取代任何一個。`drift` 讀的是
這些工具寫出來的東西，再在上面疊一層 attribution。**

| Tool | 存 AI session？ | 存在哪 | 行級 blame？ | 多 agent？ | Local-first？ |
|---|---|---|---|---|---|
| Cursor history | ✓ | 雲端（Cursor servers） | ✗ | ✗（只 Cursor） | ✗ |
| GitHub Copilot chat history | ✓ | 雲端（GitHub） | ✗ | ✗（只 Copilot） | ✗ |
| Cody (Sourcegraph) | ✓ | 雲端（Sourcegraph） | ✗ | ✗（只 Cody） | ✗ |
| `git blame` | — | 本地 repo | 只到 commit 級 | —（純 code） | ✓ |
| **`drift`** | ✓ | **repo 內的本地 `.prompts/`** | **✓ per line** | **✓ Claude + Codex + human + 可擴充** | **✓** |

## 各工具實際在回答什麼

- **Cursor history / Copilot chat history / Cody**：「我跟我的 agent 聊過什麼」。
  按日期或對話線索引。綁在單一 vendor 的 UI。取消訂閱或換工具就消失。

- **`git blame`**：「誰、在哪個 commit、引入了這行」。回答 commit message 跟
  committer email。對 agent 跟 prompt 一無所知。git 自帶，任何 repo 不用設定
  就能用。

- **`drift`**：「誰 — *哪個 agent 在哪個 prompt 上，或哪個 commit 之後的人類
  編輯* — 引入了這行，*而且他們產生了什麼 diff*？」可依檔案 + 行、依 session、
  依 commit、依 agent、依 rejected 狀態查詢。內建一份在 repo 裡的 SQLite store
  跟 MCP server，讓其他 AI 工具能反過來查 attribution。

## `drift` 在哪裡延伸了它們

- 如果你用 Cursor，可以對著 `SessionConnector`（在 `crates/drift-connectors/`）
  寫一個 `cursor` connector，`drift` 就會把 Cursor 本機的 session JSONL 跟
  Claude Code / Codex 一起索引。見 [`CONTRIBUTING.md`](../../../CONTRIBUTING.md) — Aider
  的 stub 是現成的範例。

- 如果你用 Copilot chat，同樣的 — 把 connector 對到 Copilot 本機 cache 目錄，
  你就拿到統一 blame。

- 如果你只用 `git blame` 而沒在用任何 AI agent，`drift` 加的是「人類編輯偵測」
  那一層（`AgentSlug::Human` 是「沒有 AI session 產生這個」，不是作者主張）—
  但 win 很小，`git blame` 已經滿足你的需求。

Thesis：當你的工作牽涉到多個 agent *加上* 手改 *加上* 你用的 agent 每六個月
就換一輪時，「session 存我家雲端」那種模式就破了。`drift` 把 source of truth
留在你的 git repo，這樣 attribution 在任何一次 vendor 遷移之後都還在。

## `drift` 不是什麼

- 不是 chat client。`drift` 不取代 Cursor 的 chat UI 或 Claude Code 的 REPL。
- 不是 code review 工具。它記錄發生了什麼，不判斷 AI 的建議對不對。
- 不是隱私產品。預設 config 會把 `events.db` commit 進你的 repo — trade-off
  詳見 [`SECURITY.md`](SECURITY.md)。
