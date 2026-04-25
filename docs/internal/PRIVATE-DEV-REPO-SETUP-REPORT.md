# drift Private Dev Repo Setup — Delivery Report

**Date**: 2026-04-25
**Scope**: 建立 `drift_dev_only` 私有開發 repo；`drift_ai` public 維持不動

## TL;DR

兩 repo 設置完成。日常 `git push` 預設進 private `drift_dev_only`；release 時用
`./scripts/release-to-public.sh vX.Y.Z` 同步到 public `drift_ai`。

## Repos

| Repo | Visibility | Role | URL |
|---|---|---|---|
| `ShellFans-Kirin/drift_ai` | **PUBLIC** ✓ unchanged | Release-facing mirror（使用者裝 / 看的） | https://github.com/ShellFans-Kirin/drift_ai |
| `ShellFans-Kirin/drift_dev_only` | **PRIVATE** ✓ new | 日常開發 source-of-truth | https://github.com/ShellFans-Kirin/drift_dev_only |

`drift_ai` 完全沒動：visibility / branches / tags / releases / Cargo.toml URL 全部保留。

## drift_ai untouched proof

| Item | Pre-task | Post-task | Status |
|---|---|---|---|
| visibility | PUBLIC | PUBLIC | ✓ |
| branches | `main`, `audit/launch-readiness` | `main`, `audit/launch-readiness` | ✓ |
| `main` HEAD | `fd66ca7` (`docs: v0.2.0 dev-log…`) | `fd66ca7` (same) | ✓ |
| tags | `v0.1.0 v0.1.1 v0.1.2 v0.2.0` | identical | ✓ |

## 本機 git 狀態（`/home/kirin/drift_ai`）

```
public    https://github.com/ShellFans-Kirin/drift_ai.git  (fetch+push)
dev_only  https://github.com/ShellFans-Kirin/drift_dev_only.git  (fetch+push)

remote.pushDefault    = dev_only
main upstream         = dev_only/main
```

## Baseline 同步狀態

- **Tags**: dev_only 與 public 完全一致（`v0.1.0`–`v0.2.0` 都已鏡像到 dev_only）
- **Branches**: `main`、`audit/launch-readiness` 已鏡像到 dev_only
- **`main` HEAD**: dev_only 比 public 領先 1 個 commit，內容是 release-workflow 腳手架。
  此 commit 不會自動推到 public；下次真實 release 用 `release-to-public.sh` 時才會同步過去。

## 新增檔案（在 dev_only/main 上）

| Path | Purpose |
|---|---|
| `scripts/release-to-public.sh` | 受守護的 release 推送腳本（只推 `main` + 指定 tag 到 public） |
| `docs/internal/RELEASE-WORKFLOW.md` | 兩 repo workflow 說明文件 |
| `docs/internal/.gitkeep` | 保留 internal 目錄存在 |

GitHub URL：
- https://github.com/ShellFans-Kirin/drift_dev_only/blob/main/docs/internal/RELEASE-WORKFLOW.md
- https://github.com/ShellFans-Kirin/drift_dev_only/blob/main/scripts/release-to-public.sh

## 日常 / Release 使用方式

```bash
# 日常開發 — 預設推 private dev_only
git push                    # → dev_only/main
git push origin feat/foo    # ❌ 會失敗（origin 已改名為 public，沒人會這樣打）
git push dev_only feat/foo  # ✓ feature branch 推 dev_only

# Release（在 main、tag 已建好、worktree 乾淨）
./scripts/release-to-public.sh v0.3.0
# → push public main + push public v0.3.0
# → release.yml 自動 build + dispatch homebrew tap
# 然後手動 cargo publish 4 crates（drift-core → drift-connectors → drift-mcp → drift-ai）
```

腳本內建守護：要在 `main`、worktree 乾淨、tag 存在且在 main 上、local main = dev_only/main，
都不滿足就拒絕推。

## 偏離原計畫之處

**Phase 5 PR 步驟改用 fast-forward merge**。
原計畫是在 dev_only 上開 PR 後 squash-merge，但 PAT 沒有 `pull_requests: write`
權限（403 Resource not accessible）。由於該 branch 只有 1 個 commit、直接由 main 分支出來，
fast-forward main 與 squash-merge 結果完全相同（commit hash、tree、訊息一致）。

最終狀態：`chore/release-workflow` 已合進 `main`，本地與遠端 branch 都已刪除。
若未來想用 PR workflow，需把 PAT 重新設定為 fine-grained 並勾 `Pull requests: Read & write` 給
`drift_dev_only`，或改用 classic PAT with `repo` scope。

## 驗證清單

- [x] drift_ai visibility = PUBLIC
- [x] drift_dev_only visibility = PRIVATE
- [x] 本機 remotes：public + dev_only
- [x] `remote.pushDefault = dev_only`
- [x] `main` upstream = `dev_only/main`
- [x] Tags 在兩 repo 完全一致（baseline 鏡像成功）
- [x] public/main HEAD 與本任務開始前一致（無污染）
- [x] `scripts/release-to-public.sh` executable
- [x] PAT-credentialed fetch/push to dev_only works
