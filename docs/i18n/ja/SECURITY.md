> 🌐 [English](../../SECURITY.md) · **日本語** · [简体中文](../zh-Hans/SECURITY.md) · [繁體中文](../zh-Hant/SECURITY.md)

# セキュリティとプライバシー

Drift AI は local-first のツールです。これが取り込むデータ — あなたの AI
コーディング session — には repo に commit したくないものが含まれることがあ
ります。このドキュメントでは threat model を正直に書きます。

## Threat model

Drift には **server がありません**。Drift 自身が何かをどこかにアップロードす
ることはありません。データの流れはこうです：

1. あなたの AI agent（Claude Code、Codex…）は session JSONL を `~/.claude/projects/`
   や `~/.codex/sessions/` 配下の自分のディレクトリに書き出します。これは
   agent の挙動であって、Drift のものではありません。
2. `drift capture` はそれらのファイルを読み、次を書き出します：
   - `code_events` 行を `<repo>/.prompts/events.db`（SQLite）に
   - session ごとに 1 つの Markdown を `<repo>/.prompts/sessions/` に
3. `[compaction].provider = "anthropic"`（デフォルト）の場合、`drift capture`
   は各 session の transcript を `api.anthropic.com/v1/messages` に送り、
   Markdown 要約を生成します。**これが唯一のネットワーク egress です。**
   `provider = "mock"` に切り替えれば完全にスキップできます。

ステップ 3 以外はすべてあなたのマシンに残ります。

## 現状の制約（v0.1.x）

以下は **既知** かつ **明文化済み** であり、bug ではありません：

1. **`drift capture` は session 内容を scrub しません。** Claude / Codex の
   chat に打ち込んだ内容 — うっかり貼った secret も含めて — はそのまま
   `events.db` と `.prompts/sessions/*.md` にミラーされます。
2. **`events.db` はデフォルトで git に commit されます**
   （`[attribution].db_in_git = true`）。意図は team blame の共有ですが、副作
   用として session に漏れた secret が public repo に流れます。
3. **`.prompts/sessions/*.md` は人間可読です**：compacted な要約はファイル名、
   判断、そしてしばしば diff hunk を逐語的に保持します。Anthropic の compactor
   も能動的に secret を redact することはありません。

`export AWS_SECRET_ACCESS_KEY=AKIA...` のようなものを Claude session に貼っ
たことがあれば、その文字列は `events.db` に、そしておそらく compacted な
Markdown にも残ります。

## 今すぐ使える mitigation

ワークフローに合うものから 1 つ選んでください：

1. **git 側を無効化**：
   ```toml
   # .prompts/config.toml
   [attribution]
   db_in_git = false
   ```
   `events.db` と markdown はローカルに留まります。チームは共有 blame を
   失いますが、あなたのローカルビューは保たれます。

2. **commit 前に手動 review**：
   ```bash
   drift capture
   git diff --cached -- .prompts/
   # 要約を実際に読む。必要なら直接 redact する
   git add .prompts/ && git commit
   ```

3. **secret scanner を pre-commit hook と組み合わせる**。Drift は同梱しません
   が、[gitleaks](https://github.com/gitleaks/gitleaks) や
   [trufflehog](https://github.com/trufflesecurity/trufflehog) で大半のパター
   ンを拾えます。例：
   ```bash
   # .git/hooks/pre-commit
   gitleaks protect --staged --redact -v || exit 1
   ```

4. **オフラインで動かす**：`[compaction].provider = "mock"` を設定し、
   `ANTHROPIC_API_KEY` を unset してください。LLM 要約は失われますが、
   `events.db` は純粋にローカルインデックスとして残ります。

5. **chat に secret を貼ることが日常的に起こる repo では、v0.2 の redaction
   pass が出るまで Drift を有効にしないでください**。待つことは恥ではあり
   ません。

## Roadmap（v0.2+）

以下は計画された作業であり、日付付きの約束ではありません：

- **`drift capture` 内の regex ベース redaction pass** — 高信頼度のパターン
  （Anthropic / OpenAI / AWS / GitHub PAT / Slack / private key blob）を識別
  して、`events.db` に到達する前に `<redacted>` プレースホルダーで置き換え
  ます。
- **対話式 review mode**：`drift capture --review` で各 Markdown を `$EDITOR`
  に開き、確認してから永続化します。
- **プラガブルな detector**：`trufflehog` / `gitleaks` のルール集を取り込ん
  で、regex を再発明しないようにします。
- **`drift redact <session-id>`**：すでに capture 済みの session に対する
  事後 scrub。明確な undo パス付き。

もし優先したいものがあれば feature request を立ててください — 具体的な
use case があると優先順位は上がります。

## セキュリティ問題の報告

credential リーク、supply-chain リスク、RCE につながる可能性があるものは、
[GitHub Security Advisories](https://github.com/ShellFans-Kirin/drift_ai/security/advisories/new)
を使ってください。public issue を立てないでください。

ドキュメント漏れ、threat model の訂正、「mitigation X が抜けている」と
いった内容は、通常の issue や PR で構いません。
