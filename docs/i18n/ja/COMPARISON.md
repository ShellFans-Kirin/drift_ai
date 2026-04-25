> 🌐 [English](../../COMPARISON.md) · **日本語** · [简体中文](../zh-Hans/COMPARISON.md) · [繁體中文](../zh-Hant/COMPARISON.md)

# `drift` の他ツールとの比較

これは *機能面* での比較です — 各ツールが何をどこに保存し、どんなクエリをサ
ポートするか。「どれが優れているか」の判定ではありません。これらのツールは
解く問題が重なりつつも違います。**`drift` はどれも置き換えません。`drift`
はこれらが書き出すものを読み、その上に attribution の層を載せます。**

| Tool | AI session を保存？ | 場所 | 行単位 blame？ | マルチ agent？ | Local-first？ |
|---|---|---|---|---|---|
| Cursor history | ✓ | クラウド（Cursor servers） | ✗ | ✗（Cursor のみ） | ✗ |
| GitHub Copilot chat history | ✓ | クラウド（GitHub） | ✗ | ✗（Copilot のみ） | ✗ |
| Cody (Sourcegraph) | ✓ | クラウド（Sourcegraph） | ✗ | ✗（Cody のみ） | ✗ |
| `git blame` | — | ローカル repo | commit 単位のみ | —（コードのみ） | ✓ |
| **`drift`** | ✓ | **repo 内のローカル `.prompts/`** | **✓ per line** | **✓ Claude + Codex + human + 拡張可能** | **✓** |

## 各ツールが実際に答えていること

- **Cursor history / Copilot chat history / Cody**：「自分が agent とどんな
  会話をしたか」。日付やチャットスレッドでインデックスされます。単一ベンダー
  の UI に紐づき、サブスクをやめたりツールを乗り換えたりすると消えます。

- **`git blame`**：「誰がこの行をどの commit で導入したか」。commit message
  と committer email を答えます。agent や prompt の知識はありません。git に
  同梱されており、どんな repo でもセットアップ不要で動きます。

- **`drift`**：「誰が — *どの agent がどの prompt で、あるいはどの commit の
  後にどの人間が編集したか* — この行を導入したか、*そしてそれはどんな diff
  だったか*？」file + line、session、commit、agent、rejected ステータスで
  クエリできます。repo 内に SQLite store を、そしてほかの AI ツールが逆に
  attribution を問い合わせるための MCP server を内蔵します。

## `drift` が各ツールを拡張するところ

- Cursor を使っているなら、`SessionConnector`（`crates/drift-connectors/`
  内）に対して `cursor` コネクタを書けば、`drift` は Cursor のローカル
  session JSONL を Claude Code / Codex と並べてインデックスします。
  [`CONTRIBUTING.md`](../../../CONTRIBUTING.md) を参照してください — Aider
  の stub が現成の例です。

- Copilot chat を使っているなら、同じです — Copilot がローカルに保つキャッ
  シュディレクトリにコネクタを向ければ、統一された blame が得られます。

- AI agent を一切使わずに `git blame` だけ使っているなら、`drift` が足すのは
  「人間編集の検出」層（`AgentSlug::Human` は「AI session が生成しなかった」
  の意であり、著者主張ではありません）— ただし得るものは小さいです。
  `git blame` だけであなたのケースには十分です。

Thesis：複数の agent と手作業の編集が混在し、しかも使う agent が半年ごと
に入れ替わる状況では、「session はうちのクラウドに」というモデルは壊れます。
`drift` は source of truth をあなたの git repo に置くので、attribution は
ベンダーの移行を生き延びます。

## `drift` ではないもの

- chat クライアントではありません。`drift` は Cursor の chat UI や
  Claude Code の REPL を置き換えません。
- code review ツールではありません。何が起きたかを記録するだけで、AI の
  提案が正しいかは判定しません。
- プライバシー製品ではありません。デフォルト設定では `events.db` を repo に
  commit します — トレードオフは [`SECURITY.md`](SECURITY.md) を参照してく
  ださい。
