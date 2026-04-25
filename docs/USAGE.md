# Drift AI · 使用方式（v0.2.0+）

完整使用手冊。從零安裝到 `drift handoff`，每一步都附可複製的指令 + 範例輸出。

---

## 0. 先決條件

- macOS（Apple Silicon 或 Intel）/ Linux（x86_64 或 aarch64）
- 一個 git repo（drift 操作的目標）
- 至少一個 AI coding agent 的 session log:
  - Claude Code → 預設寫到 `~/.claude/projects/`
  - Codex → 預設寫到 `~/.codex/sessions/`
  - Aider → connector 是 stub，社群 PR 歡迎
- 想用 LLM compaction / handoff 的話：`ANTHROPIC_API_KEY` 環境變數。否則 drift 會 fall back 到 deterministic mock summary（標 `[MOCK]`）

---

## 1. 安裝

### 1.1 Homebrew（macOS arm64/x86_64 + Linuxbrew）

```bash
brew install ShellFans-Kirin/drift/drift
```

### 1.2 crates.io（需 Rust 1.85+）

```bash
cargo install drift-ai
```

注意：crate 名是 `drift-ai`，安裝後的 binary 名是 `drift`。

### 1.3 GitHub Releases pre-built tarball（不需 Rust toolchain）

```bash
curl -sSfL https://github.com/ShellFans-Kirin/drift_ai/releases/latest/download/drift-v0.2.0-$(uname -m)-unknown-linux-gnu.tar.gz \
  | tar xz -C /tmp && sudo mv /tmp/drift /usr/local/bin/drift
drift --version
# expect: drift 0.2.0
```

替換 `unknown-linux-gnu` 為 `apple-darwin` 給 macOS。

### 1.4 從原始碼

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo install --path crates/drift-cli
```

### 驗證

```bash
drift --version          # drift 0.2.0
drift --help             # 列出全部 17 subcommand
```

---

## 2. 第一次設定

### 2.1 在你的 git repo 裡 init

```bash
cd /path/to/your/repo
drift init
```

會建立：

- `.prompts/` 目錄
- `.prompts/config.toml`（預設值；你可以後續編輯）
- `.prompts/.gitignore`（讓 cache 不進 git）

`drift init` 是 idempotent — 重複跑不會覆蓋你已編輯的 `config.toml`。

### 2.2 第一次 `drift capture` 會印 privacy 注意事項

這是 v0.1.2 加的 first-run notice：

```
drift capture · first-run notice
  drift mirrors your AI session content (including anything you
  pasted) into .prompts/. events.db is committed to git by default.
  See docs/SECURITY.md for the full story.

  Press Enter to continue, Ctrl-C to abort.
```

按 Enter 繼續。`~/.config/drift/state.toml` 記住「已顯示過」，之後不會再問。

CI / 自動化情境用 `DRIFT_SKIP_FIRST_RUN=1` env var 跳過（但**不會**標記成已顯示，下次互動執行還會再問一次）。

---

## 3. 抓取 session（capture）

### 3.1 一次性

```bash
drift capture
# 預期: Captured N session(s), wrote M event(s) to .prompts/events.db
```

`drift capture` 會：

1. 跑掃描 `~/.claude/projects/` + `~/.codex/sessions/` 下所有 jsonl
2. 對每個 session 抽 `CodeEvent` 並寫進 `.prompts/events.db`（SQLite）
3. 對每個 session 跑 LLM compaction 寫成 `.prompts/sessions/<date>-<agent>-<short_id>.md`

### 3.2 Filter

```bash
drift capture --agent claude-code              # 只抓 Claude Code
drift capture --agent codex                    # 只抓 Codex
drift capture --session abc12345-xxx           # 只抓特定 session id
drift capture --all-since 2026-04-22T00:00:00Z  # 只抓某時間之後
```

### 3.3 Live mode（背景 daemon）

```bash
drift watch
# drift watch · event-driven; Ctrl-C to stop
#   watching /home/you/.claude/projects
#   watching /home/you/.codex/sessions
```

`drift watch` 用 platform 原生 FS event（FSEvents on macOS / inotify on Linux），每次 session 檔變動 200 ms debounce 內 re-capture。`Ctrl-C` 收尾當前 capture 後乾淨退出。狀態存到 `~/.config/drift/watch-state.toml`，restart 後 resume 而非全掃。

---

## 4. 🌟 v0.2 新功能：`drift handoff`

把進行中的 task 包成另一個 agent 接得起來的 markdown brief。

### 4.1 用法 — 三種 scope（擇一）

```bash
# 用 git branch 圈定範圍（最常用）
drift handoff --branch feature/oauth --to claude-code

# 用時間範圍（沒有專屬 branch 時）
drift handoff --since 2026-04-25T08:00:00Z --to codex

# 單一 session（debugging / unit test）
drift handoff --session abc12345-xxx --print
```

### 4.2 Target agent

```bash
--to claude-code     # 預設；footer 配 'paste this to claude' 提示
--to codex           # footer 配 codex 慣用語氣
--to generic         # 純 brief 沒 footer，pipe 到任何工具
```

差別只在 footer。Body 一致 — body 是 task 的內容，目前不刻意做 per-vendor 翻譯（v0.3+ 會做 tool-call schema adapter）。

### 4.3 Output

```bash
drift handoff --branch feat-x --to claude-code
# → .prompts/handoffs/2026-04-25-1530-feat-x-to-claude-code.md

drift handoff --branch feat-x --to claude-code --output ~/transfer.md
# → ~/transfer.md

drift handoff --branch feat-x --to claude-code --print | pbcopy
# → stdout，再 pipe 到 clipboard
```

### 4.4 預期執行流程

```
$ drift handoff --branch feature/oauth --to claude-code
⚡ scanning .prompts/events.db
⚡ extracting file snippets and rejected approaches
⚡ compacting brief via claude-opus-4-7
✅ written to .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md
  · model=claude-opus-4-7 · in=3421 out=612 · cost=$0.0972

next:
  claude
  # then paste:
  "I'm continuing this task. Read the handoff brief and resume from 'Next steps' #1:"
  "$(cat .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md)"
```

### 4.5 Brief 結構

每份 brief 都有同樣的 section：

```markdown
# Handoff Brief — `feature/oauth`

| Field | Value |
|---|---|
| From | codex × 2 + claude-code × 2 (4 sessions, 47 turns) |
| To | claude-code |
| Generated | 2026-04-25 15:30 UTC |
| Repo | owner/repo @ feature/oauth |
| Branch dif | +156 / -23 across 3 files |

## What I'm working on
[3-5 sentences high-level intent — LLM-compacted]

## Progress so far
- ✅ Done items
- ⏳ In-progress
- ⏸ Not started

## Files in scope
### `src/auth/login.ts` (modified, +47 / -3)
```
[code excerpt with modified ranges + ±5 lines context]
```

## Key decisions made
- Decision text *(citation: codex 7c2…, turn 4)*

## Approaches tried but rejected
- Pre-extracted from session tool_result errors

## Open questions / blockers
1. ...

## Next steps (suggested)
1. ...

## How to continue (paste this to claude-code)
> [paste-friendly resume prompt]
```

### 4.6 成本

| Model | 每次 handoff 大致成本 |
|---|---|
| `claude-opus-4-7` (預設) | ~\$0.10–0.30 |
| `claude-sonnet-4-6` | ~\$0.02–0.06 |
| `claude-haiku-4-5` | ~\$0.005–0.01 |

切換在 `.prompts/config.toml`：

```toml
[handoff]
model = "claude-haiku-4-5"
```

預設 Opus 是因為 brief 是下個 agent 逐字讀的東西，narrative quality 比 per-session compaction 重要。每天 handoff 個位數次的話 Opus 一個月 ~\$30；高頻使用建議切 Haiku。

---

## 5. AI-native blame（v0.1 既有功能）

### 5.1 反查：哪個 session 寫了這行？

```bash
drift blame src/auth/login.ts
# 整個檔案的時間線

drift blame src/auth/login.ts --line 42
# 單行的時間線

drift blame src/auth/login.ts --range 40-60
# 行範圍的時間線
```

範例輸出：

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

### 5.2 順查：這個 session 改了什麼？

```bash
drift trace abc12345-xxx
# 列出此 session 產生的所有 CodeEvent
```

### 5.3 整體 audit log

```bash
drift log
# 像 git log 但每個 commit 多一段 per-agent attribution

drift log -- --since 1.day
# 把後面的 args 透傳給 git log
```

範例：

```
commit abc1234 — Add OAuth login
   💭 [claude-code] 7 events accepted, 0 rejected
   💭 [codex]       3 events accepted, 1 rejected
   ✋ [human]       2 manual edits
```

### 5.4 看單一事件

```bash
drift diff <event-id>      # 顯示這個 event 的 unified diff
drift show <session-id>    # 顯示 session 的 compacted markdown
drift list                 # 列出所有 captured sessions
drift list --agent codex   # 只列 codex sessions
```

### 5.5 看被駁回的 AI 建議

```bash
drift rejected
drift rejected --since 2026-04-22T00:00:00Z
```

`rejected` event 來源：session 裡 tool_result 標 `is_error=true` 的那些。

### 5.6 把 session 跟 git commit 綁定

```bash
drift bind <commit-sha> <session-id>     # 手動綁
drift auto-bind                           # 自動依 timestamp 配對
drift install-hook                        # 安裝 post-commit hook 自動跑 auto-bind
```

綁定資料寫在 `refs/notes/drift`（git notes），不污染 commit history。

---

## 6. MCP server（給其他 AI tool 查 drift）

```bash
drift mcp
# 啟動 stdio MCP server
# 預設 tools: drift_blame / drift_trace / drift_rejected / drift_log / drift_show_event
```

註冊到 Claude Code（一行）：

```bash
claude mcp add drift -- drift mcp
```

註冊到 Codex：

```bash
codex mcp add drift -- drift mcp
```

之後可以在 Claude / Codex 對話中直接問「show me the drift blame for src/foo.rs:42」，他們會透過 MCP 呼叫 drift 查 attribution，不需要切到 shell。

MCP 介面是**唯讀**的 by design — 任何寫入動作（capture / bind / sync）都只在 CLI 提供。

---

## 7. 帳務透明：drift cost

`drift handoff` 跟 `drift capture` 的 LLM 呼叫都記到 `events.db` 的 `compaction_calls` 表：

```bash
drift cost
# drift cost — compaction billing
#   total calls      : 10
#   input tokens     : 120958
#   output tokens    : 6582
#   total cost (USD) : $0.1539
```

更細的分組：

```bash
drift cost --by model
drift cost --by session
drift cost --by date
drift cost --since 2026-04-20T00:00:00Z --until 2026-04-25T00:00:00Z
drift cost --model claude-haiku-4-5
```

---

## 8. 設定（`.prompts/config.toml`）

完整範本：

```toml
[attribution]
db_in_git = true             # 預設 true，events.db 進 git。改 false 留本機。

[connectors]
claude_code = true
codex = true
aider = false                # feature-flag 的 stub

[compaction]
provider = "anthropic"       # 預設；改 "mock" 完全離線
model = "claude-haiku-4-5"   # 或 claude-sonnet-4-6 / claude-opus-4-7

[handoff]
model = "claude-opus-4-7"    # narrative quality 重要；可切 haiku 省 30×

[sync]
notes_remote = "origin"      # `drift sync push/pull` 用的 remote
```

兩層：

- 全域：`~/.config/drift/config.toml`
- 專案（覆寫）：`<repo>/.prompts/config.toml`

```bash
drift config get handoff.model
drift config set handoff.model claude-haiku-4-5
drift config list
```

---

## 9. 跨機器同步（git notes）

```bash
drift sync push origin   # 推 refs/notes/drift 到 remote
drift sync pull origin   # 從 remote 拉 refs/notes/drift
```

drift 的 attribution 連結（哪個 session 對應哪個 commit）存在 `refs/notes/drift`。Push / pull 用 git 自帶的 notes 機制，不會跟 main code 的 commit 對到一起。

---

## 10. 安全 / Privacy

drift **不會** scrub session content。任何你貼進 Claude / Codex chat 的內容（包括手滑貼上的 secret）會進 `.prompts/sessions/*.md` 跟 `events.db`，預設這些都會被 commit 到 git。

三條 mitigations：

1. **關閉 git side**：
   ```toml
   [attribution]
   db_in_git = false
   ```
   `events.db` 跟 markdown 留本機，team 失去共享 blame，你保有本地 view。

2. **手動 review 再 commit**：
   ```bash
   drift capture
   git diff --cached -- .prompts/
   git add .prompts/ && git commit
   ```

3. **配 secret scanner pre-commit hook**（drift 不附帶）：
   ```bash
   # .git/hooks/pre-commit
   gitleaks protect --staged --redact -v || exit 1
   ```

完整 threat model + roadmap 見 [`docs/SECURITY.md`](SECURITY.md)。

---

## 11. 進階：增加新 connector

drift 現支援 Claude Code + Codex；想加 Cursor / Cline / 自己的 agent？

實作 `SessionConnector` trait（在 `crates/drift-connectors/src/lib.rs`）：

```rust
pub trait SessionConnector {
    fn agent_slug(&self) -> &'static str;
    fn discover(&self) -> Result<Vec<SessionRef>>;
    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession>;
    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>>;
}
```

`crates/drift-connectors/src/aider.rs` 是 stub，[`CONTRIBUTING.md`](../CONTRIBUTING.md) 用 Aider 當 worked example 走過 add-a-connector 全流程。歡迎 PR。

---

## 12. 完整指令對照表

| Command | 用途 |
|---|---|
| `drift init` | scaffold `.prompts/` |
| `drift capture [--agent A] [--session ID] [--all-since DATE]` | 抓 session、跑 compaction |
| `drift watch` | 背景 daemon，event-driven 自動 capture |
| `drift handoff [--branch B \| --since ISO \| --session ID] [--to A] [--output P \| --print]` | **v0.2** 跨 agent task brief |
| `drift list [--agent A]` | 列出 captured sessions |
| `drift show <id>` | 顯示 compacted session |
| `drift blame <file> [--line N \| --range A-B]` | 反查：哪個 session 改了這行 |
| `drift trace <session>` | 順查：這個 session 改了什麼 |
| `drift diff <event>` | 單一 event 的 unified diff |
| `drift rejected [--since DATE]` | 列出被駁回的 AI 建議 |
| `drift log [-- <git args>]` | git log + per-agent attribution |
| `drift cost [--since --until --model --by]` | 帳務 |
| `drift bind <commit> <session>` | 手動綁 commit ↔ session |
| `drift auto-bind` | 自動配對 commit ↔ session by timestamp |
| `drift install-hook` | 裝 post-commit hook 自動 auto-bind |
| `drift sync push\|pull <remote>` | 同步 `refs/notes/drift` |
| `drift config get\|set\|list` | 讀寫 config |
| `drift mcp` | 啟動 stdio MCP server |

---

## 13. Troubleshooting

### "drift handoff: no sessions matched scope"

`--branch` scope 抓不到 sessions 的常見原因：

1. 你還沒 `drift capture` — 跑一次
2. branch 名字不對 — 你的 git branch 是不是 `feat/x` 而不是 `feature/x`？
3. 那個 branch 還沒有 commit 上去 — `--branch` 用 git log 找 divergent commit timestamp 當 lower bound；空 branch 會 fall back 到 14 天

退而求其次用 `--since 2026-04-22T00:00:00Z` 試。

### "ANTHROPIC_API_KEY not set — falling back to deterministic mock summary"

handoff 在沒設 API key 時會跑 MockProvider，brief 會明顯標 `[MOCK]`。設 env var 或在 `.prompts/config.toml` 把 `provider = "mock"` 改回 `"anthropic"`。

### `drift watch` 不觸發

- 確認你的 agent 真的在寫 jsonl 到 `~/.claude/projects/` 或 `~/.codex/sessions/`
- macOS: `Sandbox / Full Disk Access` 權限可能擋 FSEvents — 給 terminal app 完整磁碟存取
- Linux: `/proc/sys/fs/inotify/max_user_watches` 可能太低 — `sudo sysctl fs.inotify.max_user_watches=524288`

### crates.io 下載失敗

- 檢查 `crates.io` 帳號 email 已驗證
- 確認你用 `cargo install drift-ai`（連字符）而不是 `drift_ai`（底線）
- 從 `/tmp` 乾淨環境試一次：
  ```bash
  CARGO_HOME=/tmp/drift-clean cargo install drift-ai --locked
  /tmp/drift-clean/bin/drift --version
  ```

### Homebrew install 找不到 formula

- 確認你 `brew tap` 的是 `ShellFans-Kirin/drift`（大小寫敏感與否依 OS / brew 版本而定 — 都試試）
- `brew update` 同步最新 tap state
- macOS Intel runner 已停 — Intel Mac 仍可裝（Formula 帶 x86_64-apple-darwin tarball）

---

## 14. 相關文件

| 文件 | 內容 |
|---|---|
| [README](../README.md) | 30 秒第一印象 |
| [CHANGELOG](../CHANGELOG.md) | 版本歷史 |
| [docs/VISION.md](VISION.md) | 整個專案的 north star |
| [docs/SECURITY.md](SECURITY.md) | Threat model 完整版 |
| [docs/COMPARISON.md](COMPARISON.md) | vs Cursor / Copilot chat / Cody / git blame |
| [docs/V020-DESIGN.md](V020-DESIGN.md) | v0.2 `drift handoff` 設計提案 |
| [docs/V020-DEV-LOG.md](V020-DEV-LOG.md) | v0.2 開發週期 + 執行結果完整記錄 |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | 加新 connector 的逐步指南 |
