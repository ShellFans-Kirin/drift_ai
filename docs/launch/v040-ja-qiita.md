# Qiita 草稿 — v0.4.0(日本語、ローカル Ollama + プライバシー視点)

# Drift v0.4 リリース:あらゆる AI コーディング agent × あらゆる LLM 間の handoff

完全ローカルで動く Rust 製の CLI です。Claude Code / Codex / Cursor / Aider
の session を取り込んで、別の agent に貼って続きを書ける markdown brief を
生成します。

## 何が嬉しいか

AI コーディングは複数 agent を切り替えながら進める時代です:

- Claude Code が rate limit に当たった
- Cursor の context window が溢れた
- Codex の出力品質が突然落ちた

切り替え先の agent に「何を決めたか」「何を試して却下したか」「どこから
再開するか」を毎回手で説明し直すのは時間の無駄です。

drift はその context を **ローカルで** 拾い、**ローカルで** 圧縮し、
git repo の中に永続化します。クラウド送信なし(オプションの LLM 圧縮
呼び出しを除く)。

## v0.4 の二軸 vendor-neutral

**取り込み元** に Cursor + Aider が加わりました。Cursor は per-workspace
SQLite(`state.vscdb`)を read-only で読み、Aider は
`.aider.chat.history.md` を markdown として解析します。

**圧縮 LLM** は Anthropic / OpenAI / Gemini / Ollama がネイティブ、加えて
OpenAI 互換プロトコル汎用クライアントで DeepSeek / Groq / Mistral /
Together AI / vLLM / LM Studio が動きます。

## ローカル LLM 構成例(Ollama)

機密性の高いコードベースを扱う場合、handoff の summarise を Ollama に流
すとデータがマシン外に出ません:

```toml
# .prompts/config.toml

[handoff]
provider = "ollama"

[handoff.providers.ollama]
base_url = "http://localhost:11434"
model = "llama3.3:70b"   # 64GB RAM 推奨。8b は省メモリ向け代替
```

`ollama serve` を起動して、

```bash
drift handoff --branch feature/oauth --to claude-code
```

これだけ。クラウド送信ゼロで brief が出ます。

## 真夏の Anthropic Opus と較べると

ベンチマーク(同じ 4-turn fixture セッション):

| Provider | Cost (USD) |
|---|---:|
| Anthropic claude-haiku-4-5 | $0.00133 |
| OpenAI gpt-4o-mini | $0.00015 |
| Gemini 2.5-flash | $0.00019 |
| DeepSeek deepseek-chat | $0.00023 |
| **Ollama llama3.3:70b**(ローカル) | **$0.00** |

Ollama は無料で、ネットワークに何も流さないという最強の特性があります。
精度面では Haiku より一段下がりますが、handoff brief の用途では実用上
問題ないと感じています。

## インストール

```bash
brew install ShellFans-Kirin/drift/drift
# または
cargo install drift-ai   # Rust 1.85+
```

## 制約

- Cursor のスキーマは公式ドキュメントが無く、リバースエンジニアリングし
  たものです。Cursor 側の更新で connector が壊れる可能性があります
  ([BEST-EFFORT] と doc コメントに明記)。
- Aider は markdown 形式で tool_call 構造がないため、`rejected = false`
  がデフォルトです。SHA-256 ladder で実態との乖離は検出します。
- secret の自動 redaction は v0.5 で導入予定。それまでは
  `[attribution].db_in_git = false` でローカル限定にするか、`drift
  capture` の前に `gitleaks` 等で手動チェックを推奨します。

## ロードマップ

- v0.5 — secret redaction / Cursor agent モード完全対応 / Bedrock 等エンタープライズ wrappers
- v0.6 — team handoff(notes 同期 UX)/ ベンダー別の brief body 翻訳
- v1.0 — schema 安定保証

## 関連リンク

- リポジトリ: https://github.com/ShellFans-Kirin/drift_ai
- 設計文書: [docs/V030-V040-DESIGN.md](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-DESIGN.md)
- 実 API smoke 結果: [docs/V030-V040-SMOKE.md](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-SMOKE.md)

Apache 2.0、独立プロジェクト。Issue / PR は大歓迎です。
