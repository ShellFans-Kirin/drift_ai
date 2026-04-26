# Drift AI v0.2.0 開發與執行結果

**日期**：2026-04-25
**版本**：v0.2.0
**主題**：`drift handoff` — 跨 agent task transfer
**範圍**：完整開發週期紀錄（design → code → test → demo → release → publish → verify）

---

## 0. 開發策略

v0.2 以單一新指令為核心交付，凍結 v0.1 的所有契約：

| 凍結項 | 含義 |
|---|---|
| `events.db` schema | v0.1.x 升級 v0.2 是純 binary swap，不需要 migration |
| MCP tool list | 既有 5 個 read-only tool 不變，現有 client 繼續可用 |
| `SessionConnector` trait | 既有 connector 實作不需改 |

唯一允許的 code 變動：新增 `drift handoff` CLI 指令 + 必要的 library 程式碼。**不修 v0.1 bug 順手帶進 v0.2** — 那是 v0.1.3 的事。

---

## 1. Phase 0 · Design Proposal（人為 review gate）

### 交付物

`docs/V020-DESIGN.md`（522 行 / 9 章節）

### 章節結構

1. CLI surface（30 秒 demo 逐字稿驅動）
2. Handoff brief markdown shape（mock OAuth task fixture）
3. Implementation architecture（four collectors + LLM second pass + pure-Rust render）
4. Per-`--to` agent footer policy
5. Testing strategy（unit + golden-file + integration + 1 real smoke）
6. Demo asset planning（asciinema cast + agg → GIF, 30-sec hard cap）
7. v0.2 non-goals（紀律邊界）
8. Risks & open design questions for reviewer
9. What review would unblock

### Review 結果

只有一個改動：**default LLM model `claude-haiku-4-5` → `claude-opus-4-7`**。

理由（後寫進 design doc §3.3）：
- handoff brief 是 user-facing artifact 下個 agent 會逐字讀
- handoff 頻率低（一天少數次），cost 集中而非分散
- "What I'm working on" 3-5 句敘事是 HN demo 第一眼看到的東西，narrative quality 是價值
- cost-conscious 用戶可以 `[handoff].model = "claude-haiku-4-5"` 切回去（30× 便宜）

commit `7a94e18`：`docs(v0.2): switch default model to claude-opus-4-7`

---

## 2. Phase 1 · Implementation

### 2.1 新增程式碼（drift-core）

`crates/drift-core/src/handoff.rs`（1142 行）

公開 API：

```rust
pub fn build_handoff(
    store: &EventStore,
    provider: Option<&AnthropicProvider>,
    opts: &HandoffOptions,
) -> CompactionRes<HandoffBrief>;

pub fn render_brief(brief: &HandoffBrief, target: TargetAgent) -> String;
```

支援型別：

- `HandoffOptions` — repo / scope / target_agent
- `HandoffScope` — `Branch(String)` | `Since(DateTime<Utc>)` | `Session(String)`
- `TargetAgent` — `ClaudeCode` | `Codex` | `Generic`
- `HandoffBrief` — 中介結構（pre-render data + LLM-derived sections）
- `FileSnippet` / `Decision` / `ProgressItem` / `RejectedApproach`

四個 collector 每個 ~50–100 LOC：

1. `collect_sessions(store, scope, repo)` — 依 scope 過濾 sessions（Branch 用 git log 找 divergent commit timestamp 當 lower bound）
2. `collect_events(store, sessions)` — 分組 `CodeEvent` by `file_path`
3. `collect_rejected(store, sessions)` — pre-extract rejected events（v0.1 已有此資料）
4. `extract_file_snippets(repo, events_by_file)` — 讀 working tree，標 modified ranges，<50 行全文 / >50 行抽片段 ±5 行 context

### 2.2 LLM second pass

`crates/drift-core/templates/handoff.md`（59 行）

prompt 要求 LLM 輸出 JSON envelope：

```json
{
  "summary": "3-5 sentences",
  "progress": [{"status": "done|in_progress|not_started", "item": "..."}],
  "key_decisions": [{"text": "...", "citation": "..."}],
  "open_questions": ["..."],
  "next_steps": ["..."]
}
```

parser 容忍：
- `\`\`\`json` 包裝（model 偶爾忽略 instruction）
- 前後散文（"Sure, here is the JSON: { ... } hope that helps"）
- 非 JSON fallback：把 raw text 當 summary，其他 sections 留空

### 2.3 新增 AnthropicProvider 公開 API

`crates/drift-core/src/compaction.rs`（+236 行）

```rust
pub async fn complete_async(&self, system: &str, user: &str) -> CompactionRes<LlmCompletion>;
pub fn complete(&self, system: &str, user: &str) -> CompactionRes<LlmCompletion>;
```

重用 `compact_async` 的 retry / SSE / token-usage machinery，但接受 raw system + user prompt，回傳 `LlmCompletion`（text + tokens + cost）。`LlmCompletion` 跟 `CompactionUsage` 平行，差別是沒有 per-session_id binding（handoff aggregates across sessions）。

### 2.4 設定（config.rs）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffConfig {
    #[serde(default = "default_handoff_model")]
    pub model: String,  // default: "claude-opus-4-7"
}
```

放在 `[handoff]` section。

### 2.5 CLI command

`crates/drift-cli/src/commands/handoff.rs`（257 行）

flag parsing + scope resolution + provider construction（with `.with_progress(false)` to suppress per-token spinner that would overlap with handoff's own `⚡` progress lines）+ output 寫檔 / `--print` 走 stdout + per-target next-step hint。

`DRIFT_HANDOFF_QUIET=1` env var 抑制 `⚡` progress 行（給 piping 場景）。

### 2.6 Workspace bump

四個 `Cargo.toml`：`0.1.2 → 0.2.0`，內部 path-deps 同步 bump。

---

## 3. Phase 2 · Tests

### 3.1 Unit tests 數量增加

- v0.1.2: 45 tests
- v0.2.0: **67 tests**（+22 new）

新增的 22 個：
- handoff::tests（drift-core）：15 個 — TargetAgent::parse aliases / merge_overlapping / render_excerpt 各種 case / parse_llm_json with fence + prose + non-JSON / make_brief & 三個 render_brief target variants / truncate / deterministic_second_pass
- handoff::tests（drift-cli）：7 個 — resolve_scope branch / session / since / bad ISO / zero / multiple / slugify

### 3.2 自動驗證 gate

| 檢查 | 結果 |
|---|---|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅（0 warnings, 0 errors）|
| `cargo test --all` | ✅ 67 passed / 2 ignored / 0 failed（11 test suites） |
| `cargo build --all` | ✅ |

### 3.3 真實 Anthropic Opus smoke

```
$ drift handoff --session 3d132809-e586-435c-a4c2-4060b093dcc2 --to claude-code --print
```

- 來源：`/tmp/drift-smoke-v011/.prompts/events.db`（v0.1.1 開發期捕到的真實 session，6 turns）
- 模型：`claude-opus-4-7`
- 成本：~\$0.05 / 6 秒 wall time

**注目行為**：Opus 正確識別出這個 session 只是 `/login` + `/exit` 沒有實際 coding work，產出了「cold start」brief 要求接手 agent 先問用戶要做什麼，**沒有編造 progress**。完整輸出 → [`docs/V020-SMOKE-OUTPUT.md`](V020-SMOKE-OUTPUT.md)。

---

## 4. Phase 3 · Demo + README

### 4.1 Asciinema 錄製

```bash
asciinema rec --quiet --idle-time-limit=2.5 docs/demo/v020-handoff.cast \
  -c '/tmp/demo-script.sh'
```

- demo script 模擬「codex stalled → drift handoff → 顯示 brief → paste-to-claude prompt」流程
- 真實跑 `drift handoff` binary，真實 Anthropic Opus call
- 結果 cast 檔 4.4 KB / 74 events / ~30 秒回放

### 4.2 GIF rendering

`agg`（asciinema 官方 cast → GIF 工具）via `cargo install --git https://github.com/asciinema/agg`：

```bash
~/.cargo/bin/agg docs/demo/v020-handoff.cast docs/demo/v020-handoff.gif \
  --theme=monokai --speed=1.0 --font-size=14
```

- 結果 GIF：120 KB / 688×490 / 70 frames
- 可內嵌進 GitHub README，自動播放

### 4.3 README 改寫

| 區塊 | v0.1.2 | v0.2.0 |
|---|---|---|
| 第一行 tagline | "AI-native blame for the post-prompt era" | "Hand off your in-progress AI coding task between Claude, Codex, and whatever agent you switch to next" |
| 第一屏 hero | 文字 demo（drift log + drift blame）| **GIF**（drift handoff 真實錄製） |
| Pain copy | "Your commit says 'Add OAuth'..." | "Your AI coding agent stalled — refused, rate-limited, or just got dumb..." |
| Quickstart | 5 commands | 6 commands（含 `drift handoff`）|
| 新增章節 | — | `## Handoff — cross-agent task transfer (v0.2)`（在 `## Live mode` 之前）|
| Configuration `[handoff]` 區塊 | — | 加上 `model = "claude-opus-4-7"` 預設 + Haiku opt-out 註解 |
| About | 獨立聲明 | 加 dogfood-origin 一行：「Originally built for myself when I kept losing context between Codex stalls and Claude rate-limits」|

blame / log 內容**保留**，但從 hero 降到中段「supporting feature reference」位置。

### 4.4 CHANGELOG

`v0.2.0` 章節（74 行）：8 個 Added / 4 個 Changed / Stability guarantees re-affirming v0.1.x freezes / Known limitations。

---

## 5. Phase 4 · Release

### 5.1 git flow

```
v0.2.0 (6 commits) → squash-merge → main → tag v0.2.0 → push tag
```

main 上的 squash commit：`e6d6371 feat(v0.2.0): drift handoff — cross-agent task transfer`（18 files / +2621 / -52）。

### 5.2 release.yml

| Job | 結果 | 備註 |
|---|---|---|
| Build x86_64-unknown-linux-gnu | ✅ | ubuntu-latest native |
| Build aarch64-unknown-linux-gnu | ✅ | ubuntu-latest + cross |
| Build x86_64-apple-darwin | ✅ | macos-14 + `rustup target add` |
| Build aarch64-apple-darwin | ✅ | macos-14 native |
| release（download artefacts + create release + dispatch tap）| ✅ | run `24925365141` |

8 個 release assets（4 tarball + 4 sha256 sidecar）全部上線。

### 5.3 Homebrew tap 自動更新

`release.yml` → `peter-evans/repository-dispatch@v3` → `homebrew-drift/.github/workflows/update-formula.yml`。

| 檢查 | 結果 |
|---|---|
| Tap dispatch run | ✅ 2026-04-25T07:11:02Z（release.yml 完成 ~3 分鐘後 fire） |
| Formula `version "0.2.0"` | ✅ |
| 4 個 sha256 自動填入 | ✅（`88294ce8` arm-mac / `b526d605` x86-mac / `b4380029` arm-linux / `39ba98cc` x86-linux）|

### 5.4 crates.io 連續 publish

依依賴順序，每次間隔 45 秒等 index 同步：

```
cargo publish -p drift-core            # → 0.2.0
sleep 45
cargo publish -p drift-connectors      # → 0.2.0
sleep 45
cargo publish -p drift-mcp             # → 0.2.0
sleep 45
cargo publish -p drift-ai              # → 0.2.0
```

驗證：

```
drift-core         max_version=0.2.0  ✓
drift-connectors   max_version=0.2.0  ✓
drift-mcp          max_version=0.2.0  ✓
drift-ai           max_version=0.2.0  ✓
```

### 5.5 cargo install 從乾淨 /tmp

```
$ cargo install drift-ai --locked
   Compiling drift-connectors v0.2.0
   Compiling drift-ai v0.2.0
    Finished `release` profile [optimized] target(s) in 1m 56s
  Installing /tmp/drift-clean-cargo-v020/bin/drift
   Installed package `drift-ai v0.2.0` (executable `drift`)

$ drift --version
drift 0.2.0
```

### 5.6 Mac brew install 重驗

- 機器：Apple M4 / macOS 26.3.1 / Homebrew 5.1.7
- 連線方式：SSH 到自己的 Mac
- 驗證 11 步全綠：tap → install (7.6 s) → `drift --version = 0.2.0` → handoff --help 顯示完整 flag → mcp serverInfo.version=0.2.0 → brew test drift → uninstall → untap → 殘留掃描乾淨

---

## 6. Phase 5 · Show HN drafts（committed only, not posted）

| 文件 | 內容 |
|---|---|
| `docs/launch/v020-show-hn.md` | Title（primary + backup）、post body（dogfood-origin 開頭 + GIF + 安裝命令）、cross-link list、8 個高頻 Q&A pre-baked 答案 |
| `docs/launch/v020-twitter-thread.md` | 7-tweet thread：hook GIF / what-it-produces / why-hard / pieces / install / roadmap / independence |
| `docs/launch/v020-pre-launch-checklist.md` | T-30 install rebuild × 3 / T-25 asciinema upload / T-20 README sanity / T-15 doc tabs / T-10 post review / T-5 timing / T-0 submit / T+2 hr monitor / T+24 hr post-mortem，加 hold conditions 紅燈條件 |

發布時機交給作者決定。建議：週二/三/四 台北晚 8-10 PM = 美西早班。

---

## 7. 數字總結

### v0.2.0 dev cycle

| 指標 | 值 |
|---|---|
| Phase 數 | 5 |
| 新增程式碼 LOC | ~1900（含測試）|
| 新增 unit + integration test | 22（45 → 67）|
| 新增 doc | 7 個 .md（design / smoke / dev-log / usage / show-hn / twitter / checklist）|
| Anthropic Opus smoke calls | 2（Phase 2 + Phase 3 demo）|
| 真實 cost | ~\$0.10 |
| Demo asset | 1 cast (4.4 KB) + 1 GIF (120 KB)|
| Git commits on main | 1（squash 合併 6 個 v0.2.0 branch commits）|
| Time to ship（Phase 0 design 完成 → main tagged → all packages live）| 一個 dev session（< 1 工作日）|

### 累計（v0.1.0 → v0.2.0 全部 Anthropic 用量）

| Stage | Calls | Cost |
|---|---|---|
| v0.1.0（design + Mock smoke）| 0 | \$0 |
| v0.1.1（Phase A8 smoke + 2 demo runs）| 23 | ~\$3.07 |
| v0.1.2（dev smoke）| 13 | \$0.20 |
| v0.2.0（dev smoke + demo）| 2 | \$0.10 |
| **Total** | **38** | **~\$3.37** |

---

## 8. 上線後的下一步

1. **Show HN posting** — 等作者拍時機。Pre-launch checklist 第 30 分鐘起算。
2. **awesome-claude-plugins PR** — 既有 PR 草稿要 bump 到 v0.2.0 + 加 handoff feature 行
3. **v0.3 候選 feature**（依社群 feedback 排序）：
   - Ollama / vLLM 跨 provider 支援（讓 handoff 100% 離線）
   - Cursor / Cline connector PR welcome（既有 SessionConnector trait）
   - Cross-agent prompt schema translation（tool-call adapters）
   - Team handoff（colleague-to-colleague brief，含 sanitisation pass）
   - Regex-based secret redaction in `drift capture`（v0.1 audit blocker 的 v0.2 roadmap 項目）
   - `drift handoff list` / `drift handoff show <id>` query

---

## 9. 一行結論

🟢 v0.2.0 完整交付：design 經 review、code 經測試、demo 真錄、release 上線、Homebrew + crates.io 雙 channel 立即可裝、Mac 端 brew 驗過、Show HN 文案 ready。沒有 launch blocker，發布時機由作者決定。
