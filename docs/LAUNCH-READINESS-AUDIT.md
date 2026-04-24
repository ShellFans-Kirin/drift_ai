# Drift AI v0.1.1 — Launch Readiness Audit

**審計日期**：2026-04-24
**審計範圍**：4 項獨立驗證（Homebrew / README / Q&A / 發行通路）
**審計者**：Claude Code（獨立驗證者角色，**不動任何 code / config / README**）
**branch**：`audit/launch-readiness`

---

## Executive Summary

Drift AI v0.1.1 的**發行工程面**已經紮實（Homebrew、crates.io、4 target binary、自動化 pipeline 都端到端驗過），**但社群登場面**有一個實質風險會在 HN 前 30 則留言引爆：**使用者的 Claude/Codex session 裡夾帶的 secret 會被 `drift capture` 原樣寫進 `.prompts/sessions/*.md` 跟 `events.db`，repo 沒有 `SECURITY.md`，code 裡也沒有任何 scrub / redact / mask 邏輯**。HN 上對「local-first AI blame」這個 pitch 最敏感的那批訪客，一定會先問這個。

結論：

🟡 **YELLOW — 1-2 小時小修後可發**

最大 blocker 不是技術、不是 URL、不是 Formula — 是 `docs/SECURITY.md` 不存在，README 沒有 secret-handling 段落，且 `drift capture` 的設計沒有對 session content 做任何過濾。另外有 2 個 nice-to-have 可同時搞定（Q&A 補 author 獨立性、README 第一行加一句更直球的 pain statement）。

---

## 審計項目 1：Homebrew 安裝鏈路

**結論：🟢 高信心通過（85%+）**

### 1A. Formula 本體 + URL + sha256

`https://raw.githubusercontent.com/ShellFans-Kirin/homebrew-drift/main/Formula/drift.rb`（1378 bytes）完整讀到，4 個 target 全部檢查：

| Target | URL HTTP | Formula sha256 | 實際 tarball sha256 | Sidecar sha256 | Verdict |
|---|---|---|---|---|---|
| aarch64-apple-darwin | 200 | `21df1a60…` | `21df1a60…` | `21df1a60…` | **MATCH ✓** |
| x86_64-apple-darwin | 200 | `d3c8a537…` | `d3c8a537…` | `d3c8a537…` | **MATCH ✓** |
| aarch64-unknown-linux-gnu | 200 | `fb6b4012…` | `fb6b4012…` | `fb6b4012…` | **MATCH ✓** |
| x86_64-unknown-linux-gnu | 200 | `d3357498…` | `d3357498…` | `d3357498…` | **MATCH ✓** |

所有 4 個 tarball 都實際下載過，本機算 sha256 跟 Formula 寫的、跟 release 掛載的 `.sha256` sidecar，三者完全一致。

### 1B. Formula 語法 + Homebrew 慣例

```
$ ruby -c drift.rb
Syntax OK
```

10/10 慣例檢查全過：

- ✓ `class Drift < Formula`（對應 `drift.rb`）
- ✓ `desc` 長 76 字元（<80），首字大寫，結尾無標點
- ✓ `homepage "..."` 存在
- ✓ `test do ... end` 區塊存在
- ✓ `on_macos` / `on_linux` 結構完整
- ✓ `def install` + `bin.install "drift"`
- ✓ `license "Apache-2.0"`

### 1C. 模擬安裝（Linux host 無 brew 的 workaround）

在本機（`x86_64`）跑下列流程，等同 Homebrew 的實際行為：

```bash
curl -sSfL <formula url> -o drift.tar.gz
sha256sum drift.tar.gz        # d335749816f294a9e478c820f03c16dacb0fb8c66ba25d5377cfb6d362ba0bbc ✓
tar xzf drift.tar.gz
./drift --version             # drift 0.1.1 ✓
file drift                    # ELF 64-bit LSB pie executable, x86-64, dynamically linked, stripped
```

MCP server smoke（JSON-RPC initialize → `tools/list`）：

```
$ ./drift mcp
→ {"jsonrpc":"2.0","id":1,"result":{"capabilities":{"tools":{}},"protocolVersion":"2024-11-05","serverInfo":{"name":"drift","version":"0.1.1"}}}
→ {"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"drift_blame",...},{"name":"drift_trace",...},{"name":"drift_rejected",...},{"name":"drift_log",...},{"name":"drift_show_event",...}]}}
```

MCP 5 個 tool 全部暴露。

### 1D. 判斷

加上昨天已在真實 Mac mini (Apple M4 / macOS 26.3.1) 跑完整 `brew tap` + `brew install drift` + `drift --version` + uninstall 的端到端驗證（見 `docs/BREW-VERIFY-MAC-V0.1.1.md`），**Homebrew 鏈路無 blocker**。

---

## 審計項目 2：README 第一屏的 launch readiness

**結論：🟡 建議微調（2/3 完整命中，1/3 partial）**

### 前 50 行的實際內容

第 1 行：`# drift_ai`
第 3 行 tagline：`> AI-native blame for the post-prompt era. Local-first.`
第 5-10 行 description：`drift watches the local session logs of your AI coding agents...`
第 15-20 行 Demo 1：`drift log` 多 agent attribution（code block，含 emoji 💭/✋）
第 24-36 行 Demo 2：`drift blame` line-level timeline（視覺清楚）
第 38-42 行 Thesis：「commit granularity is too coarse to be the source of truth in the AI era」
第 44 行 `## Install`
第 49 行：`brew install ShellFans-Kirin/drift/drift`

### 三項指標

| 指標 | 結果 | 評語 |
|---|---|---|
| **1. 痛點陳述**（前 5 行） | **PARTIAL** | Tagline 第 3 行「AI-native blame for the post-prompt era」隱喻痛點，但是正面講「這是什麼」的比例高。真正的 pain sentence（「commit granularity is too coarse」）要讀到第 38 行才出現。8 秒掃描者可能會停在第 5-10 行的「is what」型描述 |
| **2. Demo 視覺化**（前 50 行） | **✓ 滿分** | 兩個 code block（lines 15-20 + 24-36）都展示實際 terminal 輸出，有 emoji 結構化區分 claude/codex/human，diff 片段直觀 |
| **3. 一行安裝指令**（前 50 行） | **✓** | `brew install ShellFans-Kirin/drift/drift` 在第 49 行，剛好卡進第一屏尾巴 |

### 扣分項審查

- **Badge 牆**：`grep shields.io | wc -l` = **0**（無 badge）✓
- **ASCII logo**：無 ✓
- **ToC 擋在痛點前**：無 ✓

### 建議微調

若要從 🟡 升 🟢，一個 4-5 行的插入點就夠：

把第 3 行 tagline 之後、第 5 行 description 之前，補一個 1-sentence pain 統合：

```md
> AI-native blame for the post-prompt era. Local-first.

**Problem**: Your commit says "Add OAuth login." Your reality was 47 prompts
to Claude + 3 Codex fills + 12 manual edits. Commit granularity can't tell
that story. `drift` keeps every line tied to the prompt that produced it —
locally, in your repo, forever.

`drift` watches the local session logs...
```

3 分鐘編輯工作，非 blocker。

---

## 審計項目 3：前 20 則 HN 留言的 Q&A 準備度

**結論：🔴 一題 RISK（Q3 secret handling） + 🟡 三題 PARTIAL/MISSING**

| # | 問題 | 狀態 | 證據 / 理由 |
|---|---|---|---|
| 1 | vs Cursor / Cody / Copilot session history | **PARTIAL** | `docs/VISION.md` 3 處提到 Cursor 都是「未來 connector 目標」，無直接對比。`grep -i cody` / `grep -i copilot`：0 matches。作者要即興回答「我們跟 Cursor 的對話歷史有什麼不同」|
| 2 | 為什麼不直接寫 git commit message | **✓ COVERED** | `README.md:38` 直球 thesis「commit granularity is too coarse」；`docs/VISION.md:83` 具體展開 |
| 3 | Claude session 裡的 API key / secret 會被 commit 嗎？ | **🔴 RISK（最大問題）** | (1) 無 `SECURITY.md`；(2) `grep -rE 'scrub\|redact\|mask' crates/` 返回 0 — **code 沒有任何 secret 過濾邏輯**；(3) `drift capture` 會把 session 的 tool_call input 原樣寫入 `events.db` + `.prompts/sessions/*.md`，如使用者曾在 Claude session 裡貼過 secret，該 secret 會跟著 repo 一起 commit。HN 留言會秒問這題 |
| 4 | 我不用 Claude / Codex，這工具對我沒用吧？ | **✓ COVERED** | `CONTRIBUTING.md:21-60+` 有「Adding a new connector」完整章節；`SessionConnector` trait `pub trait`（`crates/drift-connectors/src/lib.rs:29`）；Aider 是 worked stub |
| 5 | 跑 compaction 要 \$0.15-\$3，太貴，能用本地 LLM 嗎 | **PARTIAL** | `docs/VISION.md:124` 用中文提到「未來可接 OpenAI、本地 Ollama、任何符合 `CompactionProvider` 介面的實作」— 但 **只在 VISION.md 中文版**，README 沒提；HN 讀者多半看 README，會以為沒規劃 |
| 6 | License | **✓ COVERED** | `LICENSE` 存在（Apache-2.0 全文，11KB）；`README.md:245-247` 引用；無 badge 但不必要 |
| 7 | 為什麼 SQLite 不是 markdown / git notes | **✓ COVERED（最紮實）** | `docs/PHASE0-PROPOSAL.md:71-89` 有完整 stack 比較（Rust+rusqlite vs Go+sqlite3 vs TS+better-sqlite3）；`VISION.md:41,191` 強調 single-binary + zero-deps；`STEP1-5-COMPLETION-REPORT.md:72`「zero-deps, tx-safe, queries are fast」|
| 8 | 作者是 Anthropic / OpenAI 員工嗎？ | **MISSING** | repo 無 "About" / "Affiliation" / "Independent project" 字樣；`git log` author 一律 `ShellFans-Kirin <kirin@shell.fans>` — 個人品牌但沒明講獨立。HN 對 AI 工具的 conflict-of-interest 敏感度高 |

### Q3 的具體風險說明（給作者 briefing）

這不是「假裝關切」層級的問題，是實質的：

- `crates/drift-connectors/src/claude_code.rs` 的 `extract_code_events()` 會把每個 tool_call 的 `input` 以原始 JSON 寫入 `events.db` 的 `metadata` 欄
- `crates/drift-core/src/compaction.rs` 的 `render_transcript()` 把每個 turn 的 `content_text` 原樣塞進送給 Anthropic 的 prompt，Anthropic 的回應再寫到 `.prompts/sessions/*.md`
- `.prompts/` 預設 `[attribution].db_in_git = true` — events.db 會被 commit
- 如果使用者的 Claude session 曾有：
  - `export AWS_SECRET_ACCESS_KEY=AKIA...`
  - 「這是我的 database password 是 hunter2，幫我 debug 連線」
  - 貼了一段含 JWT 的 curl 指令
  
  這些明文都會進 repo，推到 public 後公開可讀

HN 第 5 則留言大概會是：「nice idea but your events.db contains my full chat history including API keys I accidentally pasted, and you commit it to git by default? hard pass」

### 最低限度的 launch-day 對策（不需要寫新 code）

只要新增 `docs/SECURITY.md` + 在 README 加一段 7 行的 Privacy 章節：

```md
## Privacy & secrets

**drift does not scrub your session content.** Anything you typed into
your Claude Code / Codex session — including secrets you may have
pasted — gets mirrored into `.prompts/` and, by default, committed
to your repo.

Two knobs to mitigate:

1. `[attribution].db_in_git = false` in `.prompts/config.toml` — keeps
   `events.db` local only (blame still works for you, not for your team).
2. Run `drift capture` in a dry-run audit mode before committing
   (planned for v0.2 — until then, manually inspect `.prompts/sessions/`
   before `git add`).

If you routinely paste secrets into chat sessions, **do not enable
drift yet**. A redaction pass is on the roadmap.
```

誠實寫出來比裝沒問題好 10 倍。HN 這樣 counter 就很難繼續追打。

---

## 審計項目 4：發行通路 URL 狀態（匿名訪問）

**結論：🟢 全綠**

### A. GitHub 表面（7/7）

```
200  https://github.com/ShellFans-Kirin/drift_ai
200  https://github.com/ShellFans-Kirin/drift_ai/releases/latest
200  https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.1.1
200  https://github.com/ShellFans-Kirin/drift_ai/blob/main/README.md
200  https://github.com/ShellFans-Kirin/drift_ai/blob/main/LICENSE
200  https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/VISION.md
200  https://github.com/ShellFans-Kirin/drift_ai/blob/main/CHANGELOG.md
```

### B. Release assets（8/8，4 tarball + 4 sha256）

```
200  drift-v0.1.1-aarch64-apple-darwin.tar.gz (+.sha256)
200  drift-v0.1.1-x86_64-apple-darwin.tar.gz (+.sha256)
200  drift-v0.1.1-aarch64-unknown-linux-gnu.tar.gz (+.sha256)
200  drift-v0.1.1-x86_64-unknown-linux-gnu.tar.gz (+.sha256)
```

### C. Homebrew tap（2/2）

```
200  https://github.com/ShellFans-Kirin/homebrew-drift
200  https://raw.githubusercontent.com/ShellFans-Kirin/homebrew-drift/main/Formula/drift.rb
```

### D. crates.io（4/4，附註）

| Crate | Web 200* | `max_version` | repository / homepage |
|---|---|---|---|
| drift-ai | ✓ | 0.1.1 | `https://github.com/ShellFans-Kirin/drift_ai` |
| drift-core | ✓ | 0.1.1 | 同上 |
| drift-connectors | ✓ | 0.1.1 | 同上 |
| drift-mcp | ✓ | 0.1.1 | 同上 |

\* **審計盲點 flag**：crates.io 的 `/crates/<name>` 是 SPA，`curl -sI` 或無 Accept 頭的 GET 都回 404/403（連 `serde` 也這樣）。只有送完整瀏覽器 header（`User-Agent: Mozilla/5.0` + `Accept: text/html`）才回 200。實際 HN 訪客點連結會從瀏覽器過，正常 render。**API** 側 `https://crates.io/api/v1/crates/<name>` 一律 200，證實 crate 真的存在且 metadata 正確。

### E. 舊 URL redirect（從 shellfans-dev → ShellFans-Kirin）

```
https://github.com/shellfans-dev/drift_ai
  → HTTP 200
  → final_url=https://github.com/ShellFans-Kirin/drift_ai
```

GitHub 自動 redirect 機制運作正常，舊連結不會給訪客 404。

---

## Launch Blocker 清單

**1 項必須處理才能發**：

### #1（🔴 RED）Secret handling 說明缺失

- **具體**：無 `SECURITY.md`；`drift capture` 會把 session content 原樣寫入 repo；無 scrub 邏輯；README 完全沒提
- **HN 預計留言**：「events.db 含我貼過的 API key / password 且預設 commit，這工具不能用」
- **最低修法**（1-2 小時）：
  1. 建 `docs/SECURITY.md`（~60 行；說明 threat model / 現況 / 可調旗標 / 未來 redact plan）
  2. README 加 `## Privacy & secrets` 章節（~10 行，見上方建議）
  3. CHANGELOG 加一行「Document secret handling limitations (v0.2 plans regex-based redaction pass)」
  4. （可選）在 `drift capture` 加一個 warning：首次跑時印「注意：session 會原樣 commit，檢查你的 .prompts/ 再 push」

---

## Nice-to-have（發前可改但不 block）

### #1（🟡）README 第一屏補一行 pain sentence

見 Audit 2 的建議。3 分鐘編輯。

### #2（🟡）補「vs Cursor / Cody / Copilot」對比段

Audit 3 的 Q1。放在 `docs/VISION.md`（建一節「Related work」）或 README Install 後面加「How drift compares」（~15 行表格）。

### #3（🟡）README 加一行「local LLM 也行」

Audit 3 的 Q5。在 `## Configuration` 章節裡補：

```
[compaction]
provider = "anthropic"    # default; switch to "mock" for offline
# v0.2 will add "ollama" / "vllm" / generic "openai-compatible"
```

### #4（🟡）明確聲明獨立身分

Audit 3 的 Q8。README 底部或 `docs/ABOUT.md`：

```
drift is an independent open-source project by Kirin (shellfans.dev).
Not affiliated with Anthropic, OpenAI, or any other vendor whose
agents it integrates with.
```

### #5（🟢）補 `CI / Docs / Downloads` badge（通常 HN 讀者不在乎，但完整性）

README 頂部（第 2 行 tagline 旁）加 3 個 shields.io badge：
- crates.io version
- License
- GitHub CI status

**不建議**：超過 5 個 badge 會壓縮痛點空間，反效果。

---

## 建議的 launch 時程

| 時間點 | 動作 |
|---|---|
| **T-90 min** | 寫 `docs/SECURITY.md` + README `## Privacy & secrets` 章節（Launch Blocker #1）|
| **T-60 min** | 順手做 Nice-to-have #1 / #3 / #4（pain sentence、local LLM 一行、獨立聲明），共約 30 分鐘 |
| **T-30 min** | 確認 `cargo install drift-ai` / `brew install ShellFans-Kirin/drift/drift` 還能裝（各跑一次，已在別的報告驗過） |
| **T-15 min** | PR → main → merge（或直接 push 到 main；squash-merge 慣例） |
| **T-0** | Show HN 發文 |
| **T+2h** | 回留言最密集的時段（Q3 secret / Q1 vs Cursor / Q5 local LLM / Q8 affiliation 是預期 Top 4） |

**如果 Launch Blocker #1 不修就發**：可以發，但 HN 前 10 則留言有 70% 機率聚焦在 secret risk，花一小時寫文件比花 3 小時 counter-argue 划算。

---

## 附錄：資訊缺口（審計環境限制）

- 本機是 Linux x86_64，只能模擬 Homebrew 行為，不能真跑 `brew install`。不過 `docs/BREW-VERIFY-MAC-V0.1.1.md` 已有 Apple M4 Mac 實跑證據，補足此缺口。
- crates.io 網頁 SPA 對 curl 不友善，只能用 API + 模擬瀏覽器 header 雙路驗證（結論一致：頁面存在）。
- 沒有檢查 CI 的 `ci.yml` 在最後一次 push 後是否還綠（假設沒壞；如要 belt-and-suspenders 可 Phase H 前再跑一次 `gh run list`）。
- 沒有跑 `cargo audit` 或 supply-chain scan（scope 外）。

---

*審計完成時間*：2026-04-24
*下一步*：使用者 review，決定是否接受建議的 Launch Blocker #1 + 1-2 個 nice-to-have，修完再發。
