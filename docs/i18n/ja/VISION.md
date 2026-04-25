> 🌐 **日本語** · [简体中文](../zh-Hans/VISION.md) · [繁體中文](../../VISION.md)

# Drift AI — プロジェクトの最終目標

**ステータス**:ビジョン文書(契約ではありません)。著者の現時点での仕様
理解と Phase 0 提案に基づいて執筆しています。
**v0.1.0 のデリバリー対応**:[PHASE0-PROPOSAL.md](../../PHASE0-PROPOSAL.md)
を参照してください。

---

## 一行で

Drift AI は **AI 時代の `git blame`** を目指しています — コードの本来の作
者がもはや単一の人間ではなく、「ある prompt が AI の出力を引き起こし、人
間が部分的に書き換え、さらに別の agent が refactor する」というハイブリッ
ドな timeline になったとき、Drift AI はこの timeline を「行単位」の粒度
で復元できる唯一のローカルツールです。

---

## なぜ必要か

### 中核の thesis:commit 粒度は粗すぎる

今日の source control は「1 つの commit = 1 つの意味のある変更、著者は
単一」と仮定しています。しかし AI コーディングのワークフローではこの前
提が崩れます:

- 1 つの commit にはしばしば **複数の AI session** の出力が含まれる
  (Claude Code が骨格を書き、Codex が境界条件を補い、Aider が lint を
  直す)。
- AI の提案のうち **採用されるのは一部** であり、残りの却下された / 書き
  換えられた / 置き換えられたアイデアも重要な設計の証拠ですが、commit
  には一切記録されません。
- ユーザーが **AI が書いた code を手で修正した** あと、git は「最終的な
  文字列」しか見えず、「人間が AI の上にどんな judgement を加えたか」は
  見えません。
- **Rename / refactor** が血統を断ち切り、`git blame` はファイル名変更
  以前のソースを追えません。

これらの情報は **どこかに存在しています** — Claude Code の
`~/.claude/projects/*.jsonl`、Codex の `~/.codex/sessions/**/*.jsonl`、
そしてユーザーのディスク上のファイルの過去の SHA 状態。Drift AI がやる
のは、これらの **散在し、短命で、互いに繋がっていない** 信号を、**単一
の、クエリ可能な、commit に紐づく** timeline に編み上げることです。

### なぜ「local-first(ローカル優先)」なのか

- **プライバシー**:AI session の完全な対話には商業秘密、auth token、
  ユーザーの意図が含まれます。クラウドに上げるかどうかは drift_ai が決
  めるべきことではありません。
- **オフライン可能、可搬性**:システム全体は git + SQLite + ローカルの
  session ファイルのみに依存します。repo を clone すれば blame レイヤー
  もついてきます。
- **ベンダーロックインなし**:Drift AI はあなたの prompt を所有しません。
  散在する記録を `.prompts/` ディレクトリと `refs/notes/drift` に整理す
  るだけで、いつでも無効化、削除、ツールごと差し替え可能です — データ
  はあなたのものです。

---

## ユーザーが実際に使う 3 つのシナリオ

### シナリオ 1:事後デバッグ(逆引き、`drift blame`)

> 「この rate limiter なんでこう書かれてるの?自分で書いた覚えがない。」

```
$ drift blame src/auth/login.ts --line 42
src/auth/login.ts:42
├─ 2026-04-15 14:03  [claude-code]  session abc123  prompt: "add rate limiting"
│  diff: +  if (attempts > 5) throw new RateLimitError()
├─ 2026-04-15 15:20  [human]        post-commit manual edit
│  diff: -  if (attempts > 5)
│         +  if (attempts > MAX_ATTEMPTS)
└─ 2026-04-16 09:12  [codex]        session def456  prompt: "extract magic numbers"
   diff: +  const MAX_ATTEMPTS = 5
```

新しいチームメンバー、あるいは 3 ヶ月後に古い repo を開き直した自分にと
って、この timeline はツール全体の価値に見合います。

### シナリオ 2:設計判断の振り返り(順引き、`drift trace`)

> 「先週の OAuth 大改修、Claude と何を議論したっけ?最終的に何で manual
>  JWT じゃないんだ?」

```
$ drift trace abc123
Session abc123 (claude-code, 2026-04-15 14:00–14:47, 7 turns)
  files_touched:     src/auth/{login,session,callback}.ts
  key_decisions:     NextAuth over manual JWT (turn 4)
  rejected:          manual JWT approach (turn 3) — "too much token refresh boilerplate"
  code_events:       17 total (14 accepted, 3 rejected)
```

commit message には「Add OAuth」とだけ書いてありますが、「なぜ NextAuth
で manual JWT じゃないのか」という事実を覚えているのは Drift AI だけです。

### シナリオ 3:監査とコンプライアンス(`drift log`)

> 「今回の release で、どのコードが AI 生成で、どのコードが人間が書い
>  たもの?」

```
$ drift log v0.3.0..v0.4.0
commit 7f8a12b — feat: add webhook retries
  💭 [codex]      4 turns, exponential backoff design
  ✋ [human]      1 manual edit  (src/webhooks/retry.ts L88)

commit 2b4c5d9 — fix: race in session cache
  💭 [claude-code] 2 turns, spotted the TOCTOU
  💭 [claude-code] 1 turn,   applied the fix
```

「AI 貢献率」を追跡する必要があるチーム(コンプライアンス、教育、研究)
にとって、これは **token カウントの推測ではなく、本当に commit に紐づい
た** 唯一の記録です。

---

## ビジョンの技術的バックボーン

3 つのレイヤー、それぞれが置き換え可能な抽象化:

### 1. Connector レイヤー(attribution の原料)

- **Day-one**:Claude Code + Codex のデュアル first-class — これは初日
  から「クロス agent」抽象化を実圧テストするためです。1 つの agent しか
  サポートしないと、抽象化は wishful thinking に陥りがちです。
- **Aider stub**:例として、CONTRIBUTING の「新しい connector を追加する」
  ウォークスルーは aider をデモに使います。
- 将来の任意の AI CLI(Cursor CLI、Cline、自作 agent…)は
  `SessionConnector` trait を実装するだけで接続できます。

### 2. Compaction レイヤー(可読性)

- 元の session JSONL は 1 つで 500K にもなり、人間には読めません。LLM
  compaction はそれを ~1K の markdown(frontmatter + 決定の要約 + 却下
  されたアイデア)に圧縮します。
- **Provider-agnostic**:Anthropic がデフォルト、`MockProvider` がテスト
  用です。将来は OpenAI、ローカルの Ollama、`CompactionProvider` インタ
  ーフェースに準拠する任意の実装に接続できます。
- Compacted な結果は `.prompts/sessions/` に commit されます — **repo
  と一緒に移動し、永遠に読める** ものであり、Drift AI が存続することに
  依存しません。

### 3. Attribution レイヤー(中核の差別化)

- **`CodeEvent`**:ファイル変更ごとに 1 行、`diff_hunks`、
  `parent_event_id`、`content_sha256_after`、`rejected`、`rename_from`
  を含みます。
- **SHA-256 ladder**:「AI が書き終わったあとに、人間が SHA を変えた」
  信号を検出します — これは Drift AI が誠実に人間の手による編集を検出
  できる唯一の方法であり、authorship judgement のふりはしません。
- **Rename の 2 段階戦略**:session tool call lexer → `git log --follow`
  fallback。
- **Git notes binding**:`refs/notes/drift` で compaction 結果を commit
  に紐づけ、`events.db` に CodeEvent を入れ、config が git に入れるかを
  決定します。

---

## スコープの境界 — Drift AI が **やらない** こと

意図的にやらないこと(忘れているわけではありません):

| やらない | 理由 |
|---|---|
| クラウドの SaaS ダッシュボード | local-first に違反します。将来チーム UI が必要になれば、それは `.prompts/` + notes を読むサードパーティの web UI であるべきで、Drift AI 自身がクラウドサービスを生やすべきではありません。 |
| 「ある行の作者は誰か」を判定する | SHA は「誰がこの変更をしなかったか」(AI session の外で誰かがやった)しか教えてくれず、「実際にキーを叩いたのは誰か」は教えてくれません。Drift AI の `human` slug は意味的に「AI session の生成ではない」を表すだけで、authorship を主張しません。 |
| 「AI 貢献率」を定量化する | 行数比は単純に操作できます(format、rename がスコアを膨らませる)。私たちは生の event timeline を提供します。あらゆる測定はそれを使う上位ツールが自分の定義で計算します。 |
| AI session の中で middleware として prompt を遮断 / 改変する | Drift AI は **受動的な観察者** であり、傍受も改変もプロキシもしません。すべての上流 agent はそのまま動きます。私たちは完了後の記録だけを読みます。 |
| git や git-blame を置き換える | 私たちは git を **拡張** し、git notes で 1 層を重ねるだけで、底層を置き換えません。Drift AI を取り外しても、git の操作は普段どおりです。 |

---

## 成功の長期像

1. **エコシステム互換性**:Claude Code / Codex / Aider / Cursor CLI / 将
   来登場する新 agent がリリースされた週には、対応する connector PR がコ
   ミュニティから入ってきます。Connector trait がきれいで、CONTRIBUTING
   に例があるからです。
2. **チーム blame**:複数人で開発する repo では、各自の AI session が
   それぞれ Drift AI に取り込まれ、`drift sync push/pull` で notes がマ
   シン間を流れます — **誰の `drift blame` でもチーム全体の timeline が
   見える**(プライバシーモード:`db_in_git = false` の場合はローカル
   のみ保存)。
3. **新人エンジニアの最初のコマンドになる**:`git log` ではなく、
   `drift blame <混乱を招く function>` が最初のコマンドになる — それ
   が「なぜこれがこう書かれているのか」の対話のフルコンテキストを直接
   提供するからです。
4. **コンプライアンスと研究の標準フォーマット**:AI 生成コードの監査、
   教育研究、貢献度測定はすべて `.prompts/` + `refs/notes/drift` を
   入力スキーマとして使います — これが AI session を本当に commit に
   紐づけた最初の OSS フォーマットだからです。

---

## v0.1.0 との関係

このドキュメントが描いているのは **北極星** です。v0.1.0 が証明すべきは
このルートが **技術的に成立する** ことです:

- ✅ デュアル agent をまたぐ抽象化(single-agent ハックではない)
- ✅ Line-level(commit-level の手抜きではない)
- ✅ データモデルが 4 つの非自明なシナリオ(multi-origin、human edit、
  rejected、rename)を支える — これは Phase 0 の
  [自己評価](../../PHASE0-PROPOSAL.md#f-self-evaluation-does-the-data-model-honor-the-four-requirements)
- ✅ `drift blame` のデモが本当に動く

v0.1.0 にはありません:team-sync の精細な UI、Cursor connector、web イン
ターフェース、貢献度測定。それらは v0.2+ の話で、骨組みが立つことを確
認してから加えます。

---

## なぜ今これをやるのか

- AI コーディングツールのエコシステムは 2025-2026 年にかけて「Claude
  Code / Codex の双強 + ロングテール」に急速に収束しており、スキーマが
  比較的安定していて、初の cross-agent 抽象化を打ち込むのに適していま
  す。
- 底層機構としての `git notes` は git 2.40+ 以降 merge-friendly で、
  push/pull が成熟しました。
- SQLite + 単一バイナリ配布によって、local-first daemon のエンジニア
  リングコストは史上最低レベルです。
- 「AI ネイティブ blame」を真剣にやる OSS ツールはまだありません — Mem0
  / Supermemory は memory レイヤーをやっていて、attribution レイヤーで
  はありません。それは全く別の問題です。

タイミング、技術、需要の 3 つが揃いました。v0.1.0 はこのウィンドウで
骨組みを立てる仕事です。
