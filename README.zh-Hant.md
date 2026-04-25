> 🌐 [English](README.md) · [日本語](README.ja.md) · [简体中文](README.zh-Hans.md) · **繁體中文**

# drift_ai

[![crates.io](https://img.shields.io/crates/v/drift-ai.svg)](https://crates.io/crates/drift-ai)
[![CI](https://github.com/ShellFans-Kirin/drift_ai/actions/workflows/ci.yml/badge.svg)](https://github.com/ShellFans-Kirin/drift_ai/actions/workflows/ci.yml)

> 把進行中的 AI coding task 在 Claude、Codex 與下一個 agent 之間平順 handoff。
> Local-first。

![drift handoff demo](docs/demo/v020-handoff.gif)

**問題**：你的 AI coding agent 卡住了 — 拒答、被 rate limit、或就是突然變笨。
現在你得把 30 分鐘累積的脈絡轉給另一個 agent。直接把對話歷史貼過去沒用；新
agent 不知道哪些決策已經拍板、哪些做法你已經試過放棄、或上一刻你寫到哪個檔的
哪一行。

**`drift handoff`** 把進行中的 task 包成任何 LLM 都能冷讀的 markdown brief：

```bash
$ drift handoff --branch feature/oauth --to claude-code
⚡ scanning .prompts/events.db
⚡ extracting file snippets and rejected approaches
⚡ compacting brief via claude-opus-4-7
✅ written to .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md
```

Brief 會列出已決定的事、試過卻被駁回的做法、還沒解決的點、以及下一步從哪接。
貼到下一個 agent，他們就能不需要你重新解釋直接接手。

`drift` 蓋在 v0.1 的 attribution engine 之上 — 那層在背景監看你 AI coding agent
（Claude Code、Codex、Aider…）的本地 session log，逐 session 用 LLM compact，把
結果存進 git repo 內的 `.prompts/`，並透過 `git notes` 把 session 綁到對應 commit。
Handoff 是 v0.2 新增的 wedge feature；attribution engine 仍由 `drift blame` /
`drift log` 那一側支撐。

裝完之後，`drift log` 仍然會顯示每個 commit 的多 agent attribution：

```
commit abc1234 — Add OAuth login
   💭 [claude-code] 7 events accepted, 0 rejected
   💭 [codex]       3 events accepted, 1 rejected
   ✋ [human]       2 manual edits
```

…而 `drift blame` 仍然能把任意一行 code 還原成完整的 timeline。
專案 thesis 見 [`docs/VISION.md`](docs/VISION.md)。

## 為什麼需要 drift

AI coding 已經不是單一 agent 的工作流了。今天一段真實的開發 session 比較像這樣:

- 你在 Claude Code 上開了個 feature,寫到一半遇到 rate limit、或 context
  window 塞滿,或者你發現 LLM 突然 **變蠢** 了,被迫中途放棄。
- 你切到 Codex(或 Aider、或別的 model),但新 agent 不知道你試過哪些做法、
  哪些決策已經拍板、哪些 *刻意* 被駁回。
- 你把 chat 貼過去給新 agent。資訊雜亂、agent 把已經結案的問題又拉出來討論,
  你花十分鐘重新解釋想做的事,而不是繼續往前走。
- 一週後 review commit,你分不出哪幾行是哪個 agent 寫的、哪些是人類在 AI
  建議上加的編輯、code 為什麼最後長這樣。
- 同事 clone repo 看到的是 code,但產生這份 code 的推理過程不見了 — 那段
  歷史活在某個人的 Claude / Codex chat 裡、在某個人的筆電上,現在實質已經消失。

`drift` 把這條本來會丟掉的 AI 軌跡轉成可長期保存的 project memory:

- **本機 capture**:`drift capture`(以及 `drift watch` 走 live mode)讀你
  agent 本來就會寫到 `~/.claude/projects/` 跟 `~/.codex/sessions/` 的 session
  JSONL。除了你可以關掉的選用 Anthropic compaction 之外,沒有任何資料離開你
  的機器。
- **壓成 markdown**:每段 session 變成 `.prompts/sessions/` 下一份小的 markdown
  摘要 — 留下的決策、被駁回的做法、改過的檔案。讀起來輕、grep 起來輕、跨 vendor
  遷移也不會丟,因為這就只是 repo 裡的 text。
- **綁到 commit**:`drift bind` / `drift auto-bind` 透過 `git notes`
  (`refs/notes/drift`)把每段 session 綁到它產出的 commit。連結跟著 repo 走,
  不污染 commit history。
- **切 agent 時 handoff**:`drift handoff --branch <b> --to <agent>` 產一份
  下一個 agent 能冷讀的 brief — 哪些做完、哪些還開著、哪些已經被駁回、從哪
  裡接。
- **忘記時反查**:`drift blame <file> [--line N]` 回傳那一行 code 背後的完整
  timeline:哪個 session、哪個 prompt、哪個 agent,加上後續落上去的人類編輯。
- **記得 session 不記得 diff 時順查**:`drift trace <session-id>` 列出該 session
  產生的每筆 `CodeEvent`。
- **跨 release 做 audit**:`drift log` 是 `git log` 但每個 commit 下面多一段
  per-agent 摘要 — 要回答「這次 release 多少是 AI、多少是人類」而不靠 LOC
  比例瞎猜時很有用。

最終效果:multi-agent AI coding 變成可以 handoff、review、幾個月後還原得回來的
東西 — 而不是下次關掉 tab 就消失的 chat history。

## 安裝

**Homebrew**（macOS arm64/x86_64、Linux arm64/x86_64）：

```bash
brew install ShellFans-Kirin/drift/drift
```

**crates.io**（需 Rust 1.85+ toolchain）：

```bash
cargo install drift-ai
```

**預編 binary**（GitHub Releases）：

```bash
curl -sSfL https://github.com/ShellFans-Kirin/drift_ai/releases/latest/download/drift-v0.2.0-$(uname -m)-unknown-linux-gnu.tar.gz \
  | tar xz -C /tmp && sudo mv /tmp/drift /usr/local/bin/drift
drift --version
```

**從原始碼**：

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo install --path crates/drift-cli
```

## 隱私與 secrets

`drift` **不會**主動清洗 session 內容。你打進 Claude Code / Codex session 的任何
文字 — 包含手滑貼進去的 secret — 都會被鏡像到 `.prompts/`，預設情況下也會被
commit 進 repo。

目前提供兩個調節旋鈕：

1. 在 `.prompts/config.toml` 設定 `[attribution].db_in_git = false`，讓 `events.db`
   只留本機。
2. 在 `git add` 之前先 review 一遍 `.prompts/sessions/`。

`v0.2` 會新增一輪 regex-based redaction。要當下就完整覆蓋的話，把 `drift` 跟
[gitleaks](https://github.com/gitleaks/gitleaks) 或
[trufflehog](https://github.com/trufflesecurity/trufflehog) 配成 pre-commit hook
一起跑。

> **如果你常把 secret 貼進 AI session，請等 `v0.2` 再到共用 repo 上啟用 `drift`。**

第一次跑 `drift capture` 時會顯示一段一次性 notice 重述上述內容；按 Enter 確認
即可。在 CI 用 `DRIFT_SKIP_FIRST_RUN=1` 跳過。

完整 threat model 與 roadmap 見 [`docs/SECURITY.md`](docs/SECURITY.md)。

## 快速上手

六個指令、零 config：

```bash
cd your-git-repo
drift init                                          # scaffold .prompts/
drift capture                                       # pull sessions from ~/.claude + ~/.codex
drift handoff --branch feature/oauth --to claude   # NEW in v0.2 — task transfer
drift blame src/foo.rs                              # 反查：誰寫了這行
drift trace <session-id>                            # 順查：session → events
drift install-hook                                  # 每次 commit 後自動跑
```

`drift handoff` 是 v0.2 的招牌功能：把進行中的 task 包成下一個 agent 能冷讀的
brief。完整流程見 [§Handoff](#handoff--跨-agent-task-transfer-v02)。

從 `/tmp` 零狀態驗過：

```bash
rm -rf /tmp/drift-smoke && mkdir -p /tmp/drift-smoke && cd /tmp/drift-smoke
git init -q && git config user.email ""x@y"" && git config user.name x
drift init && drift capture && drift list
```

## Handoff — 跨 agent task transfer (v0.2)

`drift handoff` 從你本機的 `events.db`（由 `drift capture` 或 `drift watch` 累積
而來）讀資料，依你指定的 scope 篩選 sessions，產出一份 handoff 用的 markdown
brief，結構如下：

- **What I'm working on** — 3-5 句意圖（LLM compacted）。
- **Progress so far** — done / in-progress / not-started 條列。
- **Files in scope** — 改過的範圍 ±5 行 context。
- **Key decisions** — 附 session+turn 引用。
- **Rejected approaches** — 從 session 的 tool error 預先抽出。
- **Open questions / blockers**。
- **Next steps**。
- **How to continue** — 直接貼進下一個 agent 的 prompt。

```bash
# 用 git branch 圈定 scope（建議）：所有最後落在這個 branch（從 main 分出之後）
# commit 對應的 session
drift handoff --branch feature/oauth --to claude-code

# 用時間 scope
drift handoff --since 2026-04-25T08:00:00Z --to codex

# 單一 session（debug 用）
drift handoff --session abc12345-xxx --print

# pipe 到 clipboard 或其他工具
drift handoff --branch feature/oauth --print | pbcopy
```

預設 model 是 `claude-opus-4-7` — brief 是下一個 agent 逐字讀的東西，敘事品質
比 v0.1 的 per-session compaction 更重要。每次 handoff 在 Opus 費率下大約
≈ \$0.10–0.30 USD。要犧牲敘事換取 ~30× 成本下降，在 `.prompts/config.toml` 切到
Haiku：

```toml
[handoff]
model = "claude-haiku-4-5"   # 預設是 "claude-opus-4-7"
```

## Live mode — 事件驅動 watcher

`drift watch` 是事件驅動的 daemon，底層用各 platform 原生的 file-system
notification（macOS FSEvents、Linux inotify、Windows ReadDirectoryChangesW）。
對同一個 session 檔的密集寫入會在 200ms 視窗內合併，所以一個跑很久的 Claude
Code 或 Codex session 每段 idle 才觸發一次 capture，不會每個 tool call 都觸發
一次。狀態存到 `~/.config/drift/watch-state.toml`，restart 之後 resume 而不是
全重掃。`Ctrl-C` 會等當前 capture 收尾再乾淨退出。

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

## 成本透明

每一次 Anthropic compaction 呼叫都會寫進 `events.db` 的 `compaction_calls` 表，
含 input / output / cache token 數量與算好的 USD 成本（內建依 model 的計價表；
要當作正式發票請先核對 <https://www.anthropic.com/pricing>）。

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

可用 `--since <date>`、`--until <date>`、`--model <name>` 過濾。在同一份 10 session
語料上把 Opus 換成 Haiku，compaction 成本從 **$2.91 → $0.15** — ~19× 下降，代價
是摘要會更精簡一點。

## AI-native blame

`drift blame` 是反查：給一行 code，回傳改過它的完整 timeline（多 agent + 人類
編輯都算），每筆都連到原始的 session 與 prompt。

完整三大場景見 [`docs/VISION.md`](docs/VISION.md)：
**反查**（`drift blame`）、**順查**（`drift trace`）、**audit**（`drift log`）。

## MCP 整合

Drift AI 內建一個 stdio MCP server（`drift mcp`）。任何 MCP-compatible 的 client
都能呼叫這 5 個唯讀 tool — `drift_blame`、`drift_trace`、`drift_rejected`、
`drift_log`、`drift_show_event` — 直接查 attribution，不需要另外開 subshell。

**Claude Code**（一行）：

```bash
claude mcp add drift -- drift mcp
```

**Codex**：

```bash
codex mcp add drift -- drift mcp
```

Tool 設計上是唯讀的 — 任何會改動 state 的動作（`capture` / `bind` / `sync`）
只在 CLI 提供。

## Commands

| Command | 用途 |
|---------|---------|
| `drift init` | scaffold `.prompts/` + 專案 config |
| `drift capture` | 一次性：發現 session、compact、attribute |
| `drift watch` | 背景 daemon，debounced re-capture |
| `drift handoff [--branch B --to A --print --output P]` | **v0.2** — 跨 agent task brief |
| `drift list [--agent A]` | 列出已 capture 的 session |
| `drift show <id>` | 顯示 compacted session |
| `drift blame <file> [--line N] [--range A-B]` | **反查** |
| `drift trace <session-id>` | **順查** |
| `drift diff <event-id>` | 單一 event 的 unified diff |
| `drift rejected [--since DATE]` | 列出被駁回的 AI 建議 |
| `drift log [-- <git-args>]` | `git log` + 每個 agent 的 session 摘要 |
| `drift bind <commit> <session>` | 把 session 綁到 commit note |
| `drift auto-bind` | 依 timestamp 自動配對每個 session 跟最近的 commit |
| `drift install-hook` | 安裝 non-blocking 的 post-commit hook |
| `drift sync push\|pull <remote>` | push/pull `refs/notes/drift` |
| `drift config get\|set\|list` | 全域 + 專案 TOML 合併讀寫 |
| `drift mcp` | 啟動 stdio MCP server |

## Configuration

全域：`~/.config/drift/config.toml`
專案（覆寫）：`<repo>/.prompts/config.toml`

```toml
[attribution]
db_in_git = true          # default — 整個 team 透過 repo 共享 blame

[connectors]
claude_code = true
codex = true
aider = false             # feature-gated stub

[compaction]
provider = "anthropic"      # default；切 "mock" 完全離線 / 測試
model = "claude-haiku-4-5"  # 或 claude-sonnet-4-6 / claude-opus-4-7

[handoff]
model = "claude-opus-4-7"   # 敘事品質就是價值；要 ~30x 成本下降切
                            # "claude-haiku-4-5"
```

`ANTHROPIC_API_KEY`：走 live API compaction 路徑時必填。沒設的話 drift_ai 會
透明地 fall back 到 `MockProvider`，每份摘要都會打上 `[MOCK]` 標記，所以你不會
把 fallback run 誤當成 real run — pipeline 其他部分都不變。

drift 跟 Cursor / Copilot history、Cody、`git blame` 本身的對比見
[`docs/COMPARISON.md`](docs/COMPARISON.md)。

## 誠實的限制（v0.2.0）

- 人類編輯偵測只靠 SHA ladder — 我們不主張「作者是誰」，`human` slug 的意思是
  「沒有 AI session 產生這段內容」。詳見 VISION.md。
- `Bash python -c "open(...).write(...)"` 是 best-effort；shell lexer 漏掉的會被
  SHA ladder 接住、歸到 `human`。
- Codex 的 `reasoning` item 是加密的；我們會計數但不會把內容 surface 出來。
- 成本總額用內建的 hardcoded 計價表 — 當作正式 invoice 之前請核對
  <https://www.anthropic.com/pricing>。
- Context-window 截斷目前是決定性的 head+tail 省略（Strategy 1）；階層式摘要
  （Strategy 2）已 stub 在 feature flag 後等 v0.2 開啟。

## 關於

`drift` 是 [@ShellFans-Kirin](https://github.com/ShellFans-Kirin)
（[shellfans.dev](https://shellfans.dev)）獨立維護的開源專案。**不**隸屬於
Anthropic、OpenAI 或任何被 drift 整合的 agent vendor — `drift` 是蓋在他們的
session log 上面，不是他們做的。

> 這個工具最初是寫給自己用 — 那時候在 Codex 卡住跟 Claude 被 rate-limit 之間
> 不斷掉脈絡。v0.2 的 `drift handoff` 是我自己每天用得最多的部分。

## License

Apache 2.0 — 見 [LICENSE](LICENSE)。

## Contributing

見 [CONTRIBUTING.md](CONTRIBUTING.md) — 裡面用 Aider stub 當例子走完一遍「新增
connector」的流程。
