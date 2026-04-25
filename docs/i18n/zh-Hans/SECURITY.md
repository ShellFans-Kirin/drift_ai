> 🌐 [English](../../SECURITY.md) · [日本語](../ja/SECURITY.md) · **简体中文** · [繁體中文](../zh-Hant/SECURITY.md)

# 安全与隐私

Drift AI 是一个 local-first 的工具。它捕捉的数据 — 你的 AI coding session — 可
能包含你不想 commit 进 repo 的东西。这份文档诚实地把 threat model 写清楚。

## Threat model

Drift **没有 server**。Drift 自身不会把任何东西上传到任何地方。数据流是：

1. 你的 AI agent（Claude Code、Codex…）把 session JSONL 写进它自己在
   `~/.claude/projects/` 或 `~/.codex/sessions/` 下的目录。这是 agent 的行为，
   不是 Drift 的。
2. `drift capture` 读那些文件，并写出：
   - `code_events` rows 进 `<repo>/.prompts/events.db`（SQLite）
   - 每个 session 对应一份 Markdown 进 `<repo>/.prompts/sessions/`
3. 如果 `[compaction].provider = "anthropic"`（默认），`drift capture` 会把每份
   session transcript 发到 `api.anthropic.com/v1/messages` 来产生 Markdown
   摘要。**这是唯一的 network egress。** 切到 `provider = "mock"` 可以完全
   跳过这一步。

第 3 步以外的所有东西都留在你机器上。

## 当前的限制（v0.1.x）

下列是**已知**且**已被记录**的，不是 bug：

1. **`drift capture` 不会 scrub session 内容。** 你打进 Claude / Codex chat 的
   任何内容 — 包括手滑贴进去的 secret — 会原样镜像到 `events.db` 跟
   `.prompts/sessions/*.md`。
2. **`events.db` 默认会被 commit 进 git**（`[attribution].db_in_git = true`）。
   这个默认值的本意是让 team 共享 blame；副作用是 session 里漏出的 secret 会
   进 public repo。
3. **`.prompts/sessions/*.md` 是人类可读的**：compacted 摘要保留文件名、决策、
   而且常常逐字保留 diff hunk。Anthropic 的 compactor 也不会主动 redact secret。

如果你曾经把 `export AWS_SECRET_ACCESS_KEY=AKIA...` 之类的内容贴进 Claude
session，那串字符串就会落到 `events.db`，也可能落进 compacted Markdown。

## 当前可用的 mitigation

挑一个最适合你 workflow 的：

1. **关掉 git 那一侧**：
   ```toml
   # .prompts/config.toml
   [attribution]
   db_in_git = false
   ```
   `events.db` 跟 markdown 都只留本机。Team 失去共享 blame；你保有本地 view。

2. **commit 前手动 review**：
   ```bash
   drift capture
   git diff --cached -- .prompts/
   # 真的把摘要读过一遍；需要 redact 就直接改文件
   git add .prompts/ && git commit
   ```

3. **配 secret scanner 当 pre-commit hook**。Drift 不附带，但
   [gitleaks](https://github.com/gitleaks/gitleaks) 跟
   [trufflehog](https://github.com/trufflesecurity/trufflehog) 能抓到大部分
   pattern。示例：
   ```bash
   # .git/hooks/pre-commit
   gitleaks protect --staged --redact -v || exit 1
   ```

4. **完全离线跑**：设 `[compaction].provider = "mock"` 并 unset
   `ANTHROPIC_API_KEY`。你会失去 LLM 摘要，但 `events.db` 还能纯粹当本地索引
   使用。

5. **如果预期会把 secret 贴进 chat，先别在那个 repo 上启用 Drift**，等 v0.2
   的 redaction pass 落地。等一下不丢脸。

## Roadmap（v0.2+）

下列是 planned work，不是有日期的承诺：

- **`drift capture` 内 regex-based redaction pass** — 识别高信心度的 pattern
  （Anthropic / OpenAI / AWS / GitHub PAT / Slack / private key blob）并替换成
  `<redacted>` placeholder，在 `events.db` 之前就替换掉。
- **交互式 review mode**：`drift capture --review` 把每份产生的 Markdown 开到
  `$EDITOR` 确认后才落盘。
- **可插拔 detector**：可选的 `trufflehog` / `gitleaks` 规则集成，不重新发明
  regex。
- **`drift redact <session-id>`**：对已 capture 的 session 做事后 scrub，带
  明确的 undo 路径。

如果你需要其中任何一项提前实现，开 feature request — 真实 use case 会推
高优先级。

## 上报安全问题

任何可能导致 credential 泄漏、supply-chain 风险、或 RCE 的问题，请走
[GitHub Security Advisories](https://github.com/ShellFans-Kirin/drift_ai/security/advisories/new)。
不要开 public issue。

文档补完、threat model 修正、「你漏了 mitigation X」之类的 — 一般 issue 或
PR 就行。
