# 掘金草稿 — v0.4.0(简体中文,DeepSeek 角度)

# Drift v0.4 发布:任意 AI 编程 agent + 任意 LLM 之间的无损 handoff

> 最大亮点:同一份 handoff brief,DeepSeek 比 Anthropic Opus 便宜 30 倍,叙事质量基本看不出差。

## 痛点

我每天写代码大约这样:用 Claude Code 起一个 feature,做到一半被 rate limit
挡下,或 context window 塞满,不得不切到 Codex / Cursor / 自建 Agent 接手。

切的那一刻最痛:

- 新 agent 不知道我已经决定用 SQLite 而不是 Postgres
- 也不知道我之前试过 token bucket 但已经放弃
- 我把 chat 整段贴过去 — 信息太杂、agent 又把已经定案的设计拉出来重新讨论
- 十分钟过去,还在重新解释,根本没在写 code

更别说一周后 review commit,我看不出哪几行是哪个 agent 写的、哪些是我后改的、为什么最终是这个写法。

## drift 做什么

drift 是一个本地 CLI(Rust 写的、单 binary)。它在背景读你 AI agent 已经
写到本地的 session log:

- Claude Code → `~/.claude/projects/`
- Codex → `~/.codex/sessions/`
- Cursor → `~/Library/Application Support/Cursor/...` 的 SQLite
- Aider → `<repo>/.aider.chat.history.md`

每个 session 用 LLM 压成一份小 markdown 摘要,存进你 git repo 里的
`.prompts/` 目录,用 `git notes` 绑到对应 commit。

然后 `drift handoff --branch feature/x --to claude-code` 给你一份**下一个
agent 能冷读的 brief**:做完了什么、还开着什么、试过哪些已经被驳回、从哪
里接。贴到下一个 agent,接着写。

## v0.4 的发布重点

**Multi-agent**:Cursor + Aider 这次一并支持。Cursor 是 reverse-engineering
Cursor 的 `state.vscdb` SQLite 直接读;Aider 是 parse `.aider.chat.history.md`
markdown 加上 `git log --grep="^aider:"` 关联 commit。

**Multi-LLM provider**:driving handoff 那次 LLM 调用的 provider 现在可以
切换:

- 原生:Anthropic / OpenAI(gpt-5/4o/o1/o3)/ Gemini / Ollama(本地)
- OpenAI-compatible 通用:DeepSeek / Groq / Mistral / Together AI / 还有
  自架的 vLLM、LM Studio

切换只需要改 `.prompts/config.toml` 一行 `provider = "deepseek"`。

## DeepSeek 实测数据

同一份 4-turn 的 fixture session(实现 sliding window rate limiter),四家
跑同样的 handoff brief 生成:

| Provider | Model | 延迟 | 输入/输出 token | 成本(USD) |
|---|---|---:|---:|---:|
| Anthropic | claude-haiku-4-5 | 2281 ms | 435 / 179 | $0.00133 |
| OpenAI | gpt-4o-mini | 3201 ms | 391 / 147 | $0.00015 |
| Gemini | gemini-2.5-flash | 1505 ms | 455 / 199 | $0.00019 |
| **DeepSeek** | **deepseek-chat** | **1906 ms** | **396 / 109** | **$0.00023** |

DeepSeek 比 Anthropic Opus(同一家最贵 model)便宜约 30 倍,叙事质量**没
有可见差距** — 因为 handoff brief 本质是一份「把 session 压成 1-2 段的摘
要」,不是深度 reasoning,不需要 frontier model。

完整 smoke 报告:[docs/V030-V040-SMOKE.md](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-SMOKE.md)

## 安装

```bash
# Homebrew
brew install ShellFans-Kirin/drift/drift

# crates.io(需要 Rust 1.85+)
cargo install drift-ai

# 切到 DeepSeek
echo '
[handoff]
provider = "deepseek"

[handoff.providers.deepseek]
type = "openai_compatible"
base_url = "https://api.deepseek.com"
model = "deepseek-chat"
api_key_env = "DEEPSEEK_API_KEY"
cost_per_1m_input_usd = 0.27
cost_per_1m_output_usd = 1.10
' >> .prompts/config.toml

export DEEPSEEK_API_KEY=sk-...
drift handoff --branch feature/x --to claude-code
```

## 局限

drift 不主动 redact secret — 你打进 chat 的密钥会被镜像进 `.prompts/`。如果你的工
作流常会贴 secret 进 chat,等 v0.5 的 redaction pass(开发中)再启用,或者
配 `[attribution].db_in_git = false` 保持只在本机。

Cursor 的 schema 是反向工程出来的,Cursor 改格式时 connector 可能要发 patch。

## 仓库

- 代码:https://github.com/ShellFans-Kirin/drift_ai
- 设计文档:[docs/V030-V040-DESIGN.md](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-DESIGN.md)

Apache 2.0,独立项目,不隶属于任何 LLM 厂商。Issue 跟改进 PR 都欢迎。
