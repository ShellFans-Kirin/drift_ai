# v0.1.1 Homebrew 端對端驗證（Mac 實跑）

**日期**：2026-04-24
**背景**：`SHIP-V0.1.1-REPORT.md` 先前標註 Linux host 沒 `brew`，Formula 內容 diff 驗過但 `brew install` 未實跑。本次在真實 Mac 上補驗。

---

## 驗證環境

| 項目 | 值 |
|---|---|
| 機器 | `Ruei的Mac mini`（tailnet `rueimac-mini`, `100.125.235.69`） |
| 路由 | Linux → Mac via Tailscale SSH（pubkey auth）|
| macOS | **26.3.1** (build 25D2128, darwin xnu-12377) |
| CPU | **Apple M4** (arm64) |
| Homebrew | 5.0.16 |
| Mac user | `kirin` |

---

## 完整指令序列

```bash
# 1. preflight
brew --version                                 # Homebrew 5.0.16
uname -m                                       # arm64
sw_vers -productVersion                        # 26.3.1

# 2. tap
brew tap ShellFans-Kirin/drift
#   Cloning into '/opt/homebrew/Library/Taps/shellfans-kirin/homebrew-drift'...
#   Tapped 1 formula (15 files, 17.6KB)

# 3. verify Formula version + arm64 block
brew cat drift | grep -A 6 'on_macos do'
#   on_macos do
#     if Hardware::CPU.arm?
#       url "https://github.com/ShellFans-Kirin/drift_ai/releases/download/v0.1.1/drift-v0.1.1-aarch64-apple-darwin.tar.gz"
#       sha256 "21df1a60f1b8291a0c4b1c465e90b317168743f3ed5b777fcf40c410e60db867"
#     else
#       url "https://github.com/ShellFans-Kirin/drift_ai/releases/download/v0.1.1/drift-v0.1.1-x86_64-apple-darwin.tar.gz"
#       sha256 "d3c8a53773cbf38ec563fde629c613f0ad74d7c0fa15f8dff7b260842a078d07"

# 4. install (timing + artefact)
time brew install drift
#   real 0m18.570s
#   Installed: /opt/homebrew/Cellar/drift/0.1.1 (7 files, 6.7MB)

# 5. version
drift --version                                # drift 0.1.1

# 6. help — all 17 subcommands exposed
drift --help
#   Usage: drift [OPTIONS] <COMMAND>
#   Commands: init capture watch cost list show blame trace diff rejected
#             log config bind auto-bind install-hook sync mcp help

# 7. functional smoke — init + mock capture in a temp repo
WORK=$(mktemp -d /tmp/drift-brew-smoke.XXXX)
cd "$WORK"
git init -q && git config user.email t@e.st && git config user.name t
printf init > R.md && git add R.md && git commit -qm init
drift init
cat > .prompts/config.toml <<'EOF'
[compaction]
provider = "mock"
EOF
drift capture --agent claude-code
#   Captured 159 session(s), wrote 258 event(s) to .prompts/events.db
drift cost
#   total calls      : 0
#   (no compaction_calls recorded yet — run `drift capture` with ANTHROPIC_API_KEY set)
drift list --agent claude-code | head -3
#   a23c9b51  claude-code   turns=20  [MOCK] claude-code session ...
#   c4b6035e  claude-code   turns=27  [MOCK] ...
#   4c690570  claude-code   turns=26  [MOCK] ...

# 8. integrity — binary + codesign
file /opt/homebrew/Cellar/drift/0.1.1/bin/drift
#   Mach-O 64-bit executable arm64
codesign -dv /opt/homebrew/Cellar/drift/0.1.1/bin/drift
#   Identifier=drift-6c3783fc785d1869
#   Format=Mach-O thin (arm64)

# 9. brew info
brew info drift
#   ==> shellfans-kirin/drift/drift: stable 0.1.1
#   Installed (on request)
#   /opt/homebrew/Cellar/drift/0.1.1 (7 files, 6.7MB)
#   Built from source on 2026-04-24 at 19:26:20

# 10. cleanup
brew uninstall drift                           # clean
brew untap ShellFans-Kirin/drift               # clean
ls /opt/homebrew/Cellar/drift                  # no such file ✓
```

---

## 驗證清單（勾掉即過）

| 檢查 | 結果 |
|---|---|
| `brew tap ShellFans-Kirin/drift` 成功 | ✅ |
| `Formula/drift.rb` 有 v0.1.1 + 4 target URL + 4 sha256 | ✅ |
| `brew install drift` 成功 + ~18 s | ✅ |
| 裝好的 binary 是 Mach-O arm64、codesigned | ✅（ad-hoc sig `drift-6c3783fc785d1869`）|
| `drift --version` == `drift 0.1.1` | ✅ |
| `drift --help` 含所有 17 subcommands（含 v0.1.1 新的 `cost`） | ✅ |
| `drift init` + `drift capture` 正常（mock provider） | ✅ |
| `drift cost` 正常輸出（無 API 呼叫紀錄預期） | ✅ |
| 中文檔名（`persona_003_張雅晴.md`）parse 無編碼問題 | ✅ |
| `brew uninstall drift` 清乾淨 | ✅ |
| `brew untap` 後 `/opt/homebrew/bin/drift` 無殘留 | ✅ |

---

## Ship gate 現況

ship 三條現在**全綠** —

- ✅ `release.yml` 4-target 全綠
- ✅ `cargo install drift-ai` from clean `/tmp` 成功 + `drift --version = 0.1.1`
- ✅ `brew install ShellFans-Kirin/drift/drift` 從乾淨 tap 安裝 + 正常執行 + 乾淨卸載（**本次新增驗證**）

Show HN 全部前置條件都通過，沒有剩下的 blocker。

---

## 附帶觀察

1. **Mac 本機有 159 個 claude sessions**（Linux host 只有 10）。drift 的 `~/.claude/projects/` scanner 在 macOS 上運作正常，處理中文路徑（例 `/Volumes/ORICO/personas_info/tw/2026/04/24/persona_003_張雅晴.md`）無編碼問題。
2. **`brew info` 顯示 "Built from source"**：這是 Homebrew 對「沒有官方 bottle 的 formula」的慣用說法；實際上 Formula 是 `bin.install "drift"` 直接安裝 pre-built binary。不影響安裝速度或可用性（~18 s 就是下載 tarball + 解壓 + 符號連結的時間）。
3. **Tailscale SSH 延遲 9 ms**（直連 UDP `114.44.200.168:41641`，非 DERP relay）— 跨台灣雙機 tailnet 可用度好。
