> 🌐 [English](README.md) · **日本語** · [简体中文](README.zh-Hans.md) · [繁體中文](README.zh-Hant.md)

# drift_ai

[![crates.io](https://img.shields.io/crates/v/drift-ai.svg)](https://crates.io/crates/drift-ai)
[![CI](https://github.com/ShellFans-Kirin/drift_ai/actions/workflows/ci.yml/badge.svg)](https://github.com/ShellFans-Kirin/drift_ai/actions/workflows/ci.yml)

> 進行中の AI コーディングタスクを Claude、Codex、その次に切り替える agent
> の間でスムーズに handoff する。Local-first。

![drift handoff demo](docs/demo/v020-handoff.gif)

**問題**：AI コーディング agent が止まりました — 拒否、rate limit、ある
いは突然賢さがなくなった。これから 30 分ぶんの文脈を別の agent に渡さなけ
ればなりません。チャット履歴を貼り直しても役に立ちません。新しい agent
は、どの判断が確定済みなのか、どのアプローチをすでに試して却下したのか、
今どのファイルのどの行を書きかけだったのかを知りません。

**`drift handoff`** は進行中のタスクを、どの LLM でも初見で読める markdown
brief にまとめます：

```bash
$ drift handoff --branch feature/oauth --to claude-code
⚡ scanning .prompts/events.db
⚡ extracting file snippets and rejected approaches
⚡ compacting brief via claude-opus-4-7
✅ written to .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md
```

Brief には決まったこと、試して却下したこと、未解決の点、続きをどこから始
めるかが並びます。次の agent に貼り付けるだけで、説明し直すことなくその
ままタスクの途中から再開できます。

`drift` は v0.1 の attribution engine を土台にしています。あの層は AI
コーディング agent（Claude Code、Codex、Aider…）のローカル session log を
監視し、各 session を LLM で compact し、結果を git repo 内の `.prompts/`
に保存し、`git notes` を通じて各 session を対応する commit に紐づけます。
Handoff は v0.2 で追加された wedge feature であり、attribution engine は
依然として（引き続きサポートされる）`drift blame` / `drift log` 側を支
えています。

インストール後、`drift log` は引き続き各 commit の multi-agent attribution
を表示します：

```
commit abc1234 — Add OAuth login
   💭 [claude-code] 7 events accepted, 0 rejected
   💭 [codex]       3 events accepted, 1 rejected
   ✋ [human]       2 manual edits
```

…そして `drift blame` は引き続き任意の行をその全タイムラインまで遡れます。
プロジェクトの thesis は [`docs/VISION.md`](docs/VISION.md) を参照してくだ
さい。

## なぜ drift が必要なのか

AI コーディングはもはや単一 agent のワークフローではありません。今日の現実
的な開発 session はこんな感じです:

- Claude Code で feature に取り掛かったが、rate limit に当たる、context
  window を使い切る、あるいは LLM が急に **バカになった** と感じて、途中で
  作業を諦めざるを得ない。
- Codex(または Aider、別の model)に切り替えるが、新しい agent はあなた
  がすでに試した方法、固まった判断、*意図的に* 却下したアプローチを知らな
  い。
- chat の履歴を貼り付けると、ノイズが多く、agent は決着済みの議論を蒸し
  返し、あなたは前に進む代わりに 10 分かけてやりたいことを再説明する羽
  目になる。
- 1 週間後に commit を review しても、どの行がどの agent から来たのか、
  どこが AI 提案の上に乗った人間の編集なのか、なぜ code が最終的にこの形
  になったのかが分からない。
- 同僚が repo を clone すると code は見えるが、その code を生んだ推論プロ
  セスは消えている — その履歴は誰かの Claude / Codex の chat に、誰かのラ
  ップトップ上に存在し、実質的に失われている。

`drift` はこの捨てられがちな AI の軌跡を、長期保存可能な project memory に
変えます:

- **ローカルで capture**:`drift capture`(および live mode 用の `drift watch`)
  は、agent がもともと `~/.claude/projects/` や `~/.codex/sessions/` に書き
  出している session JSONL を読みます。あなたがオフにできる任意の Anthropic
  compaction 呼び出しを除き、データはマシンの外には出ません。
- **markdown に圧縮**:各 session は `.prompts/sessions/` 配下の小さな
  markdown 要約になります — 残った判断、却下された方法、触れたファイル。
  読むのも grep するのも軽く、ベンダー移行でも失われません。repo の中の
  ただのテキストだからです。
- **commit に紐づけ**:`drift bind` / `drift auto-bind` が `git notes`
  (`refs/notes/drift`)経由で、各 session をそれが生んだ commit に結び付け
  ます。リンクは repo について移動し、commit history を汚しません。
- **agent を切り替えるときの handoff**:`drift handoff --branch <b> --to
  <agent>` は次の agent が初見で読み込める brief を作成します — 完了したこ
  と、未解決のこと、却下済みのこと、どこから再開するか。
- **忘れたときの逆引き**:`drift blame <file> [--line N]` は、その行の背後
  にある完全なタイムラインを返します:どの session、どの prompt、どの agent、
  そして上に乗った人間の編集。
- **session は覚えているが diff を覚えていないときの順引き**:
  `drift trace <session-id>` は、その session が生成したすべての `CodeEvent`
  を列挙します。
- **release をまたいだ audit**:`drift log` は `git log` で、各 commit の下
  に per-agent サマリが付くもの — 「この release のうち、どれだけが AI で
  どれだけが人間か」を LOC 比率の推測に頼らずに答える必要があるときに役に
  立ちます。

正味の効果:multi-agent な AI コーディングは、handoff でき、review でき、
数ヶ月後に再構成できるものになります — 次に tab を閉じたら消える chat
history ではなく。

## インストール

**Homebrew**（macOS arm64/x86_64、Linux arm64/x86_64）：

```bash
brew install ShellFans-Kirin/drift/drift
```

**crates.io**（Rust 1.85+ toolchain が必要）：

```bash
cargo install drift-ai
```

**ビルド済みバイナリ**（GitHub Releases）：

```bash
curl -sSfL https://github.com/ShellFans-Kirin/drift_ai/releases/latest/download/drift-v0.2.0-$(uname -m)-unknown-linux-gnu.tar.gz \
  | tar xz -C /tmp && sudo mv /tmp/drift /usr/local/bin/drift
drift --version
```

**ソースから**：

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo install --path crates/drift-cli
```

## プライバシーと secrets

`drift` は session の内容を **scrub しません**。Claude Code / Codex の
session に打ち込んだ内容 — うっかり貼ってしまった secret も含めて — は
そのまま `.prompts/` にミラーされ、デフォルトでは repo に commit されま
す。

現状で提供している調整は 2 つあります：

1. `.prompts/config.toml` に `[attribution].db_in_git = false` を設定し、
   `events.db` をローカル限定にする。
2. `git add` する前に `.prompts/sessions/` を確認する。

`v0.2` で regex ベースの redaction pass が追加されます。今すぐ完全にカバー
したい場合は、`drift` を [gitleaks](https://github.com/gitleaks/gitleaks)
や [trufflehog](https://github.com/trufflesecurity/trufflehog) と組み合わ
せて pre-commit hook として使ってください。

> **AI session に日常的に secret を貼り付けているなら、共有 repo で
> `drift` を有効にするのは `v0.2` まで待ったほうが安全です。**

`drift capture` を初回実行したときに上記を再掲する一回限りの notice が表
示されるので、Enter で承認してください。CI では `DRIFT_SKIP_FIRST_RUN=1`
を立てて抑制できます。

完全な threat model と roadmap は [`docs/SECURITY.md`](docs/SECURITY.md)
にあります。

## クイックスタート

6 つのコマンド、設定ゼロ：

```bash
cd your-git-repo
drift init                                          # scaffold .prompts/
drift capture                                       # pull sessions from ~/.claude + ~/.codex
drift handoff --branch feature/oauth --to claude   # NEW in v0.2 — task transfer
drift blame src/foo.rs                              # 逆引き：誰がこの行を書いたか
drift trace <session-id>                            # 順引き：session → events
drift install-hook                                  # commit ごとに自動実行
```

`drift handoff` は v0.2 の目玉機能です：進行中のタスクを次の agent が初見
で読める brief にまとめます。完全なフローは [§Handoff](#handoff--cross-agent-task-transfer-v02)
を参照してください。

`/tmp` のゼロステートで検証済み：

```bash
rm -rf /tmp/drift-smoke && mkdir -p /tmp/drift-smoke && cd /tmp/drift-smoke
git init -q && git config user.email ""x@y"" && git config user.name x
drift init && drift capture && drift list
```

## Handoff — cross-agent task transfer (v0.2)

`drift handoff` はローカルの `events.db`（`drift capture` または `drift watch`
で蓄積されたもの）を読み、指定した scope に該当する session に絞り込み、
handoff 用に構成された markdown brief を生成します：

- **What I'm working on** — 3〜5 文の意図（LLM compacted）。
- **Progress so far** — done / in-progress / not-started の箇条書き。
- **Files in scope** — 修正範囲 ±5 行の context 付き。
- **Key decisions** — session+turn の引用付き。
- **Rejected approaches** — session の tool error から抽出。
- **Open questions / blockers**。
- **Next steps**。
- **How to continue** — そのまま次の agent に貼れる prompt。

```bash
# branch で scope を指定（推奨）：この branch（main から分岐して以降）に
# 落ちた commit に対応するすべての session
drift handoff --branch feature/oauth --to claude-code

# 時間で scope を指定
drift handoff --since 2026-04-25T08:00:00Z --to codex

# 単一 session（debug 用）
drift handoff --session abc12345-xxx --print

# clipboard やほかのツールにパイプ
drift handoff --branch feature/oauth --print | pbcopy
```

デフォルトの model は `claude-opus-4-7` です — brief は次の agent がその
まま読むものなので、v0.1 の per-session compaction より叙述の質が重要で
す。1 回の handoff で Opus レートでは ≈ \$0.10–0.30 USD ほどかかります。
叙述を犠牲にして ~30× コストダウンしたい場合は、`.prompts/config.toml`
で Haiku に切り替えてください：

```toml
[handoff]
model = "claude-haiku-4-5"   # デフォルトは "claude-opus-4-7"
```

## Live mode — イベント駆動 watcher

`drift watch` は platform ネイティブのファイルシステム通知（macOS の
FSEvents、Linux の inotify、Windows の ReadDirectoryChangesW）でバックエ
ンドされたイベント駆動 daemon です。同じ session ファイルへの連続書き込
みを 200ms ウィンドウで合体させるので、長時間動く Claude Code や Codex
の session でも、tool call ごとではなくアイドルタイミングごとに 1 回ずつ
capture が走ります。状態は `~/.config/drift/watch-state.toml` に永続化さ
れるので、再起動後は再スキャンではなく resume です。`Ctrl-C` は走行中の
capture を完了させてからきれいに終了します。

```bash
drift watch
# drift watch · event-driven; Ctrl-C to stop
#   watching /home/you/.claude/projects
#   watching /home/you/.codex/sessions
#   first run; capturing every session seen
# drift capture · provider=anthropic
# Captured 10 session(s), wrote 192 event(s) to .prompts/events.db
# ...
# drift watch · interrupt received; exiting after last capture
```

## コスト透明性

すべての Anthropic compaction 呼び出しは `events.db` の
`compaction_calls` に input / output / cache token カウントと算出済みの
USD コストとともに記録されます（model ごとの料金表は組み込み済み。
請求確認用途で使う前に <https://www.anthropic.com/pricing> と必ず突き合
わせてください）。

```bash
drift cost
# drift cost — compaction billing
#   total calls      : 10
#   input tokens     : 120958
#   output tokens    : 6582
#   cache creation   : 0
#   cache read       : 0
#   total cost (USD) : $0.1539

drift cost --by model
# ── grouped by model (descending cost)
#   key                    calls   input_tok   output_tok     cost (USD)
#   claude-haiku-4-5          10      120958         6582        $0.1539

drift cost --by session
# ── grouped by session (descending cost)
#   key                                     calls   input_tok   output_tok     cost (USD)
#   4b1e2ba0-621c-4977-af3f-2a9df5ac45ec        2       51696         2448        $0.0564
#   ad01ae46-156f-403b-b263-dd04a232873a        1       33662         2390        $0.0456
#   ...
```

`--since <date>`、`--until <date>`、`--model <name>` で絞り込みできます。
同じ 10-session のコーパスで Opus を Haiku に切り替えると、compaction
コストは **$2.91 → $0.15**（~19× 削減）になります。代わりに要約はや
や簡素になります。

## AI ネイティブな blame

`drift blame` は逆引きです。ある行を渡すと、それを触った全タイムライン
（multi-agent + 人間の編集）を返し、各エントリは元の session と prompt
にリンクされます。

3 つの中核シナリオは [`docs/VISION.md`](docs/VISION.md) を参照してください：
**逆引き**（`drift blame`）、**順引き**（`drift trace`）、
**audit**（`drift log`）。

## MCP 統合

Drift AI 自身が stdio MCP server（`drift mcp`）を提供します。MCP 互換の
client なら 5 つの read-only ツール — `drift_blame`、`drift_trace`、
`drift_rejected`、`drift_log`、`drift_show_event` — を呼び出して、subshell
を立てずに attribution store を照会できます。

**Claude Code**（1 行）：

```bash
claude mcp add drift -- drift mcp
```

**Codex**：

```bash
codex mcp add drift -- drift mcp
```

ツールは設計上 read-only です — state を変更する操作（`capture` /
`bind` / `sync`）は CLI 専用のままです。

## Commands

| Command | 用途 |
|---------|---------|
| `drift init` | `.prompts/` とプロジェクト config を scaffold |
| `drift capture` | one-shot：session 発見、compact、attribute |
| `drift watch` | バックグラウンド daemon、debounce 付き再 capture |
| `drift handoff [--branch B --to A --print --output P]` | **v0.2** — cross-agent task brief |
| `drift list [--agent A]` | 取得済み session を一覧 |
| `drift show <id>` | compacted session を表示 |
| `drift blame <file> [--line N] [--range A-B]` | **逆引き** |
| `drift trace <session-id>` | **順引き** |
| `drift diff <event-id>` | 単一 event の unified diff |
| `drift rejected [--since DATE]` | 却下された AI 提案を一覧 |
| `drift log [-- <git-args>]` | `git log` + per-agent session サマリ |
| `drift bind <commit> <session>` | session を commit note に紐づけ |
| `drift auto-bind` | 各 session を timestamp で最寄りの commit にペアリング |
| `drift install-hook` | non-blocking な post-commit hook を設置 |
| `drift sync push\|pull <remote>` | `refs/notes/drift` の push / pull |
| `drift config get\|set\|list` | global + project の TOML マージ |
| `drift mcp` | stdio MCP server を起動 |

## Configuration

Global：`~/.config/drift/config.toml`
Project（上書き）：`<repo>/.prompts/config.toml`

```toml
[attribution]
db_in_git = true          # default — チームは repo 経由で blame を共有

[connectors]
claude_code = true
codex = true
aider = false             # feature-gated stub

[compaction]
provider = "anthropic"      # default；offline / テスト時は "mock" に切替
model = "claude-haiku-4-5"  # または claude-sonnet-4-6 / claude-opus-4-7

[handoff]
model = "claude-opus-4-7"   # 叙述の質が価値；~30x コストダウンしたければ
                            # "claude-haiku-4-5"
```

`ANTHROPIC_API_KEY`：live API compaction を使う場合は必須です。未設定
の場合、drift_ai は透過的に `MockProvider` にフォールバックし、すべての
要約に `[MOCK]` ラベルが付くので、フォールバック実行を本番実行と取り違
えることはありません — pipeline の他の部分は何も変わりません。

drift と Cursor / Copilot history、Cody、`git blame` 自体との比較は
[`docs/COMPARISON.md`](docs/COMPARISON.md) を参照してください。

## 正直な制約（v0.2.0）

- 人間編集の検出は SHA ladder のみ — 著者を主張することはありません。
  `human` slug は「AI session が生成しなかった」という意味です。VISION.md
  を参照してください。
- `Bash python -c "open(...).write(...)"` はベストエフォートです。shell
  lexer が取り逃したものは SHA ladder で拾われ、`human` に帰されます。
- Codex の `reasoning` items は暗号化されています — カウントはしますが、
  内容を露出することはありません。
- コスト合計はハードコードされた料金表を使用します — 請求用途で扱う前に
  <https://www.anthropic.com/pricing> と必ず突き合わせてください。
- Context-window の切り詰めは決定的な head + tail 省略（Strategy 1）で
  す。階層的サマリ（Strategy 2）は feature flag の裏で v0.2 用にスタブ化
  されています。

## About

`drift` は [@ShellFans-Kirin](https://github.com/ShellFans-Kirin)
（[shellfans.dev](https://shellfans.dev)）が個人で運営する独立した OSS
プロジェクトです。Anthropic、OpenAI、その他 drift が統合する agent
ベンダーとは **無関係** です — `drift` は彼らの session log の *上に*
構築されたものであり、彼らが作ったものではありません。

> もともと自分用に作ったツールです — Codex が止まったり Claude が
> rate-limit されたりするたびに文脈を失っていたので。v0.2 の `drift handoff`
> は私自身が一番頻繁に使う部分です。

## License

Apache 2.0 — [LICENSE](LICENSE) を参照してください。

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) を参照してください — Aider stub を
具体例として、新しい connector を追加する流れを通しで説明しています。
