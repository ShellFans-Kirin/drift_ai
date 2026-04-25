# Release Workflow

drift 用兩個 repo 維護：

| Repo | 性質 | 用途 |
|---|---|---|
| github.com/ShellFans-Kirin/drift_ai | public | 對外 release-facing 鏡像，使用者裝 / 看 |
| github.com/ShellFans-Kirin/drift_dev_only | private | 日常開發、實驗、debug、半成品 |

本機 remote：
- public  → drift_ai
- dev_only → drift_dev_only（預設推送目標）

## 日常開發

```
git push                    # 預設推 dev_only（remote.pushDefault 設好了）
git pull                    # main 追蹤 dev_only/main
gh pr create                # 在 dev_only 開 PR
```

WIP branches、debug commit、broken tests、實驗性工作都 OK，反正不對外。

## Release 流程

1. 在 dev_only 的 feature branch 開發完成
2. squash-merge 到 dev_only 的 main
3. main 上 bump version、commit、tag：
   ```
   git tag -a v0.X.Y -m "..."
   ```
4. 先推 tag 到 dev_only 驗 CI（如果有）：
   ```
   git push dev_only v0.X.Y
   ```
5. dev_only CI 綠後，推 public：
   ```
   ./scripts/release-to-public.sh v0.X.Y
   ```
6. release.yml 會自動：
   - build 4 target binaries
   - 建 GitHub Release
   - dispatch Homebrew tap update
7. release.yml 跑完後手動 cargo publish 4 crates：
   ```
   cargo publish -p drift-core
   sleep 45
   cargo publish -p drift-connectors
   sleep 45
   cargo publish -p drift-mcp
   sleep 45
   cargo publish -p drift-ai
   ```

## 哪些東西只留在 dev_only

- WIP commits / 實驗 branches
- docs/internal/* 全部
- 半成品 docs（草稿、TODO、思考筆記）
- debug 用的 scripts
- 沒 ready 的 connector / 功能

## 哪些東西進 public drift_ai

- main 上的 squashed release commit
- release tag (vX.Y.Z)
- 不會推其他 branch
- 不會推 docs/internal/

`release-to-public.sh` 只 push main + 指定 tag，不會誤推其他內容。但保險作法是：
**internal docs 不要 commit 到 main branch，改 commit 到 dev_only 的 internal-only branch**。
這樣連 main 的歷史都不會包含 internal 內容。

## 未來如果想公開 dev_only 某些東西

- internal docs 可以「升級」：把 `docs/internal/X.md` mv 到 `docs/X.md` 後 commit 進 main
- 下次 release 時就會自然推到 public
