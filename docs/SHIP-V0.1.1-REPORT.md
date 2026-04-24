# drift_ai v0.1.1 — Ship Report

**Date**: 2026-04-24
**Scope**: v0.1.1 launch-ready patch release delivered end-to-end.

---

## 交付連結

- **GitHub Release**: <https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.1.1>
  - 4 × tarball + 4 × `.sha256`：`aarch64-apple-darwin` / `x86_64-apple-darwin` / `aarch64-unknown-linux-gnu` / `x86_64-unknown-linux-gnu`
- **crates.io**（4/4 都 `max_version = 0.1.1`）：
  - <https://crates.io/crates/drift-core>
  - <https://crates.io/crates/drift-connectors>
  - <https://crates.io/crates/drift-mcp>
  - <https://crates.io/crates/drift-ai>
- **Homebrew tap**: <https://github.com/ShellFans-Kirin/homebrew-drift>
  - `Formula/drift.rb` auto-updated 至 `version "0.1.1"`，4 個 sha256 全齊
  - Install: `brew install ShellFans-Kirin/drift/drift`
- **CHANGELOG**: <https://github.com/ShellFans-Kirin/drift_ai/blob/main/CHANGELOG.md>
- **Phase-by-phase 完工報告**: <https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/STEP-V011-COMPLETION-REPORT.md>

---

## 真實 compacted 輸出（Haiku，非 MOCK）

```markdown
# 80bfcde5-3658-4449-ae7b-334acd49762b — claude-code

## Summary
Configured sudo NOPASSWD privilege for user `kirin` on a host system.
Attempted to change the host IP address to 192.168.70.30 but encountered
incomplete communication in transcript.

## Key decisions
- Created `/etc/sudoers.d/kirin-nopasswd` — modular sudoers configuration
  follows best practices
- Verified with `sudo -n true` — ensures syntax correctness

## Files touched
- `/etc/sudoers.d/kirin-nopasswd`
```

對比 v0.1.0：

```
[MOCK] claude-code session 80bfcde5 with 19 turns; files touched: (none)
```

是實質差距。

---

## drift cost 輸出

```
$ drift cost
drift cost — compaction billing

  total calls      : 10
  input tokens     : 120958
  output tokens    : 6582
  cache creation   : 0
  cache read       : 0
  total cost (USD) : $0.1539

$ drift cost --by model
── grouped by model (descending cost)
  key                    calls   input_tok   output_tok     cost (USD)
  claude-haiku-4-5          10      120958         6582        $0.1539

$ drift cost --by session
── grouped by session (descending cost)
  key                                     calls   input_tok   output_tok     cost (USD)
  4b1e2ba0-621c-4977-af3f-2a9df5ac45ec        2       51696         2448        $0.0564
  ad01ae46-156f-403b-b263-dd04a232873a        1       33662         2390        $0.0456
  ...
```

---

## drift watch event-driven 驗證證據

```
drift watch · event-driven; Ctrl-C to stop
  watching /home/kirin/.claude/projects
  watching /home/kirin/.codex/sessions
  first run; capturing every session seen
Captured 13 session(s), wrote 211 event(s) to .prompts/events.db

# 寫 bbbbbbbb-1111-2222-3333-444444444444.jsonl 到 ~/.claude/projects/...
# 200 ms 後 watcher 自動觸發：
Captured 1 session(s), wrote 0 event(s) to .prompts/events.db

# SIGINT:
drift watch · interrupt received; exiting after last capture
```

`~/.config/drift/watch-state.toml` 寫入：

```toml
last_event_at = "2026-04-23T17:18:46.372489202Z"
```

---

## Homebrew Formula 自動化驗證

- `release.yml` → `repository_dispatch(drift-released)` → `update-formula.yml` 端到端跑過兩輪：
  - 第一次：手動對 v0.1.0 dispatch，驗證 pipeline（run `24849067665`, success）
  - 第二次：v0.1.1 tag push 自動觸發（run `24870796647`, success 於 `2026-04-24T03:32:56Z`）
- Formula `version "0.1.1"`，四個 sha256 全填，`on_macos` + `on_linux` 分支完整
- 匿名 `raw.githubusercontent.com` fetch 200 ✓

**caveat**：Linux host 沒裝 brew，所以 E4 是 Formula 內容 diff 驗證，不是真跑 `brew install`。Mac 端需使用者自跑一次：

```bash
brew tap ShellFans-Kirin/drift
brew install drift
drift --version   # expect: drift 0.1.1
```

---

## crates.io 四個頁面狀態

| Crate | URL | max_version | downloads |
|---|---|---|---|
| drift-core | <https://crates.io/crates/drift-core> | 0.1.1 | 0 |
| drift-connectors | <https://crates.io/crates/drift-connectors> | 0.1.1 | 0 |
| drift-mcp | <https://crates.io/crates/drift-mcp> | 0.1.1 | 0 |
| drift-ai | <https://crates.io/crates/drift-ai> | 0.1.1 | 0 |

**cargo install 從乾淨 `/tmp` 驗證通過**：

```
$ cargo install drift-ai --locked
   Compiling drift-mcp v0.1.1
   Compiling drift-ai v0.1.1
    Finished `release` profile [optimized] target(s) in 1m 52s
  Installing /tmp/drift-clean-cargo/bin/drift
   Installed package `drift-ai v0.1.1` (executable `drift`)

$ drift --version
drift 0.1.1
```

---

## 本次真實 Anthropic 用量

| Model | Calls | Input tok | Output tok | Cost (USD) |
|---|---|---|---|---|
| claude-opus-4-7 | 10 | 142,204 | 10,380 | **\$2.9116** |
| claude-haiku-4-5 | 10 | 120,958 | 6,582 | **\$0.1539** |
| **Total** | **20** | **263,162** | **16,962** | **\$3.0655** |

19× Opus/Haiku 成本差同資料集驗證到。`cache_creation` / `cache_read` 都 0 — `drift-core` 尚未對同一個 session 做多輪，沒機會觸發 prompt cache；未來 v0.2 context incremental 才會用到。

---

## 已知未完成項

- Linux host 沒裝 brew，`brew install` 真跑需 Mac 測一次（Formula 內容 diff 驗過，預期可裝）
- `cache_creation` / `cache_read` tokens 都 0：正常，同 session 不重跑不會命中 cache
- `drift watch` 在 Windows 是 best-effort：`notify` crate `ReadDirectoryChangesW` 理論支援但未實測，v0.1.1 scope 刻意放著
- Context window Strategy 2（hierarchical summarize）：code scaffold 有了但 feature-flag off；v0.2 開

---

## Show HN 時機建議

**綠燈 ✅ 可以發**。具體到：

1. ✅ `release.yml` 4-target 全綠（Phase D3 過）
2. ✅ `cargo install drift-ai` 從空目錄成功 + `drift --version = 0.1.1`（Phase F/H7 過）
3. ⚠️ `brew install ShellFans-Kirin/drift/drift` — Formula pipeline 綠但本機沒 brew 無法實測；發文前 Mac 試一次確認無意外

帖子草稿見 `docs/launch/hn-show-hn.md`（URL 已在 migration 階段換成 canonical `ShellFans-Kirin`）。

建議 Mac 端三條驗完後就發：

```bash
brew tap ShellFans-Kirin/drift
brew install drift
drift --version   # expect: drift 0.1.1
drift mcp         # expect: stdio MCP server boots, waits for init
```

---

## 過程中遇到的問題與處置

1. **`read -s` 不能在 Claude Code `!` bash-input 收 secret**（沒 TTY，echo 回空）
   → 改用 `chmod 600 ~/drift-secrets.md` 中介檔 + `set -a; source <(grep ...)`，secret 只存在檔案裡，command line 乾淨。session 結束 `shred -u` 清除。
2. **crates.io `/me` 403**（policy 層面限制；token 其實有效）
   → 換 `cargo publish --dry-run` 做 token validity smoke，dry-run 過就往下。
3. **crates.io publish 被 email 未驗證擋住**
   → 暫停等使用者到 <https://crates.io/settings/profile> 驗證，繼續。
4. **`macos-13` runner 派不到（stuck queued 30+ min）**
   → 棄用 macos-13，把 `x86_64-apple-darwin` 也放到 `macos-14` 跑（arm64 runner 以 Apple universal SDK 交叉編譯至 x86_64）。刪 tag、force-push tag、re-release。
5. **PAT 無 `actions:write` 不能 cancel 卡住的 workflow**
   → 請使用者到 web UI 按 Cancel（半副作用的共享動作本來就該人腦決策）。
6. **第一次 Formula auto-commit skip**（`git diff --quiet Formula/drift.rb` 對不存在檔案回 0）
   → 改成 `git add && git diff --cached --quiet`，新檔也能偵測，驗過。
7. **`drift init` 會 overwrite user-edited config.toml**（capture 頂端 implicit init）
   → init 改 idempotent，只在 config 不存在時寫 default。

---

## 安全合規摘要

- 三顆 env secret（`ANTHROPIC_API_KEY`、`CARGO_REGISTRY_TOKEN`、`SHELLFANS_KIRIN_PAT`）全程**沒落入** command line、transcript、git、commit message
- Git push 認證走 `GH_TOKEN=$PAT git -c credential.helper='!gh auth git-credential' push`，token 只走 env
- `gh secret set` 用 here-string `<<<`（bash builtin，不走 argv）
- `repository_dispatch` 用 `curl -X POST` + `Authorization: Bearer` header，token 只在 env 展開
- 唯一一次 `.prompts/events.db` 誤 stage（`git add -A`）在 push 前被 `git reset --soft HEAD~1` + 重做 commit 修掉；沒流出
- 中介檔 `~/drift-secrets.md` session 結束後 `shred -u` 清除

---

*報告撰寫：Claude Opus 4.7 (1M context) · session `ad01ae46-156f-403b-b263-dd04a232873a`*
