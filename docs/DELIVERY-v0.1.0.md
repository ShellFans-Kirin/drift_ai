# Drift AI v0.1.0 — 完工交付總結

**日期**：2026-04-22
**版本**：v0.1.0
**狀態**：已 merge 到 `main`、已 tag、已發行
**對應工程報告**：[`docs/STEP1-5-COMPLETION-REPORT.md`](STEP1-5-COMPLETION-REPORT.md)

---

## 🔗 交付連結

| 項目 | URL |
|---|---|
| Repo | https://github.com/ShellFans-Kirin/drift_ai |
| PR phase1-through-5（已 squash-merge） | https://github.com/ShellFans-Kirin/drift_ai/pull/2 |
| v0.1.0 tag | https://github.com/ShellFans-Kirin/drift_ai/tree/v0.1.0 |
| GitHub Release | https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.1.0 |
| STEP1-5 工程完工報告 | https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/STEP1-5-COMPLETION-REPORT.md |
| VISION | https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/VISION.md |
| PHASE0-PROPOSAL | https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/PHASE0-PROPOSAL.md |

---

## ⚡ 一行 Quickstart

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git && cd drift_ai && cargo build --release && \
  TMP=$(mktemp -d) && (cd $TMP && git init -q && git config user.email x@y && git config user.name x && \
  /home/kirin/drift_ai/target/release/drift init && /home/kirin/drift_ai/target/release/drift capture && \
  /home/kirin/drift_ai/target/release/drift list | head)
```

或安裝後：

```bash
cargo install --path crates/drift-cli
cd your-repo && drift init && drift capture && drift blame src/foo.rs
```

---

## 🤝 雙 agent Demo（真實主機資料）

```
$ drift capture
Captured 9 session(s), wrote 91 event(s) to .prompts/events.db

$ drift list | head -4
4b1e2ba0  claude-code   turns=363 ...
40c15914  claude-code   turns=9   files touched: hi.txt
019daed6  codex         turns=24  ...
019dae93  codex         turns=6   ...
```

Claude Code + Codex 各 capture ≥ 1 份成功 ✓

---

## 🔍 drift blame 實際輸出

```
$ drift blame hi.txt
hi.txt
├─ 2026-04-21 06:59  💭 [codex] session 019daed6
│   --- a/hi.txt
│   +++ b/hi.txt
│   @@ -0,0 +1,2 @@
│   +hello
│   +drift
├─ 2026-04-21 07:00  💭 [claude-code] session 40c15914
│   +hello
│   (rejected suggestion)
```

跨 agent + rejected 訊號，從真實 session 資料算出 ✓

---

## 🧰 MCP Tools + 呼叫範例

**5 個 read-only tools**：`drift_blame` / `drift_trace` / `drift_rejected` / `drift_log` / `drift_show_event`

Stdio JSON-RPC 2.0：

```json
→ {"jsonrpc":"2.0","id":1,"method":"initialize"}
← {"jsonrpc":"2.0","id":1,"result":{"capabilities":{"tools":{}},
    "protocolVersion":"2024-11-05","serverInfo":{"name":"drift","version":"0.1.0"}}}

→ {"jsonrpc":"2.0","id":2,"method":"tools/list"}
← {"jsonrpc":"2.0","id":2,"result":{"tools":[drift_blame, drift_trace,
    drift_rejected, drift_log, drift_show_event]}}

→ {"jsonrpc":"2.0","id":3,"method":"tools/call",
    "params":{"name":"drift_blame","arguments":{"file":"hi.txt"}}}
← {"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"[...events JSON...]"}]}}
```

```bash
claude mcp add drift -- drift mcp   # 一次完成 Claude Code 整合
```

---

## 📦 發行狀態

| 通路 | 狀態 | 細節 |
|---|---|---|
| **GitHub Releases binary** | ✅ 上線 | [v0.1.0 release](https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.1.0)<br>`drift-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`<br>sha256：`fca7234401ad4da0943e894e387d94174b6121dc646d7d2807486a71e407cac3`<br>*macOS + aarch64 tarball 等 Actions 啟用後由 release.yml 自動補上* |
| **Homebrew** | 📋 模板已 commit | `docs/distribution/drift.rb.template`<br>手動 3 步：建 `ShellFans-Kirin/homebrew-drift` tap → 填 sha256 → push |
| **cargo publish** | ✅ dry-run 過 | `cargo publish --dry-run -p drift-core --allow-dirty` 通過（15 files, 87.3 KiB）<br>手動 publish 順序：`drift-core` → `drift-connectors` → `drift-mcp` → `drift-ai` |

---

## 🔑 `ANTHROPIC_API_KEY` 使用狀態

**Mock-only**。主機未設 key，整條 pipeline 跑 `MockProvider`，所有 compacted summary 標 `[MOCK]`。`AnthropicProvider` 骨架已寫，`crates/drift-core/src/compaction.rs` 裡有 marker comment 指出 HTTP 接線點（加 `reqwest` + `POST /v1/messages`，~30 行）。

---

## ⚠️ 已知未完成項

1. Anthropic HTTP 實際接線（Mock 是 shipping 預設）
2. `drift watch` 是 3 秒 debounce polling，非 event-driven
3. Codex `apply_patch` Update hunk 解析器簡化版，Add/Delete/Move 完整
4. 人類手改的 diff `before` 為空（SHA drift 本身可靠，完整 before 需 blob cache）
5. 沒 Windows binary（CI matrix：Linux + macOS）
6. GitHub Actions 在本 repo 首次 push 時 startup_failure — repo Settings 啟用 Actions 後 release.yml 即可自動建 4 target binaries
7. `cargo publish` 未實際跑（任務規則：你手動）
8. Homebrew tap repo 未建（任務規則：你手動）
9. Plugin manifest 已 commit 於 `plugins/`，未 submit marketplace（v0.2 目標）

---

## 👉 下一步建議

| 優先級 | 動作 |
|---|---|
| 🔴 高 | repo Settings 啟用 GitHub Actions，重跑 release.yml 補齊 macOS + aarch64 binary |
| 🔴 高 | `cargo publish` 依序 4 crate（搶 crates.io 名字） |
| 🟡 中 | 建 `ShellFans-Kirin/homebrew-drift` tap，貼 Formula |
| 🟡 中 | 接線 AnthropicProvider HTTP（~30 行，換掉 `[MOCK]`） |
| 🟢 低 | 送 PR 到 `ComposioHQ/awesome-claude-plugins`（草稿 `docs/launch/awesome-claude-plugins-pr.md`） |
| 🟢 低 | 發 Show HN + Twitter thread（草稿 `docs/launch/hn-show-hn.md`、`twitter-thread.md`） |
| 🟢 低 | v0.2：publish plugin manifests 到 Claude Code + Codex marketplace |

---

## ✅ 驗證結果

- **28 tests green**（14 core + 5 connector unit + 6 integration + 1 human-edit + 1 MCP unit + 1 MCP stdio smoke）
- `cargo fmt --all -- --check` clean
- `cargo clippy --all-targets --all-features -- -D warnings` clean
- Release binary 上線並可下載
- 雙 agent 真實資料 demo 通過
- MCP stdio round-trip 通過

Phase 1 → 5 如期完成，v0.1.0 已 tag + release。
