# Phase 0 — 驗證報告

**日期**：2026-04-22
**狀態**：Phase 0 交付物已對齊修訂後規格並驗證通過，等候 `approve` 後進入 Phase 1。
**Branch**：`phase0-proposal`
**相關文件**：[PHASE0-PROPOSAL.md](PHASE0-PROPOSAL.md)、[PHASE0-EXECUTION-REPORT.md](PHASE0-EXECUTION-REPORT.md)

本報告是 2026-04-22 驗證回合的人類可讀摘要：重新實測主機環境、比對磁碟上的
session schema 與既有提案內容是否仍吻合，並標註文件與現況之間的微漂移。

---

## 1. 交付物連結

| 項目 | URL |
|---|---|
| Repo | https://github.com/ShellFans-Kirin/drift_ai |
| Draft PR #1 | https://github.com/ShellFans-Kirin/drift_ai/pull/1 |
| PHASE0-PROPOSAL.md | https://github.com/ShellFans-Kirin/drift_ai/blob/phase0-proposal/docs/PHASE0-PROPOSAL.md |
| PHASE0-EXECUTION-REPORT.md | https://github.com/ShellFans-Kirin/drift_ai/blob/phase0-proposal/docs/PHASE0-EXECUTION-REPORT.md |
| 本檔案 | https://github.com/ShellFans-Kirin/drift_ai/blob/phase0-proposal/docs/PHASE0-VERIFICATION.md |

---

## 2. 推薦技術棧（一句話）

**Rust** — 單一 binary 分發、`notify` 背景 daemon、`similar` + `rusqlite`
讓 attribution 兩條熱路徑（diff 計算與 `events.db`）都乾淨好寫。

## 3. Package 命名提案

| 層 | 名稱 |
|---|---|
| Cargo crate | `drift-ai`（hyphen，自動映射 `drift_ai` module） |
| Go module | `github.com/ShellFans-Kirin/drift_ai` |
| npm | `drift-ai` |
| **CLI binary** | **`drift`** |
| git notes ref | `refs/notes/drift` |
| SQLite app id | `drift_ai` |

---

## 4. 雙 agent seed 結果

| Agent | Seed | 磁碟證據 | 抓到什麼 |
|---|---|---|---|
| Claude Code | ✅ | `~/.claude/projects/-tmp-drift-seed-claude/40c15914-...jsonl` | `Write` + `Edit` tool_use 信封完整。Sandbox 擋了實際寫入，但配對的 `tool_result.is_error: true` **正好就是 attribution 層要的 `rejected` 訊號來源**。 |
| Codex | ✅ | `~/.codex/sessions/2026/04/21/rollout-...06-59-16-...jsonl` | `apply_patch` (custom_tool_call) + `exec_command` (function_call) 信封完整。`*** Begin Patch / Add File / Update File / Delete File / Move File` 文法確認，已 diff-shaped，直接解析即可。 |

兩個 agent 的檔案操作 schema 都是 **真實證據確認**，不是從官方文件推導。

---

## 5. 資料模型對 4 個需求的自我評估

| 需求 | 模型處理 | 誠實？ |
|---|---|---|
| **Multi-origin**（一行被多個 agent 多次動過） | 每次 touch 一筆 `CodeEvent`；`parent_event_id` 串血統；`drift blame` 依時間順走 chain。 | ✅ 原生支援 |
| **人類手改偵測** | SHA-256 ladder：每個成功的 AI event 記 `content_sha256_after`；下次同步時重新 hash，不同就發一筆 `agent_slug="human"` 的 event。**不主張作者身分** — `human` 只表「沒有 AI session 產生這個變動」，這是唯一誠實的說法。 | ✅ 帶語意局限，文件有寫 |
| **被拒絕的建議** | `rejected: bool` 欄位。Claude 的 `tool_result.is_error = true` 或 Codex 的 `function_call_output` 回失敗時設為 true — seed 已觀察到。 | ✅ 原生支援 |
| **Rename 血統** | Tier 1：`shell_lexer.rs` 解析 `apply_patch *** Move File`、`Bash mv`、`git mv`。Tier 2：`git log --follow` fallback。`drift blame` 跨 rename 沿 `parent_event_id` 走。 | ✅ 明確標註 Tier 2 是 best-effort（git 用 50% 相似度閾值） |

**沒有 string-hack**：沒有任何需求是靠把值硬塞進不屬於它的欄位達成。

誠實缺口（已寫進 [PROPOSAL §F](PHASE0-PROPOSAL.md#f-self-evaluation-does-the-data-model-honor-the-four-requirements)）：

- `Bash python -c "open(...).write(...)"` 對 shell lexer 是透明的。SHA ladder
  還是抓得到檔案變動，只是會被歸給 `human` 而非跑這段 python 的 AI。v0.1.0
  可接受。
- Codex `reasoning` 項是加密的；我們只計數，不展示。
- Claude `MultiEdit` 會拆成多筆 `CodeEvent`，用 intra-call `parent_event_id`
  串起來 — 稍微延展了「一次 tool call 一個 event」的定義，但換得正確的
  per-line attribution。

---

## 6. [NEEDS-INPUT] — 四個項目，按爆炸半徑排

| # | 項目 | 阻擋什麼 | 你只說 `approve` 時的預設值 |
|---|---|---|---|
| 1 | 技術棧 approve | Phase 1 啟動 | **Rust** |
| 2 | `ANTHROPIC_API_KEY` | Phase 3 真 API smoke + 人類手改 demo 截圖 | 跑 Mock-only、跳過 demo 截圖 |
| 3 | `attribution.db_in_git` 預設 | Phase 1 config schema | **`true`**（方便團隊協作 blame） |
| 4 | `human` slug 語意 | Phase 4 README 文案 | **「沒有 AI session 產生這個變動」**（事件 timeline，不是作者判定） |

只有 #1 阻擋 Phase 1 啟動。#2–4 可以晚點給，爆炸半徑較小。

---

## 7. 主機環境 — 2026-04-22 實測

| 工具 | 版本 | 狀態 |
|---|---|---|
| `git` | 2.43.0 | OK |
| `gh` | 2.45.0 | ✅ 以 `shellfans-dev` 登入 |
| `node` | 18.19.1 | OK |
| `python3` | 3.12.3 | OK |
| `rustc` / `cargo` | 1.75.0 | OK |
| `go` | 1.22.2 | OK |
| `claude` (Claude Code) | **2.1.117** | 微漂移：PROPOSAL / EXECUTION-REPORT 寫 `2.1.116` |
| `codex` | codex-cli 0.122.0 | OK |
| `git config user.name / user.email` | kirin / kirin@shell.fans | OK |
| `ANTHROPIC_API_KEY` | **未設定** | `[NEEDS-INPUT]` — 只阻擋 Phase 3 真 API smoke |
| `gh repo view ShellFans-Kirin/drift_ai` | 存在 | 預先建好（本次未建立新 repo） |

**與先前文件的漂移**：
- Claude Code 從 `2.1.116` 升到 `2.1.117`。無關 schema 結論，Phase 1 動這些
  文件時會一併修正。

**Repo 現況**：
- `/home/kirin/drift_ai` 已 clone，追蹤 `origin/phase0-proposal`。
- `git status`：乾淨。
- Branches：`main`、`phase0-proposal`（local + origin 皆有）。
- `phase0-proposal` 上的 commit：
  ```
  5443cf7 docs: add Phase 0 execution report
  9a3afb8 docs: phase 0 rev 2 — add line-level attribution data model
  aba36b2 docs: add Phase 0 proposal (host inventory, stack, MVP scope, JSONL schema analysis)
  6bb7e44 chore: rename project to drift_ai (CLI binary: drift)
  2f1f5c1 chore: scaffold repo (Apache 2.0 license, README stub, .gitignore)
  ```

---

## 8. 與修訂規格的對照表

本次驗證檢查了修訂規格列出的每一項 Phase 0 交付物，是否都在已 commit 的
文件裡找得到對應。

| 規格要求 | 對應章節 |
|---|---|
| A. 主機環境（版本、auth、seed 路徑） | [PROPOSAL §A](PHASE0-PROPOSAL.md#a-host-environment-inventory)、[EXECUTION §2–3](PHASE0-EXECUTION-REPORT.md#2-host-environment-inventory)、本檔 §7 |
| A. 雙 agent 檔案操作欄位 | [PROPOSAL §C](PHASE0-PROPOSAL.md#c-jsonl-schema-analysis-file-op-focused)、[EXECUTION §4](PHASE0-EXECUTION-REPORT.md#4-jsonl-schema-evidence) |
| A. drift_ai repo 可存取、不建新 repo | 本檔 §1 + §7 |
| B. 技術棧 3 選項比較 + 推薦 | [PROPOSAL §B](PHASE0-PROPOSAL.md#b-technical-stack--three-options)、本檔 §2 |
| B. Package 命名提案 | [PROPOSAL §B 命名表](PHASE0-PROPOSAL.md#package--binary-naming)、本檔 §3 |
| C. MVP 範圍（connector、compaction、git、檔案樹） | [PROPOSAL §E](PHASE0-PROPOSAL.md#e-mvp-scope) |
| D.1 NormalizedSession | [PROPOSAL §D.1](PHASE0-PROPOSAL.md#d1-normalizedsession-session-layer) |
| D.2 CodeEvent（全部必要欄位） | [PROPOSAL §D.2](PHASE0-PROPOSAL.md#d2-codeevent-line-layer--the-new-core-record) |
| D.3 人類手改偵測策略 | [PROPOSAL §D.3](PHASE0-PROPOSAL.md#d3-human-edit-detection-sha-256-ladder) |
| D.4 Rename 處理（兩層） | [PROPOSAL §D.4](PHASE0-PROPOSAL.md#d4-rename-handling) |
| D.5 儲存結構 + `db_in_git` 開關 | [PROPOSAL §D.5](PHASE0-PROPOSAL.md#d5-storage-layout) |
| D. Schema 圖 | [PROPOSAL §D schema picture](PHASE0-PROPOSAL.md#schema-picture-mermaid)（Mermaid ER） |
| E. clone → 寫 proposal → branch → draft PR | 本檔 §1（PR #1 已 open + draft） |

所有規格項目皆有對應，無缺口。

---

## 9. 下一關

回覆 **`approve`**（套用 §6 預設值）即可啟動 Phase 1。
或明示覆寫，例如 **`approve, stack=go, db_in_git=false`**。

一旦 approve，Phase 1 → 2 → 3 → 4 會一路跑到底不再停。
