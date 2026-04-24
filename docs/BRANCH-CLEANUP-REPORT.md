# Branch cleanup · post-v0.1.1

**日期**：2026-04-24
**範圍**：drift_ai repo 累積 4 個 branch + 1 個 open draft PR；全部盤點、處置、清乾淨，只留 `main` 與兩個 tag / release。

---

## 處置前盤點

| Branch | 位置 | ahead of main | behind main | Tip hash | 內容狀態 | 結論 |
|---|---|---|---|---|---|---|
| `main` | local+remote | 0 | 0 | `1e713e2` | — | **保留** |
| `phase0-proposal` | local+remote | 6 | 10 | `539a35e` | 內容已 squash-merged 進 main 的 `ad2a505` (v0.1.0) | **刪** |
| `phase1-through-5` | local+remote | 10 | 10 | `3a53b97` | 內容已 squash-merged 進 main 的 `ad2a505` (v0.1.0) | **刪** |
| `v0.1.1` | local+remote | 6 | 4 | `59a675e` | 內容已 squash-merged 進 main 的 `b2c7d0b` (v0.1.1) | **刪** |

> **注意 — 關於 `git merge-base --is-ancestor`**：三個 branch 的 tip 都**不是** main 的祖先（因為 main 是 squash-merge 重建的，lineage 斷掉），`git cherry` 會把這 6 / 10 / 6 個 commit 標 `+`（patch-id 不匹配）。**但內容等價已在 main 裡**（可從 `ad2a505` 的 stat 26 files 與 `b2c7d0b` 的 stat 23 files 驗證）。因此直接刪除、不再重複 merge。

## Open PR 盤點

| # | Title | Head → Base | 狀態 | 處置 |
|---|---|---|---|---|
| 1 | Phase 0 (rev 2): proposal + line-level attribution data model | `phase0-proposal` → `main` | DRAFT / OPEN | **close**（內容已 squash-merged）|

---

## 執行結果

### PR close

```
PR #1  →  state=CLOSED, closedAt=2026-04-24T11:42:17Z
```

*附帶觀察*：`gh pr close 1 --comment "..."` 指令回報 `GraphQL: Resource not accessible by personal access token (addComment)` — PAT 有 pull-request write 但沒 issues write（GitHub 把 PR comments 當 issues comments 管）。**close 動作本身成功**，只是那條 comment 沒貼上去。不影響結果。

### Remote branches

```
DELETE /repos/ShellFans-Kirin/drift_ai/git/refs/heads/phase0-proposal    → 204 ✓
DELETE /repos/ShellFans-Kirin/drift_ai/git/refs/heads/phase1-through-5   → 204 ✓
DELETE /repos/ShellFans-Kirin/drift_ai/git/refs/heads/v0.1.1             → 204 ✓
```

### Local branches

```
Deleted branch phase0-proposal   (was 539a35e)
Deleted branch phase1-through-5  (was 3a53b97)
Deleted branch v0.1.1            (was 59a675e)
```

### `git fetch --all --prune`

```
- [deleted]  (none)  -> origin/phase0-proposal
- [deleted]  (none)  -> origin/phase1-through-5
- [deleted]  (none)  -> origin/v0.1.1
```

---

## 處置後驗證

```
remote branches (gh api ...):
  main

tags (gh api ...):
  v0.1.1
  v0.1.0

releases (gh release list):
  v0.1.1   Latest   v0.1.1    2026-04-24T03:32:53Z
  drift_ai v0.1.0 — AI-native blame (Rust + MCP + Claude Code + Codex)
                            v0.1.0    2026-04-22T15:23:54Z

local:
  * main
    remotes/origin/main

open PRs:
  (empty)
```

**結論**：repo 只剩 `main`，兩個 tag 與 release 完整保留，無任何 orphan branch 或 open PR。

---

## 一個看起來像「丟東西」但其實沒有的小事

盤點時 `git rev-parse v0.1.1` 回的是 `be751f36...`，跟實際 branch tip `59a675e...` 不同，讓人擔心有未 push 的 local commit。

真相：`be751f3` 是**annotated tag object** `v0.1.1` 的 SHA（annotated tag 本身是獨立 git object，有自己的 SHA），它 dereference 到的 commit 是 `d86405c ci(release): run x86_64-apple-darwin on macos-14 too`，這個 commit **已經在 main 上**。

```
$ git cat-file -t be751f360c28aad56a59ef8bee47bf1af410e20c
tag
$ git cat-file -p be751f360c28aad56a59ef8bee47bf1af410e20c
object d86405ce57ba8277cbbf75e3f1048a9f9b904494
type commit
tag v0.1.1
tagger ShellFans-Kirin <kirin@shell.fans> 1777001394 +0000
drift_ai v0.1.1 — Launch-ready patch release
...
```

無資料遺失，純粹是 `rev-parse` 解析 ambiguous reference（同名 branch + annotated tag）時優先回 tag SHA 的行為。

---

## 取消路徑（萬一需要回復某個 branch）

所有刪除動作都可還原：

- **遠端**：GitHub 90 天內保留刪除的 branch ref；API `gh api /repos/.../git/refs -f ref=refs/heads/<name> -f sha=<old_tip>` 可重建
- **本機**：`git reflog` 有完整紀錄，直到 gc（預設 90 天）
- **Tip hashes（給 forensic 用）**：
  - `phase0-proposal` → `539a35e450ca29fece743c18d9a925eb78db8bf8`
  - `phase1-through-5` → `3a53b976073bd4629f35b9574a086e8b586453f9`
  - `v0.1.1` → `59a675e9fe484312f699818e3db5ef737706370e`
