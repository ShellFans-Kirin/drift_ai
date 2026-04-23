# 帳號搬家報告：shellfans-dev → ShellFans-Kirin

**日期**：2026-04-23
**範圍**：把所有 drift_ai 對外資源從 `shellfans-dev` 帳號整併到 `ShellFans-Kirin`，維持 crates.io / Homebrew / GitHub 品牌一致性。`shellfans-dev` 帳號保留於未公開的開發工作（跟 drift_ai 無關）。

---

## 終局資源對照

| 資源 | 搬家前 | 搬家後 |
|---|---|---|
| 主 repo | `shellfans-dev/drift_ai` | **`ShellFans-Kirin/drift_ai`**（repo transfer，保留 issues / PRs / releases / stars / tags） |
| Homebrew tap | 未建立（計畫 `shellfans-dev/homebrew-drift`） | **`ShellFans-Kirin/homebrew-drift`**（公開，M5 新建） |
| Commit author（drift_ai repo local） | `kirin <kirin@shell.fans>` | **`ShellFans-Kirin <kirin@shell.fans>`**（local-only，global 未動） |
| crates.io 發行帳號 | 未發行 | 預計 `ShellFans-Kirin`（v0.1.1 本體執行） |

> Canonical 拼法：GitHub API 回傳 `.login` 為 `ShellFans-Kirin`（大小寫交錯）。所有 artifacts（URLs / Cargo.toml / README / commit author / Homebrew tap namespace）統一用 canonical 拼法；GitHub 本身兩種 case 都 resolve，但 canonical 在 search、commit、跨服務顯示一致性最好。

---

## Phase 執行記錄

### M1 · 前置驗證

- ✅ 身分：`GH_TOKEN=$SHELLFANS_KIRIN_PAT gh api user` → `ShellFans-Kirin`
- ⚠️ 安全事件：原 prompt 的第一步 `echo "${SHELLFANS_KIRIN_PAT:+SET}${SHELLFANS_KIRIN_PAT:-UNSET}"` 會在 PAT 有設時把 **完整 PAT 值印到 stdout**（shell expansion 細節：`${VAR:-X}` 有值展開為值，非 X）。此 idiom 昨日（2026-04-22 ~ 2026-04-23）兩度洩漏 PAT，今次已用 `[ -n "$VAR" ]` 替代，無再發生。詳見 `~/.claude/projects/-home-kirin/memory/feedback_secret_verification_bash.md`。
- ✅ Scope：fine-grained PAT，權限為 per-resource 不是 OAuth scope；確認足以執行後續 API（repo admin / secrets / workflow）。
- ⚠️ PAT 無 `Email addresses (read)` 權限，`GET /user/emails` 回 403；改以使用者直接提供 email (`kirin@shell.fans`) 用於 git config。
- ✅ Repo 衝突檢查：`ShellFans-Kirin/drift_ai` **已存在**（搬家於此次 session 前手動完成，而非本 session 觸發 transfer API）；`ShellFans-Kirin/homebrew-drift` 不存在 → 可建立。
- ✅ Source repo state（via redirect）：private, default_branch=main, 1 open issue, 0 stars, 124 KB, v0.1.0 release 保留（`drift-v0.1.0-x86_64-unknown-linux-gnu.tar.gz` + `.sha256`），tags `[v0.1.0]`。

### M2 · Repo transfer（mop-up only）

Repo 轉移在本 session 前已手動完成，本階段僅執行 cleanup：

- ✅ Local remote `git remote set-url origin` → `https://github.com/ShellFans-Kirin/drift_ai.git`
- ✅ `git fetch origin` 成功
- ✅ v0.1.0 release assets 完整性驗證：透過 API `/releases/assets/{id}` endpoint 下載 tarball（2,334,055 bytes），實測 sha256 = `fca7234401ad4da0943e894e387d94174b6121dc646d7d2807486a71e407cac3`，**完全符合 release 公告值**。
- ⚠️ `browser_download_url`（即 `github.com/.../releases/download/...`）在 repo 為 private 時連帶 auth 都回 404；這是 GitHub 的設計 — 私有 repo asset 只能走 API endpoint。一旦 visibility 轉 public，`browser_download_url` 即 200。

### M3 · Git identity 切換

- ✅ `git config --local user.name "ShellFans-Kirin"` + `user.email "kirin@shell.fans"`
- ✅ Global git config 未動（保留 `kirin / kirin@shell.fans`）
- ✅ Marker commit `a2708ba`：`chore: switch commit author to ShellFans-Kirin`，author 顯示 `ShellFans-Kirin <kirin@shell.fans>` ✓

### M4 · 全 repo URL find-replace

- 規則 1：`shellfans-dev/drift_ai` → `ShellFans-Kirin/drift_ai`
- 規則 2：`shellfans-dev/homebrew-drift` → `ShellFans-Kirin/homebrew-drift`
- 規則 3：裸 `shellfans-dev`（僅指帳號，如「authenticated as shellfans-dev」）**保留**，因其記錄當時事實

**受影響檔案（17）**：

`Cargo.toml`（workspace `repository`/`homepage`，4 member 全繼承）/ `README.md` / `CONTRIBUTING.md` / `CODE_OF_CONDUCT.md` / `.github/workflows/release.yml` / `docs/DELIVERY-v0.1.0.md` / `docs/PHASE0-EXECUTION-REPORT.md` / `docs/PHASE0-PROPOSAL.md` / `docs/PHASE0-VERIFICATION.md` / `docs/STEP1-5-COMPLETION-REPORT.md` / `docs/distribution/drift.rb.template` / `docs/launch/awesome-claude-plugins-pr.md` / `docs/launch/hn-show-hn.md` / `docs/launch/twitter-thread.md` / `plugins/claude-code/.claude-plugin/plugin.json` / `plugins/codex/marketplace.json` / `.gitignore`

- Commit `a779dc5`（57 insertions / 56 deletions）
- `.gitignore` 新增 `.prompts/` — 初次執行時因 `git add -A` 誤把 drift_ai 本地產生的 `.prompts/events.db`（57KB SQLite）一併 stage；經 `git reset --soft HEAD~1` 回溯、unstage、補 gitignore 後以新 commit 重做。舊 commit hash `718eaeb` 僅存於 reflog，未 push，~30 天 `git gc` 回收。

### M5 · Homebrew tap 建立

- ✅ `GH_TOKEN=$PAT gh repo create ShellFans-Kirin/homebrew-drift --public --description "Homebrew tap for Drift AI — the git blame for AI-era code" --homepage https://github.com/ShellFans-Kirin/drift_ai`
- ✅ Clone 至 `/home/kirin/homebrew-drift`，設 local git identity
- ✅ Seed `README.md`（commit `411ee5a`）：install 指令 `brew install ShellFans-Kirin/drift/drift`；Formula 本體待 v0.1.1 dispatch pipeline 生成
- ✅ Public visibility：anonymous `curl -sI https://github.com/ShellFans-Kirin/homebrew-drift` 回 200

### M6 · 跨 repo workflow 認證

- ✅ `TAP_REPO_PAT` secret 設入 `ShellFans-Kirin/drift_ai`
  - 命令：`GH_TOKEN="$PAT" gh secret set TAP_REPO_PAT --repo ShellFans-Kirin/drift_ai <<< "$PAT"`
  - 值走 bash here-string（shell builtin，in-shell，無 `ps` 洩漏；值不出現在命令列字串上）
- ✅ 驗證：`gh secret list` 顯示 `TAP_REPO_PAT 2026-04-23T06:27:49Z`（值本身不可讀）
- ✅ drift_ai `release.yml` 新增兩步（commit `29b92f3`）：
  - `Collect sha256 outputs`：讀四個 target 的 `.sha256` sidecar，輸出為 step outputs
  - `Dispatch tap update`：使用 `peter-evans/repository-dispatch@v3` + `${{ secrets.TAP_REPO_PAT }}` 對 `ShellFans-Kirin/homebrew-drift` 發 `drift-released` 事件，payload 含 `version` 與所有 4 target sha256
- ✅ homebrew-drift `update-formula.yml` 新增（commit `e4570a1`）：
  - 觸發：`repository_dispatch[drift-released]` + `workflow_dispatch`（手動重跑）
  - 邏輯：從 payload 或 release sha256 sidecar 取值，生成 `Formula/drift.rb`（macOS arm/x86 + Linux arm/x86 四 target），由 `github-actions[bot]` commit + push

### M7 · 驗證

| 檢查 | 結果 |
|---|---|
| 本機 remote 指向 `ShellFans-Kirin/drift_ai` | ✅ |
| 本機 git config：`user.name=ShellFans-Kirin`, `user.email=kirin@shell.fans`（local only） | ✅ |
| Global git config 未動 | ✅ |
| 最近 3 個新 commit author 都是 `ShellFans-Kirin <kirin@shell.fans>` | ✅ |
| 全 repo 無 `shellfans-dev/drift_ai` 或 `shellfans-dev/homebrew-drift` 字樣 | ✅ |
| Cargo.toml `repository`/`homepage` 指向新 URL；4 member 全繼承 | ✅ |
| `TAP_REPO_PAT` secret 已設（值不可讀） | ✅ |
| `release.yml` 含 `repository-dispatch` + `TAP_REPO_PAT` 引用 | ✅ |
| `homebrew-drift/.github/workflows/update-formula.yml` 存在 | ✅ |
| drift_ai + homebrew-drift 工作樹皆乾淨 | ✅ |
| v0.1.0 release assets 完整（via API endpoint） | ✅ |
| **anonymous 直接連結到 `ShellFans-Kirin/drift_ai`** | ⚠️ 404 — repo 仍為 private |
| **`shellfans-dev/drift_ai` → canonical 的 web redirect** | ⚠️ 無回應 — private repo 不對外 redirect |

---

## 未完項（block v0.1.1 本體的前置）

### 1. `ShellFans-Kirin/drift_ai` visibility：private → public？

當前 `visibility: private`。對 migration 本身無影響，但影響：

- **Homebrew formula 一旦生成會 404**：`brew install` 走 anonymous HTTP，drfit_ai 是 private 時 asset URL 無論 auth 與否都 404（GitHub 私有 repo 隱藏存在性的設計）。
- **crates.io publish 可進行**：crates.io 只讀 `Cargo.toml` 的 `repository` field 做 metadata 連結，不實際下載；但使用者點進連結會看到 "Repository not found"。
- **公開分發計畫**：v0.1.0 原計劃 shipped，但實際對外完全不可見直到 public。

**建議動作**（等使用者下令）：

```bash
GH_TOKEN="$SHELLFANS_KIRIN_PAT" gh repo edit ShellFans-Kirin/drift_ai \
  --visibility public --accept-visibility-change-consequences
```

半可逆 — 可再改回 private，但 public 期間可能被 crawler / archive.org / 其他鏡像抓走。

### 2. v0.1.1 本體（已在 session 外等待）

Migration 完成後才進 v0.1.1 Phase A。v0.1.1 的具體範圍（Actions 啟用、macOS/aarch64 二進位、Anthropic HTTP、cargo publish）見 v0.1.1 prompt 與 `project_drift_ai_v0_1_0.md` memory。

---

## 參考 commit 清單

**drift_ai** (`ShellFans-Kirin/drift_ai`)：

```
29b92f3 ci(release): dispatch drift-released event to homebrew-drift tap
a779dc5 chore: update repo URLs after transfer to ShellFans-Kirin
a2708ba chore: switch commit author to ShellFans-Kirin
1b4ce31 docs: add v0.1.0 delivery summary (zh-TW)  ← 搬家前最後一個 commit
```

**homebrew-drift** (`ShellFans-Kirin/homebrew-drift`，搬家後新建）：

```
e4570a1 ci: add update-formula workflow
411ee5a chore: seed Homebrew tap README
```

---

## 安全摘要

- **PAT 處理**：全程使用環境變數 `SHELLFANS_KIRIN_PAT`，從未寫入任何檔案（`.git/config`、`.cargo/config.toml`、`.env` 皆未觸碰），從未出現在任何 commit，從未 echo / print / log 到 stdout/stderr。
- **Git push 認證**：`GH_TOKEN="$PAT" git -c credential.helper='!gh auth git-credential' push …` — gh 從 env 讀 token，無需落 `.git/config`。
- **Secret 設定**：使用 here-string `<<<` 餵給 `gh secret set` 的 stdin；bash 建構不經 argv / exec，`ps` 觀察不到。
- **gh CLI 狀態**：`gh auth status` 顯示 env-based `ShellFans-Kirin` token（GH_TOKEN）與 config-based `shellfans-dev` token（hosts.yml）並存，前者 active，後者保留；未 `gh auth login` 切帳號以避免污染 `~/.config/gh/hosts.yml`。
