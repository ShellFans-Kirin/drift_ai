> 🌐 **日本語** · [简体中文](../zh-Hans/USAGE.md) · [繁體中文](../../USAGE.md)

# Drift AI · 使い方（v0.2.0+）

完全な利用ガイド。ゼロからのインストールから `drift handoff` まで、各ステップにコピペできるコマンドと出力例を添えています。

---

## 0. 前提条件

- macOS（Apple Silicon または Intel）/ Linux（x86_64 または aarch64）
- 1 つの git repo（drift が操作する対象）
- 少なくとも 1 つの AI コーディング agent の session log:
  - Claude Code → デフォルトで `~/.claude/projects/` に書き込み
  - Codex → デフォルトで `~/.codex/sessions/` に書き込み
  - Aider → connector は stub、コミュニティ PR を歓迎します
- LLM compaction / handoff を使う場合は `ANTHROPIC_API_KEY` 環境変数。設定がない場合は drift が deterministic な mock summary（`[MOCK]` ラベル付き）にフォールバックします。

---

## 1. インストール

### 1.1 Homebrew（macOS arm64/x86_64 + Linuxbrew）

```bash
brew install ShellFans-Kirin/drift/drift
```

### 1.2 crates.io（Rust 1.85+ が必要）

```bash
cargo install drift-ai
```

注意：crate 名は `drift-ai`、インストール後の binary 名は `drift` です。

### 1.3 GitHub Releases pre-built tarball（Rust toolchain 不要）

```bash
curl -sSfL https://github.com/ShellFans-Kirin/drift_ai/releases/latest/download/drift-v0.2.0-$(uname -m)-unknown-linux-gnu.tar.gz \
  | tar xz -C /tmp && sudo mv /tmp/drift /usr/local/bin/drift
drift --version
# expect: drift 0.2.0
```

macOS の場合は `unknown-linux-gnu` を `apple-darwin` に置き換えてください。

### 1.4 ソースから

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo install --path crates/drift-cli
```

### 確認

```bash
drift --version          # drift 0.2.0
drift --help             # 17 個すべての subcommand を表示
```

---

## 2. 初回セットアップ

### 2.1 git repo 内で init

```bash
cd /path/to/your/repo
drift init
```

これで以下が作成されます：

- `.prompts/` ディレクトリ
- `.prompts/config.toml`（デフォルト値、後で編集可能）
- `.prompts/.gitignore`（cache を git に入れない）

`drift init` は idempotent — 再実行しても既に編集した `config.toml` を上書きしません。

### 2.2 初回の `drift capture` はプライバシー通知を表示

これは v0.1.2 で追加された first-run notice です：

```
drift capture · first-run notice
  drift mirrors your AI session content (including anything you
  pasted) into .prompts/. events.db is committed to git by default.
  See docs/SECURITY.md for the full story.

  Press Enter to continue, Ctrl-C to abort.
```

Enter で続行。`~/.config/drift/state.toml` が「表示済み」を記憶するので、次回からは聞かれません。

CI / 自動化シナリオでは `DRIFT_SKIP_FIRST_RUN=1` 環境変数でスキップします（**ただし**「表示済み」とはマークされません — 次に対話的に実行するとまた聞かれます）。

---

## 3. session の取り込み（capture）

### 3.1 一回限り

```bash
drift capture
# 期待: Captured N session(s), wrote M event(s) to .prompts/events.db
```

`drift capture` の動作：

1. `~/.claude/projects/` + `~/.codex/sessions/` 配下のすべての jsonl をスキャン
2. 各 session から `CodeEvent` を抽出して `.prompts/events.db`（SQLite）に書き込み
3. 各 session に LLM compaction をかけて `.prompts/sessions/<date>-<agent>-<short_id>.md` として書き出し

### 3.2 Filter

```bash
drift capture --agent claude-code              # Claude Code のみ
drift capture --agent codex                    # Codex のみ
drift capture --session abc12345-xxx           # 特定の session id のみ
drift capture --all-since 2026-04-22T00:00:00Z  # 特定時刻以降のみ
```

### 3.3 Live mode（バックグラウンド daemon）

```bash
drift watch
# drift watch · event-driven; Ctrl-C to stop
#   watching /home/you/.claude/projects
#   watching /home/you/.codex/sessions
```

`drift watch` は platform ネイティブの FS event（macOS の FSEvents / Linux の inotify）を使い、session ファイルが変更されるたびに 200ms の debounce ウィンドウ内で再 capture します。`Ctrl-C` は走行中の capture を完了してからきれいに終了します。状態は `~/.config/drift/watch-state.toml` に保存されるので、再起動時は再スキャンではなく resume します。

---

## 4. 🌟 v0.2 の新機能：`drift handoff`

進行中のタスクを、別の agent が引き継げる markdown brief にパッケージします。

### 4.1 使い方 — 3 つの scope（いずれか）

```bash
# git branch でスコープを指定（最もよく使う）
drift handoff --branch feature/oauth --to claude-code

# 時刻範囲（専用 branch がない場合）
drift handoff --since 2026-04-25T08:00:00Z --to codex

# 単一 session（debug / unit test）
drift handoff --session abc12345-xxx --print
```

### 4.2 ターゲット agent

```bash
--to claude-code     # デフォルト；footer に 'paste this to claude' のヒント
--to codex           # footer は codex 流の言い回し
--to generic         # footer なしの純粋な brief、任意のツールにパイプできます
```

違いは footer のみ。Body は同一です — body はタスクの内容で、現状ベンダー別の翻訳は意図的にしていません（v0.3+ で tool-call schema adapter を入れる予定）。

### 4.3 Output

```bash
drift handoff --branch feat-x --to claude-code
# → .prompts/handoffs/2026-04-25-1530-feat-x-to-claude-code.md

drift handoff --branch feat-x --to claude-code --output ~/transfer.md
# → ~/transfer.md

drift handoff --branch feat-x --to claude-code --print | pbcopy
# → stdout、それを clipboard にパイプ
```

### 4.4 期待される実行フロー

```
$ drift handoff --branch feature/oauth --to claude-code
⚡ scanning .prompts/events.db
⚡ extracting file snippets and rejected approaches
⚡ compacting brief via claude-opus-4-7
✅ written to .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md
  · model=claude-opus-4-7 · in=3421 out=612 · cost=$0.0972

next:
  claude
  # then paste:
  "I'm continuing this task. Read the handoff brief and resume from 'Next steps' #1:"
  "$(cat .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md)"
```

### 4.5 Brief の構造

すべての brief は以下のセクションを持ちます：

```markdown
# Handoff Brief — `feature/oauth`

| Field | Value |
|---|---|
| From | codex × 2 + claude-code × 2 (4 sessions, 47 turns) |
| To | claude-code |
| Generated | 2026-04-25 15:30 UTC |
| Repo | owner/repo @ feature/oauth |
| Branch dif | +156 / -23 across 3 files |

## What I'm working on
[3-5 sentences high-level intent — LLM-compacted]

## Progress so far
- ✅ Done items
- ⏳ In-progress
- ⏸ Not started

## Files in scope
### `src/auth/login.ts` (modified, +47 / -3)
```
[code excerpt with modified ranges + ±5 lines context]
```

## Key decisions made
- Decision text *(citation: codex 7c2…, turn 4)*

## Approaches tried but rejected
- Pre-extracted from session tool_result errors

## Open questions / blockers
1. ...

## Next steps (suggested)
1. ...

## How to continue (paste this to claude-code)
> [paste-friendly resume prompt]
```

### 4.6 コスト

| Model | handoff 1 件あたり概算 |
|---|---|
| `claude-opus-4-7`（デフォルト） | ~\$0.10–0.30 |
| `claude-sonnet-4-6` | ~\$0.02–0.06 |
| `claude-haiku-4-5` | ~\$0.005–0.01 |

切り替えは `.prompts/config.toml` で：

```toml
[handoff]
model = "claude-haiku-4-5"
```

デフォルトが Opus なのは、brief は次の agent が逐語的に読むものであり、per-session compaction より叙述の質が重要だからです。1 日に handoff が数回程度なら Opus でも月 ~\$30。高頻度なら Haiku への切り替えを推奨します。

---

## 5. AI ネイティブな blame（v0.1 既存機能）

### 5.1 逆引き：どの session がこの行を書いたか？

```bash
drift blame src/auth/login.ts
# ファイル全体のタイムライン

drift blame src/auth/login.ts --line 42
# 単一行のタイムライン

drift blame src/auth/login.ts --range 40-60
# 行範囲のタイムライン
```

出力例：

```
src/auth/login.ts
├─ 2026-04-15 14:03  💭 [claude-code] session abc12345
│   --- a/src/auth/login.ts
│   +++ b/src/auth/login.ts
│   @@
│   +if (attempts > 5) throw new RateLimitError()
├─ 2026-04-15 15:20  ✋ [human]       post-commit manual edit
│   -  if (attempts > 5)
│   +  if (attempts > MAX_ATTEMPTS)
└─ 2026-04-16 09:12  💭 [codex]       session def45678
    +const MAX_ATTEMPTS = 5
```

### 5.2 順引き：この session は何を変更したか？

```bash
drift trace abc12345-xxx
# この session が生成したすべての CodeEvent を表示
```

### 5.3 全体の audit log

```bash
drift log
# git log のようなもの、ただし commit ごとに per-agent attribution セクション付き

drift log -- --since 1.day
# 後ろの引数を git log にパススルー
```

例：

```
commit abc1234 — Add OAuth login
   💭 [claude-code] 7 events accepted, 0 rejected
   💭 [codex]       3 events accepted, 1 rejected
   ✋ [human]       2 manual edits
```

### 5.4 単一 event を見る

```bash
drift diff <event-id>      # この event の unified diff を表示
drift show <session-id>    # session の compacted markdown を表示
drift list                 # captured 済みのすべての session を一覧
drift list --agent codex   # codex の session のみ一覧
```

### 5.5 却下された AI 提案を見る

```bash
drift rejected
drift rejected --since 2026-04-22T00:00:00Z
```

`rejected` event のソース：session の中で tool_result が `is_error=true` でマークされたもの。

### 5.6 session を git commit に紐づけ

```bash
drift bind <commit-sha> <session-id>     # 手動で紐づけ
drift auto-bind                           # timestamp で自動ペアリング
drift install-hook                        # post-commit hook をインストールして auto-bind を自動実行
```

紐づけデータは `refs/notes/drift`（git notes）に書かれ、commit history を汚しません。

---

## 6. MCP server（他の AI ツールが drift を照会するため）

```bash
drift mcp
# stdio MCP server を起動
# デフォルトのツール: drift_blame / drift_trace / drift_rejected / drift_log / drift_show_event
```

Claude Code に登録（1 行）：

```bash
claude mcp add drift -- drift mcp
```

Codex に登録：

```bash
codex mcp add drift -- drift mcp
```

これで Claude / Codex の対話の中で「show me the drift blame for src/foo.rs:42」のように直接質問でき、彼らは MCP 経由で drift を呼び出して attribution を取得します。shell に切り替える必要はありません。

MCP インターフェースは設計上 **read-only** です — 書き込み系アクション（capture / bind / sync）は CLI 専用です。

---

## 7. 課金透明性：drift cost

`drift handoff` と `drift capture` の LLM 呼び出しはすべて `events.db` の `compaction_calls` テーブルに記録されます：

```bash
drift cost
# drift cost — compaction billing
#   total calls      : 10
#   input tokens     : 120958
#   output tokens    : 6582
#   total cost (USD) : $0.1539
```

詳細なグルーピング：

```bash
drift cost --by model
drift cost --by session
drift cost --by date
drift cost --since 2026-04-20T00:00:00Z --until 2026-04-25T00:00:00Z
drift cost --model claude-haiku-4-5
```

---

## 8. 設定（`.prompts/config.toml`）

完全なテンプレート：

```toml
[attribution]
db_in_git = true             # デフォルト true、events.db を git に入れる。false でローカル限定。

[connectors]
claude_code = true
codex = true
aider = false                # feature-flag の stub

[compaction]
provider = "anthropic"       # デフォルト；"mock" に変えれば完全オフライン
model = "claude-haiku-4-5"   # または claude-sonnet-4-6 / claude-opus-4-7

[handoff]
model = "claude-opus-4-7"    # 叙述の質が重要。30× コスト削減なら haiku に切り替え

[sync]
notes_remote = "origin"      # `drift sync push/pull` で使う remote
```

2 階層：

- グローバル：`~/.config/drift/config.toml`
- プロジェクト（上書き）：`<repo>/.prompts/config.toml`

```bash
drift config get handoff.model
drift config set handoff.model claude-haiku-4-5
drift config list
```

---

## 9. マシン間同期（git notes）

```bash
drift sync push origin   # refs/notes/drift を remote に push
drift sync pull origin   # remote から refs/notes/drift を pull
```

drift の attribution リンク（どの session がどの commit に対応するか）は `refs/notes/drift` に保存されます。Push / pull は git 標準の notes メカニズムを使うので、main code の commit には混入しません。

---

## 10. セキュリティ / Privacy

drift は session 内容を **scrub しません**。Claude / Codex の chat に貼り付けた内容（うっかり貼った secret を含む）は `.prompts/sessions/*.md` と `events.db` に入り、デフォルトでは git に commit されます。

3 つの mitigations：

1. **git 側を無効化**：
   ```toml
   [attribution]
   db_in_git = false
   ```
   `events.db` と markdown はローカルに留まります。team は共有 blame を失いますが、あなたのローカルビューは保たれます。

2. **commit 前に手動 review**：
   ```bash
   drift capture
   git diff --cached -- .prompts/
   git add .prompts/ && git commit
   ```

3. **secret scanner を pre-commit hook と組み合わせる**（drift には同梱なし）：
   ```bash
   # .git/hooks/pre-commit
   gitleaks protect --staged --redact -v || exit 1
   ```

完全な threat model + roadmap は [`docs/SECURITY.md`](../../SECURITY.md) を参照してください。

---

## 11. 上級者向け：新しい connector の追加

drift は現在 Claude Code + Codex をサポートしています。Cursor / Cline / 自作 agent を追加したい？

`SessionConnector` trait を実装してください（`crates/drift-connectors/src/lib.rs`）：

```rust
pub trait SessionConnector {
    fn agent_slug(&self) -> &'static str;
    fn discover(&self) -> Result<Vec<SessionRef>>;
    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession>;
    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>>;
}
```

`crates/drift-connectors/src/aider.rs` は stub で、[`CONTRIBUTING.md`](../../../CONTRIBUTING.md) は Aider を worked example として「コネクタ追加」のフロー全体をウォークスルーしています。PR 歓迎です。

---

## 12. コマンド一覧

| Command | 用途 |
|---|---|
| `drift init` | `.prompts/` を scaffold |
| `drift capture [--agent A] [--session ID] [--all-since DATE]` | session 取り込み + compaction |
| `drift watch` | バックグラウンド daemon、event-driven な自動 capture |
| `drift handoff [--branch B \| --since ISO \| --session ID] [--to A] [--output P \| --print]` | **v0.2** クロス agent task brief |
| `drift list [--agent A]` | captured 済み session を一覧 |
| `drift show <id>` | compacted session を表示 |
| `drift blame <file> [--line N \| --range A-B]` | 逆引き：どの session がこの行を書いたか |
| `drift trace <session>` | 順引き：この session は何を変更したか |
| `drift diff <event>` | 単一 event の unified diff |
| `drift rejected [--since DATE]` | 却下された AI 提案を一覧 |
| `drift log [-- <git args>]` | git log + per-agent attribution |
| `drift cost [--since --until --model --by]` | 課金 |
| `drift bind <commit> <session>` | commit ↔ session を手動で紐づけ |
| `drift auto-bind` | commit ↔ session を timestamp で自動ペアリング |
| `drift install-hook` | post-commit hook を設置して auto-bind を自動実行 |
| `drift sync push\|pull <remote>` | `refs/notes/drift` を同期 |
| `drift config get\|set\|list` | config の読み書き |
| `drift mcp` | stdio MCP server を起動 |

---

## 13. Troubleshooting

### "drift handoff: no sessions matched scope"

`--branch` scope で sessions が拾えない一般的な原因：

1. まだ `drift capture` していない — 一度実行してください
2. branch 名が間違っている — git branch が `feat/x` ではなく `feature/x` ではないか確認
3. その branch にまだ commit がない — `--branch` は git log を使って分岐 commit timestamp を下限とし、空 branch の場合は 14 日にフォールバックします

代替として `--since 2026-04-22T00:00:00Z` を試してください。

### "ANTHROPIC_API_KEY not set — falling back to deterministic mock summary"

handoff は API key が未設定なら MockProvider を実行し、brief には明確に `[MOCK]` ラベルが付きます。env var を設定するか、`.prompts/config.toml` で `provider = "mock"` を `"anthropic"` に戻してください。

### `drift watch` が発火しない

- agent が本当に `~/.claude/projects/` または `~/.codex/sessions/` に jsonl を書き出しているか確認
- macOS: `Sandbox / Full Disk Access` の権限が FSEvents をブロックしている可能性 — terminal app にフルディスクアクセスを付与
- Linux: `/proc/sys/fs/inotify/max_user_watches` が低すぎる可能性 — `sudo sysctl fs.inotify.max_user_watches=524288`

### crates.io ダウンロード失敗

- `crates.io` アカウントの email 検証が済んでいるか確認
- `cargo install drift-ai`（ハイフン）であって `drift_ai`（アンダースコア）ではないことを確認
- `/tmp` のクリーン環境で試す：
  ```bash
  CARGO_HOME=/tmp/drift-clean cargo install drift-ai --locked
  /tmp/drift-clean/bin/drift --version
  ```

### Homebrew install で formula が見つからない

- `brew tap` しているのが `ShellFans-Kirin/drift` であることを確認（大文字小文字の感応性は OS / brew バージョンによる — 両方試して）
- `brew update` で最新 tap state を同期
- macOS Intel runner は停止済み — Intel Mac でもインストール可能（Formula に x86_64-apple-darwin tarball あり）

---

## 14. 関連ドキュメント

| ドキュメント | 内容 |
|---|---|
| [README](../../../README.md) | 30 秒で第一印象 |
| [CHANGELOG](../../../CHANGELOG.md) | バージョン履歴 |
| [docs/VISION.md](VISION.md) | プロジェクト全体の north star |
| [docs/SECURITY.md](SECURITY.md) | Threat model 完全版 |
| [docs/COMPARISON.md](COMPARISON.md) | vs Cursor / Copilot chat / Cody / git blame |
| [docs/V020-DESIGN.md](../../V020-DESIGN.md) | v0.2 `drift handoff` 設計提案 |
| [docs/V020-DEV-LOG.md](../../V020-DEV-LOG.md) | v0.2 開発サイクル + 実行結果完全記録 |
| [CONTRIBUTING.md](../../../CONTRIBUTING.md) | 新コネクタ追加の手順ガイド |
