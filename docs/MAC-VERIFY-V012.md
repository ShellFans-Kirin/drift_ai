# Mac Verification Report — drift v0.1.2

**日期**：2026-04-25
**驗證方式**：在一台 Apple M4 / macOS 26.3.1 的 Mac 上實跑全套 Homebrew
install / smoke / cleanup(作者人在 Linux,透過 SSH 連到自己的 Mac 跑)。
**Mac 架構**：**arm64** (Apple Silicon)
**Homebrew 版本**：**5.1.7** at `/opt/homebrew/bin/brew`

---

## Phase 1 — 環境盤點

| 檢查 | 結果 | 證據 |
|---|---|---|
| SSH 連通 (Tailscale) | ✅ | `OK\nDarwin arm64`, exit=0 |
| brew 在 PATH | ✅ | `/opt/homebrew/bin/brew`, version 5.1.7 |
| Mac 架構 | arm64 | 預期走 Formula `on_macos do · if Hardware::CPU.arm?` 分支 |
| 既有 tap | ❌ NO_TAP | 從零開始 |
| 既有 drift 安裝 | ❌ NOT_INSTALLED | 從零開始 |

---

## Phase 2 — 端對端驗證

| 步驟 | 結果 | exit | 證據 |
|---|---|---|---|
| **2A** Defensive cleanup | ✅ | 0 | (no-op：本來就沒裝) |
| **2B** `brew tap ShellFans-Kirin/drift` | ✅ | 0 | `Tapped 1 formula (15 files, 18.3KB)`；Formula 拉到 `/opt/homebrew/Library/Taps/shellfans-kirin/homebrew-drift/Formula/drift.rb` |
| **2C** `brew install drift` | ✅ | 0 | `Fetching downloads for: drift` → `Formula drift (0.1.2)` → `🍺 /opt/homebrew/Cellar/drift/0.1.2: 7 files, 6.7MB`，**8.4 秒** wall time |
| **2D** `drift --version` | ✅ | 0 | `drift 0.1.2` |
| **2E** `drift --help` | ✅ | 0 | 列出全部 17 subcommands 含新增的 `cost` |
| **2F** `drift cost --help` | ✅ | 0 | 印 `Aggregate compaction token usage and cost` + 4 個旗標（`--since` / `--until` / `--model` / `--by`） |
| **2G** `drift mcp` initialize | ✅ | 0 | JSON-RPC response `serverInfo.name=drift`, `serverInfo.version=0.1.2` |
| **2H** `DRIFT_SKIP_FIRST_RUN=1` bypass | ✅ | 0 | `drift capture` 不 hang on stdin；mock provider 完整跑完 160 sessions / 268 events |

### 2G 完整 MCP 回應

```json
{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"tools":{}},"protocolVersion":"2024-11-05","serverInfo":{"name":"drift","version":"0.1.2"}}}
```

### 2H 一段 capture 輸出證明 first-run notice 沒擋住

```
Initialised /private/tmp/drift-mac-verify.XXXX/.prompts with config at .../.prompts/config.toml
drift capture · provider=mock (set [compaction].provider="anthropic" + ANTHROPIC_API_KEY for live compaction)
Captured 160 session(s), wrote 268 event(s) to .../.prompts/events.db
```

`DRIFT_SKIP_FIRST_RUN=1` 確實 bypass 掉 stdin Enter prompt（沒 hang），同時**沒寫**
`~/.config/drift/state.toml`（與 Phase 1-test `skip_env_var_bypasses_notice` 設計一致）。

---

## Phase 3 — Formula self-test + cleanup

| 步驟 | 結果 | exit | 證據 |
|---|---|---|---|
| **3A** `brew test drift` | ✅ | 0 | `==> Testing shellfans-kirin/drift/drift` → `==> /opt/homebrew/Cellar/drift/0.1.2/bin/drift --version`（Formula `test do` 區塊：`assert_match "drift", shell_output("#{bin}/drift --version")`）|
| **3B** `brew uninstall drift` | ✅ | 0 | `Uninstalling /opt/homebrew/Cellar/drift/0.1.2... (7 files, 6.7MB)` |
| **3B** `brew untap ShellFans-Kirin/drift` | ✅ | 0 | `Untapped 1 formula (15 files, 18.3KB)` |
| **3C** Cellar / bin / tap 殘留 | ✅ | — | `Error: No available formula "drift"`、`NO_TAP`、`/opt/homebrew/bin/drift: No such file` |

---

## 附帶觀察

1. **brew 自動清掉舊版 cache**：`brew install drift` 跑完順手 `Removing /Users/kirin/Library/Caches/Homebrew/drift--0.1.1.tar.gz... (3.3MB)` — 昨天 v0.1.1 verify 留下的 tarball 自動回收，不必手動清。
2. **Mac 本機有 160 個 claude sessions**（昨天 v0.1.1 verify 是 159 — 一日內多一個）。Connector 對 macOS HFS+/APFS 不分大小寫的特性沒問題；中文路徑（如 `/Volumes/ORICO/personas_info/tw/2026/04/24/persona_*.md`）也正常 parse。
3. **首次 SSH `timeout` 不存在**：macOS 沒裝 GNU `coreutils`，`timeout` 指令找不到（Linux 慣用方式）。改用 `command & pid=$!; sleep N; kill $pid` 模式，跨平台。本報告用後者方法。
4. **`brew test`** 第一次跑會自動進 Homebrew developer mode 並安裝一堆 Ruby gem（44 個），這是 Homebrew 規範化測試環境的副作用，跟 drift 自身無關。Test 區塊本體只跑 `drift --version` 一行。

---

## 結論

🟢 **drift v0.1.2 在 Apple Silicon (arm64) Mac + Homebrew 端對端通過**。

涵蓋：tap → install → smoke (version / help / cost / mcp / first-run-bypass) → Formula self-test → uninstall → untap → 殘留掃描。8/8 Phase 2 smokes + 3/3 Phase 3 cleanup 全綠，無一失敗。

v0.1.2 **真的可發 Show HN**，沒有任何剩餘 launch blocker。

---

## 不在本驗證範圍

以下意圖性留白：

- **Intel Mac (x86_64)**：作者沒 Intel Mac 可驗。Formula `on_macos do · else` 分支跟 arm64 分支邏輯一致，差別只在下載 URL + sha256（兩者都已在 v0.1.2 release 上線且 sha256 與 Formula 對齊 — 見 `docs/V012-SHIP-REPORT.md`）。Intel 用戶實裝若有問題，issue 報。
- **Linux brew**：Linux 也支援 brew (Linuxbrew)，但作者本機沒裝。`cargo install drift-ai` 已在 Linux 端從 `/tmp` 乾淨環境驗過（見 V012-SHIP-REPORT.md Phase 4-I）。
- **`drift watch` 在 macOS FSEvents** 整合：本次只驗 install / smoke，沒實跑 watch。`drift watch` 在 Linux 已驗過（v0.1.1 ship）。`notify` crate 對 FSEvents 的支援是 mature，理論上 work — 等 dogfood 暴露問題再補。
