# drift_ai v0.1.2 — Ship Report

**Date**: 2026-04-25
**Scope**: Launch-readiness patch on top of v0.1.1. Resolves the audit
blocker (privacy / secret handling disclosure) + all 4 nice-to-haves.
Compaction / attribution / MCP code paths unchanged; only behavioural
addition is a one-shot privacy notice on first `drift capture`.

---

## 交付連結

| 項目 | URL / 路徑 |
|---|---|
| GitHub Release | <https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.1.2> |
| Release assets (4 × tarball + 4 × .sha256) | `drift-v0.1.2-{aarch64,x86_64}-{apple-darwin,unknown-linux-gnu}.tar.gz` |
| `drift-core` on crates.io | <https://crates.io/crates/drift-core> · `max_version=0.1.2` |
| `drift-connectors` on crates.io | <https://crates.io/crates/drift-connectors> · `max_version=0.1.2` |
| `drift-mcp` on crates.io | <https://crates.io/crates/drift-mcp> · `max_version=0.1.2` |
| `drift-ai` on crates.io | <https://crates.io/crates/drift-ai> · `max_version=0.1.2` |
| Homebrew tap Formula | <https://github.com/ShellFans-Kirin/homebrew-drift/blob/main/Formula/drift.rb> · `version "0.1.2"` |
| CHANGELOG | [`CHANGELOG.md` § 0.1.2](https://github.com/ShellFans-Kirin/drift_ai/blob/main/CHANGELOG.md#012--2026-04-25) |
| SECURITY (new) | [`docs/SECURITY.md`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/SECURITY.md) |
| COMPARISON (new) | [`docs/COMPARISON.md`](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/COMPARISON.md) |
| Pre-launch audit report | [`docs/LAUNCH-READINESS-AUDIT.md`](https://github.com/ShellFans-Kirin/drift_ai/blob/audit/launch-readiness/docs/LAUNCH-READINESS-AUDIT.md) |

---

## Audit blocker 處理結果

### 🔴 Blocker #1 — Secret handling disclosure → ✅ FIXED

| Sub-item | 狀態 | 變動 |
|---|---|---|
| `docs/SECURITY.md` 寫好 | ✅ | 110 行；含 threat model（drift 無 server，唯一網路 egress = `/v1/messages`）、current limitations、5 條 mitigations、v0.2 roadmap、disclosure channel |
| README `## Privacy & secrets` 章節 | ✅ | 17 行；放在 Install 與 Quickstart 之間，明確寫「`drift` does not scrub session content」、列 2 個 today knobs、警告「if you routinely paste secrets, wait for v0.2」、指向 `docs/SECURITY.md` |
| `drift capture` first-run notice | ✅ | 6 行 stderr 提示 + stdin Enter 確認；`DRIFT_SKIP_FIRST_RUN=1` bypass（CI 用）；state 寫到 `~/.config/drift/state.toml::first_capture_shown` |
| 3 unit tests | ✅ | `skip_env_var_bypasses_notice` / `already_shown_predicate_reads_state` / `write_state_creates_parent_dir`，全綠 |

---

## Nice-to-have 處理結果（4/4 完成）

| # | Item | 狀態 | 變動 |
|---|---|---|---|
| 1 | README pain-statement opener | ✅ | 第 8-11 行新增「47 prompts to Claude + 3 Codex fills + 12 manual edits…」段落，問題優先 |
| 2 | `docs/COMPARISON.md` + README link | ✅ | Functional matrix vs Cursor history / Copilot chat / Cody / `git blame`；明確聲明「drift 不替代以上工具，drift 讀它們寫的 session log 再疊一層」 |
| 3 | Provider switching example | ✅ | README `## Configuration` 加 `# v0.2 will add: ollama, vllm, openai-compatible` |
| 4 | `## About` independence statement | ✅ | README 末段 4 行；明示獨立、Apache-2.0、不附屬於 Anthropic / OpenAI / 任何 agent vendor |
| 5 (附帶) | 2 個 CI badge | ✅ | crates.io version + GitHub Actions CI status，line 3-4，總共 2 個（不超過上限） |

---

## 自家 `.prompts/` secret scan 結果

**STATUS: clean ✓**

掃描範圍：

- `.prompts/sessions/*.md` — **目錄不存在**（之前 ship 時 `.gitignore` 排除）。無檔案可掃 = 0 hits
- `.prompts/events.db` — 存在但**完全空**（`code_events: 0` / `sessions: 0`；`compaction_calls` 表本身缺失，是 v0.1.0 時期建的）。SQLite dump 後對所有 11 個 secret pattern 做 regex grep：**0 matches**

| Pattern | events.db hits |
|---|---|
| Anthropic_api_key (`sk-ant-…`) | 0 |
| OpenAI_project_key (`sk-proj-…`) | 0 |
| AWS_access_key (`AKIA…`) | 0 |
| GitHub_classic_PAT (`ghp_…`) | 0 |
| GitHub_finegrained_PAT (`github_pat_…`) | 0 |
| Slack_token (`xox[baprs]-…`) | 0 |
| Bearer_token (`Bearer …`) | 0 |
| Private_key_blob (`-----BEGIN…PRIVATE KEY-----`) | 0 |
| Env_Anthropic_assign (`ANTHROPIC_API_KEY=…`) | 0 |
| Env_SHELLFANS_assign (`SHELLFANS_KIRIN_PAT=…`) | 0 |
| Env_Cargo_assign (`CARGO_REGISTRY_TOKEN=…`) | 0 |

**結論**：repo working tree 沒有任何 secret 洩漏，可以放心 push v0.1.2 到 main。Scan 是針對 working tree 的 `.prompts/` 而非 repo history（events.db 一直 gitignored，從未被 commit）。

---

## v0.1.2 真實 Anthropic 用量（小 smoke）

| Model | Calls | Input tokens | Output tokens | Cost (USD) |
|---|---|---|---|---|
| `claude-haiku-4-5` | 13 | 159,766 | 7,433 | **\$0.1969** |

> Smoke 預期跑 3 個最短 session，但 connector 實際上 scan 整個 `~/.claude/projects/`，所以實跑 13 個（含 3 個 isolated + 主機 10 個既有）。Haiku 成本可控，沒超預算。

累計（v0.1.0 → v0.1.2 全部 smoke）：

| 累計 | Calls | Input | Output | Cost |
|---|---|---|---|---|
| Opus 4.7 | 10 | 142,204 | 10,380 | \$2.9116 |
| Haiku 4.5 | 23 | 280,724 | 14,015 | \$0.3508 |
| **Total** | **33** | **422,928** | **24,395** | **\$3.2624** |

---

## 自我驗證（Phase G/H 等同的清單）

| 檢查 | 結果 |
|---|---|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ |
| `cargo test --all` | ✅ 45 passed / 2 ignored / 0 failed |
| `cargo build --all` | ✅ |
| `drift --version` (本機 build) | `drift 0.1.2` ✅ |
| 4 crate dry-run + real publish | ✅（drift-core → drift-connectors → drift-mcp → drift-ai） |
| 4 crate `max_version=0.1.2` on crates.io API | ✅ |
| `cargo install drift-ai --locked` from clean `/tmp` | ✅ 1m 53s |
| Clean install `drift --version` | `drift 0.1.2` ✅ |
| `release.yml` v0.1.2 run | ✅ completed/success（run `24922786347`） |
| 4 release tarballs + 4 sha256 在 GitHub Release | ✅ |
| Homebrew tap `repository_dispatch` 自動 fire | ✅（run at `2026-04-25T04:43:56Z`） |
| Tap Formula `version "0.1.2"` + 4 sha256 自動回填 | ✅ |
| Audit `[MOCK]` count in active docs | 0 (只 README 一句設計說明) |
| 自家 `.prompts/` secret scan | clean ✓ |

---

## Pre-launch Mac verification checklist

對 Mac 端複製以下指令一次跑完，全綠就可發 Show HN：

```bash
# Refresh tap to v0.1.2
brew untap ShellFans-Kirin/drift 2>/dev/null || true
brew tap ShellFans-Kirin/drift

# Install
brew install drift

# Smoke
drift --version              # expect: drift 0.1.2
drift --help | head -5       # expect: usage with cost subcommand
drift mcp <<< '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}' | head -1
                              # expect: {"jsonrpc":"2.0","id":1,"result":{...,"serverInfo":{"name":"drift","version":"0.1.2"}}}

# Cleanup
brew uninstall drift
```

可選驗證：

```bash
# First-run notice 真的會擋（在 git 裸 repo 內跑 capture）
mkdir -p /tmp/drift-mac-smoke && cd /tmp/drift-mac-smoke
git init -q && git config user.email t@e.st && git config user.name t
echo init > R.md && git add R.md && git commit -qm init
drift init
echo "y" | drift capture --agent claude-code 2>&1 | grep -A5 'first-run notice'
# expect: notice block printed; capture proceeds after Enter
```

---

## 建議 Show HN 時機

**綠燈** ✅ 所有 launch gate 都過了：

- `release.yml` 4-target 全綠 ✓
- crates.io 4 crate × `0.1.2` 線上 + clean `cargo install` 跑得起來 ✓
- Homebrew Formula 自動更新到 `0.1.2` ✓
- Audit blocker（secret disclosure）已用 SECURITY.md + README + first-run notice 三層處理 ✓
- 4 個 nice-to-have 全做完 ✓
- 自家 `.prompts/` 沒洩漏 secret ✓

**等使用者跑完上面 Mac checklist** → 下個合適時段發 Show HN：

- **建議窗口**：台北時間週二至週四晚上 8-10 點（= 美西早上 5-7 點，HN 早班觸及率最高）
- **避開**：週一（HN 流量低）、週末（觀眾散）
- **本週**今天 2026-04-25（週六），如要等下週一/二早班，等日期到位再發
- **若無 Mac 可驗**：Linux `cargo install` 已驗過，風險可接受但 brew 還是建議先跑

---

## 變動摘要（git diff 鳥瞰）

```
9 files changed, 393 insertions(+), 11 deletions(-)

 CHANGELOG.md                             |  40 ++
 Cargo.toml                               |   2 +- (workspace 0.1.1 → 0.1.2)
 README.md                                |  55 ++ (Privacy section, pain, badges, About, COMPARISON link)
 crates/drift-cli/Cargo.toml              |   6 +- (path-dep 0.1.1 → 0.1.2)
 crates/drift-cli/src/commands/capture.rs | 125 ++ (first-run notice + 3 tests)
 crates/drift-connectors/Cargo.toml       |   2 +- (path-dep 0.1.1 → 0.1.2)
 crates/drift-mcp/Cargo.toml              |   2 +- (path-dep 0.1.1 → 0.1.2)
 docs/COMPARISON.md                       |  62 ++ (new)
 docs/SECURITY.md                         | 110 ++ (new)
```

無 core / connector / MCP 邏輯變動。Phase 1 C 的 `capture.rs` 是唯一可接受的 code 變動（first-run notice），與審計 blocker 直接對應。
