> 🌐 [English](CHANGELOG.md) · [日本語](CHANGELOG.ja.md) · [简体中文](CHANGELOG.zh-Hans.md) · **繁體中文**

# Changelog

drift_ai 的所有重大改動都記在這。
格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)；
版本號遵循 [SemVer](https://semver.org/spec/v2.0.0.html)。

## [0.2.0] — 2026-04-25

「你不會被綁死在單一 LLM vendor」的 release。新增 **`drift handoff`** —
新的招牌指令 — 跟一份 v0.2 風格的 README，把 task transfer 推到最前面、把
blame 降為 supporting feature。

### Added

- **`drift handoff` CLI**。把進行中的 task（用 `--branch`、`--since`、或
  `--session` 過濾）打包成一份下一個 agent 能冷讀的 markdown brief。Flag：
  `--to claude-code|codex|generic`、`--output <path>`、`--print`。預設輸出：
  `.prompts/handoffs/<YYYY-MM-DD-HHMM>-<branch>-to-<agent>.md`。
- **`crates/drift-core/src/handoff.rs`** — orchestrator + 4 個小 collector
  （sessions、events-by-file、rejected approaches、file snippets）+ LLM 二段
  pass + 純 Rust 的 `render_brief`。新單元測試覆蓋 scope parsing、snippet
  extraction（full vs. modified-range 摘錄）、JSON-from-LLM parsing（容忍
  code-fence + 周邊敘述）、以及 per-`--to` footer 渲染。新增 15 個 test。
- **`crates/drift-core/templates/handoff.md`** — 二段 pass 的 LLM prompt
  template；要求 model 輸出含 `summary` / `progress` / `key_decisions` /
  `open_questions` / `next_steps` 的 JSON。
- **`AnthropicProvider::complete_async`** + 同步 `complete` — 通用的
  system+user → text 補全，重用 `compact_async` 的 retry / streaming /
  token-usage 機制給需要不同 prompt 形狀的 caller 用（handoff 用）。回傳
  新的 `LlmCompletion` struct（text + per-call token / cost）。
- **`[handoff]` config section** 在 `.prompts/config.toml` 裡。預設 model
  `claude-opus-4-7`。預設選 Opus — handoff brief 是 user-facing 產物，下一個
  agent 會逐字讀，敘事品質就是 value，handoff 頻率本來就低（一個工作日通常
  一兩次）。要 ~30× 成本下降可以切 Haiku。
- **30 秒 demo** 在 `docs/demo/v020-handoff.gif`（用 fixture data 對著
  `drift handoff` 真實錄製；`docs/demo/v020-handoff.cast` 是原 cast 檔）。
- **真實 Anthropic smoke 輸出**收在
  [`docs/V020-SMOKE-OUTPUT.md`](docs/V020-SMOKE-OUTPUT.md)。
- **`docs/V020-DESIGN.md`** — Phase 0 設計提案，留 repo 裡作為 `drift handoff`
  形狀的參考。

### Changed

- README 第一屏改 `drift handoff` 為招牌，hero 位放 demo GIF。blame / log
  保留為「supporting feature」放在同一屏作為 reference。
- Quickstart 從 5 個指令增至 6 個（加了 `drift handoff`）。
- About section 加一句 dogfood 出身的小註腳。
- 預編 binary 安裝 URL 升到 `drift-v0.2.0`。

### v0.1 帶過來的穩定性保證

- `events.db` schema **不變**。從 v0.1.x 升上來純粹是 binary 替換；不需要
  migration。
- MCP tool 列表**不變**。既有的 MCP client 照常運作。
- `SessionConnector` trait **不變**。既有 connector 照常運作。
- v0.1.2 的 first-run privacy notice 仍在第一次跑 `drift capture` 時觸發；
  handoff 不需要重新 acknowledge。

### 已知限制（v0.2）

- `--branch <name>` scope 是 best-effort：它跑 `git log <branch> --not main
  --format=%aI` 找最早分歧的 commit 當 lower-bound filter。同一天落在多個並
  行 branch 的 session 可能會交疊 — 用 `--since` 收斂。
- handoff 的 LLM call 跟任何 Opus call 同樣 cost profile（每份 brief ~$0.10）。
  重度使用的話設 `[handoff].model = "claude-haiku-4-5"`。
- 還沒有 `drift handoff list` / `drift handoff show <id>` — 產生的 brief 就是
  `.prompts/handoffs/` 下的 markdown 檔。`ls` 跟 `cat` 是 v0.2 的查詢介面。

## [0.1.2] — 2026-04-25

蓋在 v0.1.1 上的文件 + 訊息 patch。compaction / attribution / MCP 的 code
path 跟 v0.1.1 完全相同；唯一行為變動是使用者第一次跑 `drift capture` 時的
一次性 privacy notice。

### Added
- **`docs/SECURITY.md`** — threat model、目前限制、可用 mitigation
  （db_in_git toggle、手動 review、gitleaks/trufflehog pre-commit）、
  v0.2 roadmap（regex redaction pass、互動式 review mode、`drift redact`
  事後 scrub）、安全揭露管道。
- **README `## Privacy & secrets` section** — 直接、不軟銷、明確揭露
  `drift capture` 會把 session content 鏡像進 `.prompts/`，且預設把
  `events.db` commit 進 git。
- **`drift capture` 第一次的 notice** — 第一次調用會印一段 privacy 立場
  提醒並等 stdin。`DRIFT_SKIP_FIRST_RUN=1` 跳過（CI-friendly）。狀態記在
  `~/.config/drift/state.toml::first_capture_shown`。
- **`docs/COMPARISON.md`** — 對 Cursor / Copilot chat / Cody / `git blame`
  的功能比較。從 README 連過來。
- **README 痛點開場** — 一段（"47 prompts to Claude + 3 Codex fills + 12
  manual edits ..."）放在技術描述上方。
- **README `## About` section** — 明確聲明 drift 是獨立專案，不隸屬於
  Anthropic、OpenAI 或任何 agent vendor。
- **README badges**：crates.io 版本 + CI 狀態（限兩個）。
- **Provider-switching 範例** 在 `## Configuration` 提到 v0.2 計畫
  （ollama / vllm / openai-compatible）。

### Tests
- `tests/first_run_notice.rs` 涵蓋 `DRIFT_SKIP_FIRST_RUN=1` 的 bypass 跟
  state-file persistence 路徑。

### v0.1.1 帶過來的已知限制
- Drift 仍然不主動 redact secret — 那是 v0.2 的事。
- 計價表是 hardcoded；當作正式 invoice 之前請核對 Anthropic 的 public
  pricing。

## [0.1.1] — 2026-04-23

### Added
- **Live Anthropic compaction.** `AnthropicProvider` 現在真的會打
  `POST /v1/messages?stream=true`，消費 SSE stream，CLI 跑時把 content
  delta echo 到 stderr，並在 `message_stop` 解析 `usage` block 做 billing。
- **Typed compaction error**（`CompactionError`）：`AuthInvalid`、
  `RateLimited { retry_after }`、`ModelNotFound`、`ContextTooLong`、
  `TransientNetwork`、`Stream`、`Other`。每個 variant 對應一個獨立的、
  operator 看得到的 CLI 訊息。
- **Model 切換靠 config**：`[compaction].model` 接受 `claude-opus-4-7`
  （預設）、`claude-sonnet-4-6`、`claude-haiku-4-5`。
- **Retry policy**：429 跑 5 次並遵守 `Retry-After`；5xx 跑 4 次配指數
  backoff（1s → 2s → 4s → 8s）；401/404 直接失敗。
- **Context-window 截斷**：char-based token 估計 + 80% 門檻；Strategy 1
  保留 head(8) + tail(8) turn，中間用明確 marker 省略。
- **`compaction_calls` table**（SQLite migration v2）：per-call 的
  input / output / cache-creation / cache-read token 數量加上算好的 USD
  cost。
- **`drift cost`** CLI：`--since <iso>` / `--until <iso>` /
  `--model <name>` / `--by model|session|date`。
- **`drift watch` 是 event-driven**：用 `notify`
  （FSEvents/inotify/ReadDirectoryChangesW）支撐，200ms debounce、依檔名
  推導出 session_id 來 per-session capture，狀態存 `~/.config/drift/watch-state.toml`，
  SIGINT/SIGTERM 收尾完當前 capture 才退出。
- **Homebrew tap 上線**：`brew install ShellFans-Kirin/drift/drift` 對著
  公開的 [homebrew-drift](https://github.com/ShellFans-Kirin/homebrew-drift)
  tap；formula 每次 release 都透過 `release.yml` 的 `repository_dispatch`
  自動 regenerate。
- **發布到 crates.io**：`drift-core`、`drift-connectors`、`drift-mcp`、
  `drift-ai`。

### Changed
- `CompactionProvider::compact` 現在回傳 `CompactionResult`
  （summary + 選用 usage）而不是只有 `CompactedSummary`，讓 live provider
  能把 billing data round-trip 回來。
- `drift init` 是 idempotent：再跑不會覆蓋已存在的 `config.toml`。
- `drift capture` 對單一 session 的 compaction error 採 soft-fail（log + 跳過），
  一個 oversized session 不會中斷整批。
- `summary_to_markdown` 現在會輸出真正的 section heading（`## Summary`、
  `## Key decisions`、`## Files touched`、`## Rejected approaches`、
  `## Open threads`），取代原本一行的 `[MOCK]` blurb。

### Fixed
- Workspace 內部依賴釘在 0.1.1（之前是 0.1.0），讓 `cargo publish` 能對
  crates.io 解析。
- 不小心 check-in 進來的 ship-time smoke `.prompts/events.db` 現在會被
  ignore；`.prompts/` 加進 `.gitignore` 給乾淨 clone 用。

### Known limitations
- Context-window Strategy 2（階層式 summarization）骨架完成但 feature flag
  關閉。預設行為是 Strategy 1。
- Cost 總額用 hardcoded 計價表（對著 Anthropic 截至 2026-04-23 的 public
  pricing 對過）；當 billing report 用之前再對著
  <https://www.anthropic.com/pricing> 核一次。

## [0.1.0] — 2026-04-22

### Added
- Cargo workspace 含四個 crate：`drift-core`、`drift-connectors`、
  `drift-cli`（binary：`drift`）、`drift-mcp`。
- Claude Code + Codex 的 first-class connector；Aider stub 在 feature flag
  後面（`aider`）。
- Attribution engine：`CodeEvent` row 落在 `.prompts/events.db`（SQLite），
  人類編輯偵測用 SHA-256 ladder，rename 兩層處理（session tool call +
  git-log-follow fallback），MultiEdit intra-call parent chain。
- Compaction engine 含 `MockProvider`（v0.1.0 預設，標 `[MOCK]`）跟一個
  `AnthropicProvider` skeleton（HTTP 呼叫在 v0.1.1 接通）。
- CLI：`init`、`capture`、`watch`、`list`、`show`、`blame`、`trace`、
  `diff`、`rejected`、`log`、`bind`、`auto-bind`、`install-hook`、
  `sync push/pull`、`config get/set/list`、`mcp`。
- Git notes 整合（`refs/notes/drift`）：手動 binding、依 timestamp 自動
  binding、non-blocking 的 post-commit hook。
- Stdio MCP server 含 5 個唯讀 tool：`drift_blame`、`drift_trace`、
  `drift_rejected`、`drift_log`、`drift_show_event`。
- Plugin skeleton（`plugins/claude-code/`、`plugins/codex/`）— v0.1.0 沒
  publish；v0.2 才上 marketplace。
- CI（`.github/workflows/ci.yml`）跟 release（`release.yml`）矩陣覆蓋
  Linux x86_64/aarch64 + macOS x86_64/aarch64。
- Apache-2.0 授權，CONTRIBUTING 走過新增 connector 流程，code-of-conduct。

### 已知限制
- Anthropic compaction HTTP call 還是 stub。Mock 是 shipping 預設；接通的
  說明留在 `crates/drift-core/src/compaction.rs`。
- 人類編輯偵測只到 timeline — 不主張作者身分。
- Codex 的 `reasoning` item 是加密的；只計數，不 surface。
- `drift watch` 是 debounced polling daemon；v0.2 改成完全 event-driven。
- `cargo publish` 這次 cut 沒跑；`0.1.1` 的 Cargo.toml metadata 都備齊了。
