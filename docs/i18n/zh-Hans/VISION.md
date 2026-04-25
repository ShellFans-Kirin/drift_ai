> 🌐 [日本語](../ja/VISION.md) · **简体中文** · [繁體中文](../../VISION.md)

# Drift AI — 项目终极目标

**状态**：愿景文档（非合约）。作者基于当前规格与 Phase 0 提案的理解撰写。
**对应 v0.1.0 交付**：见 [PHASE0-PROPOSAL.md](../../PHASE0-PROPOSAL.md)。

---

## 一句话

Drift AI 要成为 **AI 时代的 `git blame`** — 当代码的原始作者不再是单一个人、
而是「一个 prompt 触发一段 AI 产出、被人类部分改写、又被另一个 agent 重构」
的混血 timeline 时，Drift AI 是唯一能把这段 timeline 还原到「每一行」粒度
的本地工具。

---

## 为什么需要它

### 核心 thesis：commit 粒度太粗

今天的 source control 假设「一个 commit = 一次有意义的变动、作者单一」。
但在 AI coding 的工作流里，这个前提破了：

- 一次 commit 经常包含 **多个 AI session** 的产出（Claude Code 写了骨架、
  Codex 补了边界条件、Aider 修了 lint）。
- AI 提的建议里，**被采纳的只是一部分** — 剩下被驳回、被改写、被替换的
  思路，同样是重要的设计证据，但 commit 完全不会记录。
- 用户 **手改了 AI 写的 code** 之后，git 只看得到「最后的文字」，看不出
  「人类在 AI 的基础上做了什么 judgement」。
- **Rename / refactor** 把血统切断，`git blame` 跟不到文件名变更前的来源。

这些信息**都在某处存在** — Claude Code 的 `~/.claude/projects/*.jsonl`、
Codex 的 `~/.codex/sessions/**/*.jsonl`、以及用户硬盘上文件的历次 SHA
状态。Drift AI 做的事情就是把这些 **散落、短命、彼此不相通** 的信号，编织
成一份 **单一、可查询、跟着 commit 走** 的 timeline。

### 为什么是「本地优先 (local-first)」

- **隐私**：AI session 的完整对话含商业秘密、auth token、用户意图。上云
  不是 drift_ai 该做的决定。
- **可离线可携**：整个系统只依赖 git + SQLite + 本地 session 文件。clone 一个
  repo，blame 层就跟着走。
- **没有厂商锁定**：Drift AI 不拥有你的 prompt；它只是把散落的记录整理成
  `.prompts/` 目录与 `refs/notes/drift`，任何时候可以停用、移除、甚至换掉
  整个工具 — 数据仍是你的。

---

## 用户真正会用到的三个场景

### 场景 1：事后调试（反查，`drift blame`）

> 「这段 rate limiter 为什么这样写？我不记得自己写过。」

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

对一个新进 team member、或三个月后重开老 repo 的自己，这份 timeline 值回
整个工具的价值。

### 场景 2：回顾设计决策（顺查，`drift trace`）

> 「上周那个 OAuth 大改，我到底跟 Claude 讨论了什么？最后为什么不用 manual JWT？」

```
$ drift trace abc123
Session abc123 (claude-code, 2026-04-15 14:00–14:47, 7 turns)
  files_touched:     src/auth/{login,session,callback}.ts
  key_decisions:     NextAuth over manual JWT (turn 4)
  rejected:          manual JWT approach (turn 3) — "too much token refresh boilerplate"
  code_events:       17 total (14 accepted, 3 rejected)
```

commit message 写了「Add OAuth」，但是「为什么 NextAuth 不是 manual JWT」
这件事，只有 Drift AI 记得。

### 场景 3：审计与合规（`drift log`）

> 「这次 release 里，有哪些 code 是 AI 生成、哪些是人类写的？」

```
$ drift log v0.3.0..v0.4.0
commit 7f8a12b — feat: add webhook retries
  💭 [codex]      4 turns, exponential backoff design
  ✋ [human]      1 manual edit  (src/webhooks/retry.ts L88)

commit 2b4c5d9 — fix: race in session cache
  💭 [claude-code] 2 turns, spotted the TOCTOU
  💭 [claude-code] 1 turn,   applied the fix
```

对需要追踪「AI contribution 比例」的团队（合规、education、研究），这是
**唯一不靠 token 计数猜测、而是真正绑到 commit** 的记录。

---

## 愿景的技术骨干

三个分层，每层都是可替换的抽象：

### 1. Connector 层（attribution 的原料）

- **Day-one**：Claude Code + Codex 双 first-class，是为了从第一天就压测
  「跨 agent」的抽象 — 如果只支持一个 agent，抽象很容易变 wishful thinking。
- **Aider stub**：示例，CONTRIBUTING 里写「新增 connector」的 walkthrough
  都用 aider 演示。
- 任何未来的 AI CLI（Cursor CLI、Cline、一个自建的 agent...）都可以通过
  实现 `SessionConnector` trait 接进来。

### 2. Compaction 层（可读性）

- 原始 session JSONL 动辄 500K 一份，人类无法读。LLM compaction 把它压到
  ~1K 的 markdown（frontmatter + 决策摘要 + 被驳回的思路）。
- **Provider-agnostic**：Anthropic 是默认，`MockProvider` 跑测试；未来可接
  OpenAI、本地 Ollama、任何符合 `CompactionProvider` 接口的实现。
- Compacted 结果 commit 进 `.prompts/sessions/` — **随 repo 移动，永远可读**，
  不依赖 Drift AI 仍然存在。

### 3. Attribution 层（核心差异化）

- **`CodeEvent`**：每次文件变动一条，带 `diff_hunks`、`parent_event_id`、
  `content_sha256_after`、`rejected`、`rename_from`。
- **SHA-256 ladder**：监测「AI 写完之后，人类把 SHA 改了」的信号 — 这是
  Drift AI 唯一能诚实检测人类手改的方法，不假装做 authorship judgement。
- **Rename 两层策略**：session tool call lexer → `git log --follow` fallback。
- **Git notes binding**：`refs/notes/drift` 把 compaction 结果绑到 commit；
  `events.db` 装 CodeEvent，config 决定进不进 git。

---

## 范围边界 — Drift AI **不做**的事

刻意不做、不是忘了做：

| 不做 | 理由 |
|---|---|
| 云端 SaaS 仪表板 | 违反 local-first。未来若需要团队界面，应是第三方 web UI 读取 `.prompts/` + notes，而非 Drift AI 自己长出云端服务。 |
| 判定「某行的作者是谁」 | SHA 只能告诉我们「谁没做这个变动」（AI session 之外的人做的），不能告诉我们「实际敲键盘的是谁」。Drift AI 的 `human` slug 只表语义「非 AI session 产生」，不主张作者身份。 |
| 量化「AI 贡献比例」 | 行数比例会被简单操纵（format、rename 都会放大）。我们提供原始 event timeline；任何衡量都交给上层工具按自己的定义算。 |
| 在 AI session 里当 middleware 拦/改 prompt | Drift AI 是 **被动观察者**，不拦截、不修改、不代理。所有上游 agent 照原样运作；我们只读完成后的记录。 |
| 取代 git 或 git-blame | 我们 **延伸** git，靠 git notes 叠加一层，不取代底层。卸掉 Drift AI，git 操作一如往常。 |

---

## 成功的长期样貌

1. **生态兼容**：Claude Code / Codex / Aider / Cursor CLI / 未来某个新 agent
   上线那一周，社区就有对应 connector PR 进来。因为 Connector trait 干净、
   CONTRIBUTING 有示例。
2. **Team-blame**：多人共同开发的 repo，每个人的 AI session 都各自被 Drift AI
   捕捉，`drift sync push/pull` 让 notes 跨机器流通，**任何人的 `drift blame`
   都看到完整团队 timeline**（含隐私模式：`db_in_git = false` 时只存本机）。
3. **成为新进工程师的第一个 command**：不是 `git log`，而是
   `drift blame <某个让人困惑的 function>` — 因为它直接给出「这段为什么
   这样写」的完整对话脉络。
4. **合规与研究的标准格式**：AI-generated code 的审计、教学研究、contribution
   量测都用 `.prompts/` + `refs/notes/drift` 作为 input schema。因为这是第一个
   真正把 AI session 绑到 commit 的开源格式。

---

## 与 v0.1.0 的关系

这份文档描述的是 **北极星**。v0.1.0 要证明的是这条路线 **技术上走得通**：

- ✅ 双 agent 跨越抽象（不是 single-agent hack）
- ✅ Line-level（不是 commit-level 敷衍）
- ✅ 数据模型撑得住 4 个非 trivial 场景（multi-origin、human edit、rejected、
  rename）— 这是 Phase 0 的 [自我评估](../../PHASE0-PROPOSAL.md#f-self-evaluation-does-the-data-model-honor-the-four-requirements)
- ✅ `drift blame` 的 demo 真的能跑

v0.1.0 不会有：team-sync 的精细 UI、Cursor connector、web 界面、贡献量测。
那些是 v0.2+ 的事，确定骨干站得住才加。

---

## 为什么现在做这件事

- AI coding tool 生态 2025-2026 年快速收敛为「Claude Code / Codex 双强 + 长尾」，
  schema 相对稳定，适合下第一份跨 agent 抽象。
- `git notes` 作为底层机制在 git 2.40+ 之后 merge-friendly、push/pull 成熟。
- SQLite + 单 binary 分发让 local-first daemon 的工程成本历史新低。
- 还没有任何开源工具认真做「AI-native blame」 — Mem0 / Supermemory 都在做
  memory 层，不是 attribution 层；那是完全不同的问题。

时机、技术、需求三者都对上了。v0.1.0 要在这个窗口把骨干立起来。
