> 🌐 [English](CHANGELOG.md) · **日本語** · [简体中文](CHANGELOG.zh-Hans.md) · [繁體中文](CHANGELOG.zh-Hant.md)

# Changelog

drift_ai のすべての注目すべき変更をここに記録します。
フォーマットは [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) に従い、
バージョニングは [SemVer](https://semver.org/spec/v2.0.0.html) に従います。

## [0.2.0] — 2026-04-25

「単一の LLM ベンダーにロックインされない」リリース。新しい目玉コマンド
**`drift handoff`** と、task transfer をフロントに据えて blame を補助機能に
位置付け直した v0.2 スタイルの README を追加しました。

### Added

- **`drift handoff` CLI**。進行中のタスク（`--branch`、`--since`、
  `--session` でフィルタ）を、別の agent が初見で読み込める markdown brief に
  パッケージします。フラグ：`--to claude-code|codex|generic`、
  `--output <path>`、`--print`。デフォルト出力先：
  `.prompts/handoffs/<YYYY-MM-DD-HHMM>-<branch>-to-<agent>.md`。
- **`crates/drift-core/src/handoff.rs`** — orchestrator + 4 つの小さな
  collector（sessions、events-by-file、rejected approaches、file snippets）
  + LLM second pass + 純 Rust の `render_brief`。新しい unit test が scope
  parsing、snippet 抽出（full vs. modified-range の抜粋）、JSON-from-LLM
  パース（code-fence と周辺文の許容）、`--to` ごとの footer 描画をカバー。
  15 件の新規テスト。
- **`crates/drift-core/templates/handoff.md`** — second pass 用の LLM prompt
  テンプレート。`summary` / `progress` / `key_decisions` / `open_questions`
  / `next_steps` を含む JSON を出力するよう model に指示します。
- **`AnthropicProvider::complete_async`** + 同期版 `complete` — 汎用の
  system+user → text 補完。`compact_async` の retry / streaming / token
  使用量機構を、別の prompt 形状で LLM を呼びたい caller（handoff など）
  向けに再利用します。新しい `LlmCompletion` struct（text + per-call
  token / cost）を返します。
- **`[handoff]` config セクション**を `.prompts/config.toml` に追加。
  デフォルト model は `claude-opus-4-7`。デフォルトが Opus なのは、
  handoff brief が次の agent によって逐語的に読まれる user-facing アーティ
  ファクトであり、叙述の質こそが価値で、handoff の頻度はそもそも低い
  （多くて 1 営業日に数回）からです。コストを ~30× 下げたいなら Haiku に
  切り替えられます。
- **30 秒のデモ**を `docs/demo/v020-handoff.gif` に追加（fixture data に
  対する `drift handoff` の実録。元の cast ファイルは
  `docs/demo/v020-handoff.cast`）。
- **本物の Anthropic smoke 出力**を
  [`docs/V020-SMOKE-OUTPUT.md`](docs/V020-SMOKE-OUTPUT.md) に保存。
- **`docs/V020-DESIGN.md`** — Phase 0 の設計提案。`drift handoff` の形を
  決めた経緯のリファレンスとして repo に残しています。

### Changed

- README の最初の画面を `drift handoff` を主役に作り変え、demo GIF を
  hero 位置に。blame / log は同じ画面の「supporting feature」リファレ
  ンスとして保持。
- Quickstart を 5 コマンドから 6 コマンドに（`drift handoff` を追加）。
- About セクションに dogfood 由来であることを 1 行追記。
- 事前ビルド済みバイナリの URL を `drift-v0.2.0` に更新。

### v0.1 から引き継ぐ安定性保証

- `events.db` のスキーマは **不変** です。v0.1.x からのアップグレードは
  純粋なバイナリ差し替えで OK。マイグレーションは不要です。
- MCP のツール一覧は **不変** です。既存の MCP クライアントはそのまま動
  きます。
- `SessionConnector` trait は **不変** です。既存の connector はそのまま
  動きます。
- v0.1.2 の first-run プライバシー通知は最初の `drift capture` で引き続
  き発火します。handoff のために再承認する必要はありません。

### 既知の制約（v0.2）

- `--branch <name>` のスコープは best-effort です：`git log <branch>
  --not main --format=%aI` を使って最も古い分岐 commit を取り、それを
  下限フィルタにします。同日に並行する複数の branch に乗っている session
  はにじむ可能性があります — `--since` で絞り込んでください。
- handoff の LLM 呼び出しは Opus 呼び出しのコストプロファイルです
  （brief 1 件あたり ~$0.10）。ヘビーユーザーは
  `[handoff].model = "claude-haiku-4-5"` を設定してください。
- `drift handoff list` / `drift handoff show <id>` はまだありません —
  生成された brief は `.prompts/handoffs/` 配下の markdown ファイルに
  すぎません。`ls` と `cat` が v0.2 のクエリインターフェースです。

## [0.1.2] — 2026-04-25

v0.1.1 の上に重ねたドキュメント + メッセージのパッチ。compaction /
attribution / MCP のコードパスは v0.1.1 から変わっていません。挙動として
の変更は、初めて `drift capture` を実行したときの一回限りのプライバシー
通知のみです。

### Added
- **`docs/SECURITY.md`** — threat model、現状の制約、利用可能な mitigation
  （db_in_git トグル、手動 review、gitleaks/trufflehog の pre-commit）、
  v0.2 ロードマップ（regex redaction pass、対話 review モード、
  `drift redact` の事後 scrub）、セキュリティ報告経路。
- **README の `## Privacy & secrets` セクション** — `drift capture` が
  session 内容を `.prompts/` にミラーし、`events.db` をデフォルトで git
  に commit することを、ぼかさず明示的に開示。
- **`drift capture` の初回 notice** — 初回呼び出しでプライバシー方針を
  1 段落リマインドし、stdin を待ちます。`DRIFT_SKIP_FIRST_RUN=1` でバイ
  パス（CI 向け）。状態は `~/.config/drift/state.toml::first_capture_shown`
  に記録。
- **`docs/COMPARISON.md`** — Cursor / Copilot chat / Cody / `git blame`
  に対する機能比較。README からリンク。
- **README 痛点オープナー** — 1 段落（"47 prompts to Claude + 3 Codex
  fills + 12 manual edits ..."）を技術説明の上に。
- **README の `## About` セクション** — drift が独立プロジェクトであり、
  Anthropic、OpenAI、その他の agent ベンダーとは無関係であることを明示。
- **README バッジ**：crates.io バージョン + CI ステータス（最大 2 つに）。
- **Provider 切替の例**を `## Configuration` に追加し、v0.2 のプラン
  （ollama / vllm / openai-compatible）に言及。

### Tests
- `tests/first_run_notice.rs` で `DRIFT_SKIP_FIRST_RUN=1` のバイパスと
  state ファイル永続化のパスをカバー。

### v0.1.1 から引き継ぐ既知の制約
- Drift は依然として secret を能動的に redact しません — それは v0.2 の
  作業です。
- 価格表はハードコードです。請求レポートに使う前に Anthropic の公開価格
  表と必ず突き合わせてください。

## [0.1.1] — 2026-04-23

### Added
- **Live Anthropic compaction.** `AnthropicProvider` は実際に
  `POST /v1/messages?stream=true` を叩き、SSE ストリームを消費し、CLI
  実行中は content delta を stderr に echo し、`message_stop` の `usage`
  ブロックを billing 用にパースします。
- **Typed compaction error**（`CompactionError`）：`AuthInvalid`、
  `RateLimited { retry_after }`、`ModelNotFound`、`ContextTooLong`、
  `TransientNetwork`、`Stream`、`Other`。各 variant が CLI 上の固有の
  オペレータ向けメッセージに対応します。
- **Config による model 切替**：`[compaction].model` は
  `claude-opus-4-7`（デフォルト）、`claude-sonnet-4-6`、
  `claude-haiku-4-5` を受け付けます。
- **Retry policy**：429 は `Retry-After` を尊重して 5 回；5xx は
  指数バックオフ（1s → 2s → 4s → 8s）で 4 回；401/404 は即時失敗。
- **Context-window 切り詰め**：char ベースの token 推定 + 80% しきい
  値。Strategy 1 は head(8) + tail(8) のターンを残し、中間を明示マーカー
  で省略します。
- **`compaction_calls` テーブル**（SQLite migration v2）：呼び出しごと
  の input / output / cache-creation / cache-read トークンと算出済み
  USD コスト。
- **`drift cost`** CLI：`--since <iso>` / `--until <iso>` /
  `--model <name>` / `--by model|session|date`。
- **`drift watch` は event-driven**：`notify`（FSEvents/inotify/
  ReadDirectoryChangesW）でバックエンド、200ms debounce、ファイル名から
  推定する session_id ごとの capture、状態は
  `~/.config/drift/watch-state.toml` に永続化、SIGINT/SIGTERM は走行中
  の capture を完了させてから終了します。
- **Homebrew tap が稼働**：`brew install ShellFans-Kirin/drift/drift` で
  公開 [homebrew-drift](https://github.com/ShellFans-Kirin/homebrew-drift)
  tap に対してインストール可能。formula は release のたびに `release.yml`
  の `repository_dispatch` から自動再生成。
- **crates.io に公開**：`drift-core`、`drift-connectors`、`drift-mcp`、
  `drift-ai`。

### Changed
- `CompactionProvider::compact` は `CompactedSummary` 単独ではなく
  `CompactionResult`（summary + 任意の usage）を返すように。これにより
  live provider が billing データをラウンドトリップできます。
- `drift init` は idempotent：再実行しても既存の `config.toml` を上書き
  しません。
- `drift capture` は単一 session の compaction error に対して soft-fail
  します（log + skip）。1 つの過大な session でバッチ全体が止まらないよ
  うに。
- `summary_to_markdown` は本物のセクション見出し（`## Summary`、
  `## Key decisions`、`## Files touched`、`## Rejected approaches`、
  `## Open threads`）を出すように。元の 1 行 `[MOCK]` blurb を置き換え。

### Fixed
- Workspace 内部依存を 0.1.1 にピン止め（以前は 0.1.0）。これで
  `cargo publish` が crates.io に対して解決できるように。
- 出荷時の smoke で誤ってチェックインされていた `.prompts/events.db` を
  ignore に。新規 clone 用に `.prompts/` を `.gitignore` に追加。

### Known limitations
- Context-window の Strategy 2（階層的サマリ）は scaffold 済みですが
  feature flag でオフ。デフォルトの挙動は Strategy 1。
- コスト合計はハードコードされた料金表を使用（2026-04-23 時点の
  Anthropic の公開価格と照合済み）。請求レポートに使う前に
  <https://www.anthropic.com/pricing> と再度突き合わせてください。

## [0.1.0] — 2026-04-22

### Added
- 4 crate からなる Cargo workspace：`drift-core`、`drift-connectors`、
  `drift-cli`（binary は `drift`）、`drift-mcp`。
- Claude Code + Codex の first-class コネクタ。Aider は feature flag
  （`aider`）の裏に stub。
- Attribution engine：`CodeEvent` 行を `.prompts/events.db`（SQLite）に
  永続化、人間編集検出用の SHA-256 ladder、リネームの 2 段処理
  （session tool call → `git log --follow` fallback）、MultiEdit の
  intra-call parent chain。
- Compaction engine：`MockProvider`（v0.1.0 のデフォルト、`[MOCK]`
  タグ付き）と `AnthropicProvider` のスケルトン（HTTP 呼び出しは v0.1.1
  で接続）。
- CLI：`init`、`capture`、`watch`、`list`、`show`、`blame`、`trace`、
  `diff`、`rejected`、`log`、`bind`、`auto-bind`、`install-hook`、
  `sync push/pull`、`config get/set/list`、`mcp`。
- Git notes の統合（`refs/notes/drift`）：手動 binding、timestamp に
  よる auto-bind、non-blocking な post-commit hook。
- 5 つの read-only ツールを持つ stdio MCP server：`drift_blame`、
  `drift_trace`、`drift_rejected`、`drift_log`、`drift_show_event`。
- Plugin スケルトン（`plugins/claude-code/`、`plugins/codex/`）— v0.1.0
  では未公開。v0.2 でマーケットプレイスを目指します。
- CI（`.github/workflows/ci.yml`）と release（`release.yml`）の
  Linux x86_64/aarch64 + macOS x86_64/aarch64 マトリクス。
- Apache-2.0 ライセンス、新コネクタ追加の CONTRIBUTING ウォークスルー、
  code-of-conduct。

### 既知の制約
- Anthropic compaction の HTTP 呼び出しは stub。出荷デフォルトは
  Mock パス。接続方法は `crates/drift-core/src/compaction.rs` に記載。
- 人間編集検出は timeline のみ — 著者主張はしません。
- Codex の `reasoning` items は暗号化されています。カウントはしますが
  内容は surface しません。
- `drift watch` は debounce 付きのポーリング daemon。v0.2 で完全に
  event-driven に移行します。
- このカットでは `cargo publish` を実行していません。`0.1.1` 用の
  Cargo.toml メタデータは整備済みです。
