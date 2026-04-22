# Drift AI — 專案終極目標

**狀態**：願景文件（非契約）。作者基於目前規格與 Phase 0 提案的理解撰寫。
**對應 v0.1.0 交付**：見 [PHASE0-PROPOSAL.md](PHASE0-PROPOSAL.md)。

---

## 一句話

Drift AI 要成為 **AI 時代的 `git blame`** — 當 code 的原始作者不再是單一個人、
而是「一個 prompt 觸發一段 AI 產出、被人類部分改寫、又被另一個 agent 重構」
的混血 timeline 時，Drift AI 是唯一能把這段 timeline 還原到「每一行」粒度
的本地工具。

---

## 為什麼需要它

### 核心 thesis：commit 顆粒度太粗

今天的 source control 假設「一個 commit = 一次有意義的變動、作者單一」。
但在 AI coding 的工作流裡，這個前提破了：

- 一次 commit 常常包含 **多個 AI session** 的產出（Claude Code 寫了骨架、
  Codex 補了邊界條件、Aider 修了 lint）。
- AI 提的建議裡，**被採納的只是一部分** — 剩下被拒絕、被改寫、被替換的
  思路，同樣是重要的設計證據，但 commit 完全不會記錄。
- 使用者 **手改了 AI 寫的 code** 之後，git 只看得到「最後的文字」，看不出
  「人類在 AI 的基礎上做了什麼 judgement」。
- **Rename / refactor** 把血統切斷，`git blame` 跟不到檔名變更前的來源。

這些資訊**都在某處存在** — Claude Code 的 `~/.claude/projects/*.jsonl`、
Codex 的 `~/.codex/sessions/**/*.jsonl`、以及使用者硬碟上檔案的歷次 SHA
狀態。Drift AI 做的事情就是把這些 **散落、短命、彼此不相通** 的訊號，編織
成一份 **單一、可查詢、跟著 commit 走** 的 timeline。

### 為什麼是「本地優先 (local-first)」

- **隱私**：AI session 的完整對話含商業秘密、auth token、使用者意圖。上雲
  不是 drift_ai 該做的決定。
- **可離線可攜**：整個系統只依賴 git + SQLite + 本地 session 檔。clone 一個
  repo，blame 層就跟著走。
- **沒有廠商鎖定**：Drift AI 不擁有你的 prompt；它只是把散落的紀錄整理成
  `.prompts/` 目錄與 `refs/notes/drift`，任何時候可以停用、移除、甚至換掉
  整個工具 — 資料仍是你的。

---

## 使用者真正會用到的三個場景

### 場景 1：事後除錯（反查，`drift blame`）

> 「這段 rate limiter 為什麼這樣寫？我不記得自己寫過。」

```
$ drift blame src/auth/login.ts --line 42
src/auth/login.ts:42
├─ 2026-04-15 14:03  [claude-code]  session abc123  prompt: "add rate limiting"
│  diff: +  if (attempts > 5) throw new RateLimitError()
├─ 2026-04-15 15:20  [human]        post-commit manual edit
│  diff: -  if (attempts > 5)
│         +  if (attempts > MAX_ATTEMPTS)
└─ 2026-04-16 09:12  [codex]        session def456  prompt: "extract magic numbers"
   diff: +  const MAX_ATTEMPTS = 5
```

對一個新進 team member、或三個月後重開老 repo 的自己，這份 timeline 值回
整個工具的價值。

### 場景 2：回顧設計決策（正查，`drift trace`）

> 「上週那個 OAuth 大改，我到底跟 Claude 討論了什麼？最後為什麼不用 manual JWT？」

```
$ drift trace abc123
Session abc123 (claude-code, 2026-04-15 14:00–14:47, 7 turns)
  files_touched:     src/auth/{login,session,callback}.ts
  key_decisions:     NextAuth over manual JWT (turn 4)
  rejected:          manual JWT approach (turn 3) — "too much token refresh boilerplate"
  code_events:       17 total (14 accepted, 3 rejected)
```

commit message 寫了「Add OAuth」，但是「為什麼 NextAuth 不是 manual JWT」
這件事，只有 Drift AI 記得。

### 場景 3：審計與合規（`drift log`）

> 「這次 release 裡，有哪些 code 是 AI 生成、哪些是人類寫的？」

```
$ drift log v0.3.0..v0.4.0
commit 7f8a12b — feat: add webhook retries
  💭 [codex]      4 turns, exponential backoff design
  ✋ [human]      1 manual edit  (src/webhooks/retry.ts L88)

commit 2b4c5d9 — fix: race in session cache
  💭 [claude-code] 2 turns, spotted the TOCTOU
  💭 [claude-code] 1 turn,   applied the fix
```

對需要追蹤「AI contribution 比例」的團隊（合規、education、研究），這是
**唯一不靠 token 計數猜測、而是真正綁到 commit** 的紀錄。

---

## 願景的技術骨幹

三個分層，每層都是可替換的抽象：

### 1. Connector 層（attribution 的原料）

- **Day-one**：Claude Code + Codex 雙 first-class，是為了從第一天就壓測
  「跨 agent」的抽象 — 如果只支援一個 agent，抽象很容易變 wishful thinking。
- **Aider stub**：範例，CONTRIBUTING 裡寫「新增 connector」的 walkthrough
  都用 aider 示範。
- 任何未來的 AI CLI（Cursor CLI、Cline、一個自建的 agent...）都可以透過
  實作 `SessionConnector` trait 接進來。

### 2. Compaction 層（可讀性）

- 原始 session JSONL 動輒 500K 一份，人類無法讀。LLM compaction 把它壓到
  ~1K 的 markdown（frontmatter + 決策摘要 + 被拒絕的思路）。
- **Provider-agnostic**：Anthropic 是預設，`MockProvider` 跑測試；未來可接
  OpenAI、本地 Ollama、任何符合 `CompactionProvider` 介面的實作。
- Compacted 結果 commit 進 `.prompts/sessions/` — **隨 repo 移動，永遠可讀**，
  不依賴 Drift AI 仍然存在。

### 3. Attribution 層（核心差異化）

- **`CodeEvent`**：每次檔案變動一筆，帶 `diff_hunks`、`parent_event_id`、
  `content_sha256_after`、`rejected`、`rename_from`。
- **SHA-256 ladder**：監測「AI 寫完之後，人類把 SHA 改了」的訊號 — 這是
  Drift AI 唯一能誠實偵測人類手改的方法，不假裝做 authorship judgement。
- **Rename 兩層策略**：session tool call lexer → `git log --follow` fallback。
- **Git notes binding**：`refs/notes/drift` 把 compaction 結果綁到 commit；
  `events.db` 裝 CodeEvent，config 決定進不進 git。

---

## 範圍邊界 — Drift AI **不做**的事

刻意不做、不是忘了做：

| 不做 | 理由 |
|---|---|
| 雲端 SaaS 儀表板 | 違反 local-first。未來若需要團隊介面，應是第三方 web UI 讀取 `.prompts/` + notes，而非 Drift AI 自己長出雲端服務。 |
| 判定「某行的作者是誰」 | SHA 只能告訴我們「誰沒做這個變動」（AI session 之外的人做的），不能告訴我們「實際敲鍵盤的是誰」。Drift AI 的 `human` slug 只表語意「非 AI session 產生」，不主張作者身分。 |
| 量化「AI 貢獻比例」 | 行數比例會被簡單操縱（format、rename 都會放大）。我們提供原始 event timeline；任何衡量都交給上層工具依自己的定義算。 |
| 在 AI session 裡當 middleware 擋/改 prompt | Drift AI 是 **被動觀察者**，不攔截、不修改、不代理。所有上游 agent 照原樣運作；我們只讀完成後的紀錄。 |
| 取代 git 或 git-blame | 我們 **延伸** git，靠 git notes 疊加一層，不取代底層。卸掉 Drift AI，git 操作一如往常。 |

---

## 成功的長期樣貌

1. **生態相容**：Claude Code / Codex / Aider / Cursor CLI / 未來某個新 agent
   上線那一週，社群就有對應 connector PR 進來。因為 Connector trait 乾淨、
   CONTRIBUTING 有範例。
2. **Team-blame**：多人共同開發的 repo，每個人的 AI session 都各自被 Drift AI
   捕捉，`drift sync push/pull` 讓 notes 跨機器流通，**任何人的 `drift blame`
   都看到完整團隊 timeline**（含隱私模式：`db_in_git = false` 時只存本機）。
3. **成為新進工程師的第一個 command**：不是 `git log`，而是
   `drift blame <某個讓人困惑的 function>` — 因為它直接給出「這段為什麼
   這樣寫」的完整對話脈絡。
4. **合規與研究的標準格式**：AI-generated code 的審計、教學研究、contribution
   量測都用 `.prompts/` + `refs/notes/drift` 作為 input schema。因為這是第一個
   真正把 AI session 綁到 commit 的開源格式。

---

## 與 v0.1.0 的關係

這份文件描述的是 **北極星**。v0.1.0 要證明的是這條路線 **技術上走得通**：

- ✅ 雙 agent 跨越抽象（不是 single-agent hack）
- ✅ Line-level（不是 commit-level 敷衍）
- ✅ 資料模型撐得住 4 個非 trivial 場景（multi-origin、human edit、rejected、
  rename）— 這是 Phase 0 的 [自我評估](PHASE0-PROPOSAL.md#f-self-evaluation-does-the-data-model-honor-the-four-requirements)
- ✅ `drift blame` 的 demo 真的能跑

v0.1.0 不會有：team-sync 的細緻 UI、Cursor connector、web 介面、貢獻量測。
那些是 v0.2+ 的事，確定骨幹站得住才加。

---

## 為什麼現在做這件事

- AI coding tool 生態 2025-2026 年快速收斂為「Claude Code / Codex 雙強 + 長尾」，
  schema 相對穩定，適合下第一份跨 agent 抽象。
- `git notes` 作為底層機制在 git 2.40+ 之後 merge-friendly、push/pull 成熟。
- SQLite + 單 binary 分發讓 local-first daemon 的工程成本歷史新低。
- 還沒有任何開源工具認真做「AI-native blame」 — Mem0 / Supermemory 都在做
  memory 層，不是 attribution 層；那是完全不同的問題。

時機、技術、需求三者都對上了。v0.1.0 要在這個視窗把骨幹立起來。
