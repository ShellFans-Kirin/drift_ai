> 🌐 [English](../../SECURITY.md) · [日本語](../ja/SECURITY.md) · [简体中文](../zh-Hans/SECURITY.md) · **繁體中文**

# 安全與隱私

Drift AI 是一個 local-first 的工具。它捕捉的資料 — 你的 AI coding session — 可
能包含你不想 commit 進 repo 的東西。這份文件誠實地把 threat model 寫清楚。

## Threat model

Drift **沒有 server**。Drift 自己不會把任何東西上傳到任何地方。資料流是：

1. 你的 AI agent（Claude Code、Codex…）把 session JSONL 寫進它自己在
   `~/.claude/projects/` 或 `~/.codex/sessions/` 下的目錄。這是 agent 的行為，
   不是 Drift 的。
2. `drift capture` 讀那些檔案，並寫出：
   - `code_events` rows 進 `<repo>/.prompts/events.db`（SQLite）
   - 每個 session 對應一份 Markdown 進 `<repo>/.prompts/sessions/`
3. 如果 `[compaction].provider = "anthropic"`（預設），`drift capture` 會把每份
   session transcript 送到 `api.anthropic.com/v1/messages` 來產生 Markdown
   摘要。**這是唯一的 network egress。** 切到 `provider = "mock"` 可以完全
   跳過這一步。

第 3 步以外的所有東西都留在你機器上。

## 目前的限制（v0.1.x）

下列是**已知**且**已被記錄**的，不是 bug：

1. **`drift capture` 不會 scrub session 內容。** 你打進 Claude / Codex chat 的
   任何內容 — 包含手滑貼進去的 secret — 會原樣鏡像到 `events.db` 跟
   `.prompts/sessions/*.md`。
2. **`events.db` 預設會被 commit 進 git**（`[attribution].db_in_git = true`）。
   這個預設值的本意是讓 team 共享 blame；副作用是 session 裡漏出的 secret 會
   進 public repo。
3. **`.prompts/sessions/*.md` 是人類可讀的**：compacted 摘要保留檔名、決策、
   而且常常逐字保留 diff hunk。Anthropic 的 compactor 也不會主動 redact secret。

如果你曾經把 `export AWS_SECRET_ACCESS_KEY=AKIA...` 之類的東西貼進 Claude
session，那串字串就會落到 `events.db`，也可能落進 compacted Markdown。

## 目前可用的 mitigation

挑一個最適合你 workflow 的：

1. **關掉 git 那一側**：
   ```toml
   # .prompts/config.toml
   [attribution]
   db_in_git = false
   ```
   `events.db` 跟 markdown 都只留本機。Team 失去共享 blame；你保有本地 view。

2. **commit 前手動 review**：
   ```bash
   drift capture
   git diff --cached -- .prompts/
   # 真的把摘要讀過一遍；需要 redact 就直接改檔
   git add .prompts/ && git commit
   ```

3. **配 secret scanner 當 pre-commit hook**。Drift 不附帶，但
   [gitleaks](https://github.com/gitleaks/gitleaks) 跟
   [trufflehog](https://github.com/trufflesecurity/trufflehog) 能抓到大部分
   pattern。範例：
   ```bash
   # .git/hooks/pre-commit
   gitleaks protect --staged --redact -v || exit 1
   ```

4. **完全離線跑**：設 `[compaction].provider = "mock"` 並 unset
   `ANTHROPIC_API_KEY`。你會失去 LLM 摘要，但 `events.db` 還能單純當本地索引
   使用。

5. **如果預期會把 secret 貼進 chat，先別在那個 repo 上啟用 Drift**，等 v0.2
   的 redaction pass 落地。等一下不丟臉。

## Roadmap（v0.2+）

下列是 planned work，不是有日期的承諾：

- **`drift capture` 內 regex-based redaction pass** — 認得高信心度的 pattern
  （Anthropic / OpenAI / AWS / GitHub PAT / Slack / private key blob）並換成
  `<redacted>` placeholder，在 `events.db` 之前就替換掉。
- **互動式 review mode**：`drift capture --review` 把每份產生的 Markdown 開到
  `$EDITOR` 確認後才落檔。
- **可插拔 detector**：選用 `trufflehog` / `gitleaks` 規則的整合，不重新發明
  regex。
- **`drift redact <session-id>`**：對已 capture 的 session 做事後 scrub，附
  明確的 undo 路徑。

如果你需要其中任何一項提前實作，開 feature request — 真實 use case 會推
高優先級。

## 回報安全問題

任何可能導致 credential 洩漏、supply-chain 風險、或 RCE 的問題，請走
[GitHub Security Advisories](https://github.com/ShellFans-Kirin/drift_ai/security/advisories/new)。
不要開 public issue。

文件補完、threat model 修正、「你漏了 mitigation X」之類的 — 一般 issue 或
PR 就好。
