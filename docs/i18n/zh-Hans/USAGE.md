> 🌐 [日本語](../ja/USAGE.md) · **简体中文** · [繁體中文](../../USAGE.md)

# Drift AI · 使用方式（v0.2.0+）

完整使用手册。从零安装到 `drift handoff`，每一步都附可复制的命令 + 示例输出。

---

## 0. 先决条件

- macOS（Apple Silicon 或 Intel）/ Linux（x86_64 或 aarch64）
- 一个 git repo（drift 操作的目标）
- 至少一个 AI coding agent 的 session log:
  - Claude Code → 默认写到 `~/.claude/projects/`
  - Codex → 默认写到 `~/.codex/sessions/`
  - Aider → connector 是 stub，社区 PR 欢迎
- 想用 LLM compaction / handoff 的话：`ANTHROPIC_API_KEY` 环境变量。否则 drift 会 fall back 到 deterministic mock summary（标 `[MOCK]`）

---

## 1. 安装

### 1.1 Homebrew（macOS arm64/x86_64 + Linuxbrew）

```bash
brew install ShellFans-Kirin/drift/drift
```

### 1.2 crates.io（需要 Rust 1.85+）

```bash
cargo install drift-ai
```

注意：crate 名是 `drift-ai`，安装后的 binary 名是 `drift`。

### 1.3 GitHub Releases pre-built tarball（不需要 Rust toolchain）

```bash
curl -sSfL https://github.com/ShellFans-Kirin/drift_ai/releases/latest/download/drift-v0.2.0-$(uname -m)-unknown-linux-gnu.tar.gz \
  | tar xz -C /tmp && sudo mv /tmp/drift /usr/local/bin/drift
drift --version
# expect: drift 0.2.0
```

替换 `unknown-linux-gnu` 为 `apple-darwin` 给 macOS。

### 1.4 从源码

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo install --path crates/drift-cli
```

### 验证

```bash
drift --version          # drift 0.2.0
drift --help             # 列出全部 17 个 subcommand
```

---

## 2. 第一次设置

### 2.1 在你的 git repo 里 init

```bash
cd /path/to/your/repo
drift init
```

会建立：

- `.prompts/` 目录
- `.prompts/config.toml`（默认值；你可以后续编辑）
- `.prompts/.gitignore`（让 cache 不进 git）

`drift init` 是 idempotent — 重复跑不会覆盖你已编辑的 `config.toml`。

### 2.2 第一次 `drift capture` 会打印 privacy 注意事项

这是 v0.1.2 加的 first-run notice：

```
drift capture · first-run notice
  drift mirrors your AI session content (including anything you
  pasted) into .prompts/. events.db is committed to git by default.
  See docs/SECURITY.md for the full story.

  Press Enter to continue, Ctrl-C to abort.
```

按 Enter 继续。`~/.config/drift/state.toml` 记住「已显示过」，之后不会再问。

CI / 自动化场景用 `DRIFT_SKIP_FIRST_RUN=1` env var 跳过（但**不会**标记成已显示，下次交互执行还会再问一次）。

---

## 3. 抓取 session（capture）

### 3.1 一次性

```bash
drift capture
# 预期: Captured N session(s), wrote M event(s) to .prompts/events.db
```

`drift capture` 会：

1. 扫描 `~/.claude/projects/` + `~/.codex/sessions/` 下所有 jsonl
2. 对每个 session 抽 `CodeEvent` 并写进 `.prompts/events.db`（SQLite）
3. 对每个 session 跑 LLM compaction 写成 `.prompts/sessions/<date>-<agent>-<short_id>.md`

### 3.2 Filter

```bash
drift capture --agent claude-code              # 只抓 Claude Code
drift capture --agent codex                    # 只抓 Codex
drift capture --session abc12345-xxx           # 只抓特定 session id
drift capture --all-since 2026-04-22T00:00:00Z  # 只抓某时间之后
```

### 3.3 Live mode（背景 daemon）

```bash
drift watch
# drift watch · event-driven; Ctrl-C to stop
#   watching /home/you/.claude/projects
#   watching /home/you/.codex/sessions
```

`drift watch` 用 platform 原生 FS event（FSEvents on macOS / inotify on Linux），每次 session 文件变动 200 ms debounce 内 re-capture。`Ctrl-C` 收尾当前 capture 后干净退出。状态存到 `~/.config/drift/watch-state.toml`，restart 后 resume 而非全扫。

---

## 4. 🌟 v0.2 新功能：`drift handoff`

把进行中的 task 打包成另一个 agent 接得起来的 markdown brief。

### 4.1 用法 — 三种 scope（择一）

```bash
# 用 git branch 圈定范围（最常用）
drift handoff --branch feature/oauth --to claude-code

# 用时间范围（没有专属 branch 时）
drift handoff --since 2026-04-25T08:00:00Z --to codex

# 单一 session（debugging / unit test）
drift handoff --session abc12345-xxx --print
```

### 4.2 Target agent

```bash
--to claude-code     # 默认；footer 配 'paste this to claude' 提示
--to codex           # footer 配 codex 惯用语气
--to generic         # 纯 brief 没 footer，pipe 到任何工具
```

差别只在 footer。Body 一致 — body 是 task 的内容，目前不刻意做 per-vendor 翻译（v0.3+ 会做 tool-call schema adapter）。

### 4.3 Output

```bash
drift handoff --branch feat-x --to claude-code
# → .prompts/handoffs/2026-04-25-1530-feat-x-to-claude-code.md

drift handoff --branch feat-x --to claude-code --output ~/transfer.md
# → ~/transfer.md

drift handoff --branch feat-x --to claude-code --print | pbcopy
# → stdout，再 pipe 到 clipboard
```

### 4.4 预期执行流程

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

### 4.5 Brief 结构

每份 brief 都有同样的 section：

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

### 4.6 成本

| Model | 每次 handoff 大致成本 |
|---|---|
| `claude-opus-4-7` (默认) | ~\$0.10–0.30 |
| `claude-sonnet-4-6` | ~\$0.02–0.06 |
| `claude-haiku-4-5` | ~\$0.005–0.01 |

切换在 `.prompts/config.toml`：

```toml
[handoff]
model = "claude-haiku-4-5"
```

默认 Opus 是因为 brief 是下个 agent 逐字读的内容，narrative quality 比 per-session compaction 重要。每天 handoff 个位数次的话 Opus 一个月 ~\$30；高频使用建议切 Haiku。

---

## 5. AI-native blame（v0.1 既有功能）

### 5.1 反查：哪个 session 写了这行？

```bash
drift blame src/auth/login.ts
# 整个文件的时间线

drift blame src/auth/login.ts --line 42
# 单行的时间线

drift blame src/auth/login.ts --range 40-60
# 行范围的时间线
```

示例输出：

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

### 5.2 顺查：这个 session 改了什么？

```bash
drift trace abc12345-xxx
# 列出此 session 产生的所有 CodeEvent
```

### 5.3 整体 audit log

```bash
drift log
# 像 git log 但每个 commit 多一段 per-agent attribution

drift log -- --since 1.day
# 把后面的 args 透传给 git log
```

示例：

```
commit abc1234 — Add OAuth login
   💭 [claude-code] 7 events accepted, 0 rejected
   💭 [codex]       3 events accepted, 1 rejected
   ✋ [human]       2 manual edits
```

### 5.4 看单个事件

```bash
drift diff <event-id>      # 显示这个 event 的 unified diff
drift show <session-id>    # 显示 session 的 compacted markdown
drift list                 # 列出所有 captured sessions
drift list --agent codex   # 只列 codex sessions
```

### 5.5 看被驳回的 AI 建议

```bash
drift rejected
drift rejected --since 2026-04-22T00:00:00Z
```

`rejected` event 来源：session 里 tool_result 标 `is_error=true` 的那些。

### 5.6 把 session 跟 git commit 绑定

```bash
drift bind <commit-sha> <session-id>     # 手动绑
drift auto-bind                           # 自动按 timestamp 配对
drift install-hook                        # 安装 post-commit hook 自动跑 auto-bind
```

绑定数据写在 `refs/notes/drift`（git notes），不污染 commit history。

---

## 6. MCP server（给其他 AI tool 查 drift）

```bash
drift mcp
# 启动 stdio MCP server
# 默认 tools: drift_blame / drift_trace / drift_rejected / drift_log / drift_show_event
```

注册到 Claude Code（一行）：

```bash
claude mcp add drift -- drift mcp
```

注册到 Codex：

```bash
codex mcp add drift -- drift mcp
```

之后可以在 Claude / Codex 对话中直接问「show me the drift blame for src/foo.rs:42」，他们会通过 MCP 调用 drift 查 attribution，不需要切到 shell。

MCP 接口是**只读**的 by design — 任何写入动作（capture / bind / sync）都只在 CLI 提供。

---

## 7. 账务透明：drift cost

`drift handoff` 跟 `drift capture` 的 LLM 调用都记到 `events.db` 的 `compaction_calls` 表：

```bash
drift cost
# drift cost — compaction billing
#   total calls      : 10
#   input tokens     : 120958
#   output tokens    : 6582
#   total cost (USD) : $0.1539
```

更细的分组：

```bash
drift cost --by model
drift cost --by session
drift cost --by date
drift cost --since 2026-04-20T00:00:00Z --until 2026-04-25T00:00:00Z
drift cost --model claude-haiku-4-5
```

---

## 8. 设置（`.prompts/config.toml`）

完整模板：

```toml
[attribution]
db_in_git = true             # 默认 true，events.db 进 git。改 false 留本机。

[connectors]
claude_code = true
codex = true
aider = false                # feature-flag 的 stub

[compaction]
provider = "anthropic"       # 默认；改 "mock" 完全离线
model = "claude-haiku-4-5"   # 或 claude-sonnet-4-6 / claude-opus-4-7

[handoff]
model = "claude-opus-4-7"    # narrative quality 重要；可切 haiku 省 30×

[sync]
notes_remote = "origin"      # `drift sync push/pull` 用的 remote
```

两层：

- 全局：`~/.config/drift/config.toml`
- 项目（覆盖）：`<repo>/.prompts/config.toml`

```bash
drift config get handoff.model
drift config set handoff.model claude-haiku-4-5
drift config list
```

---

## 9. 跨机器同步（git notes）

```bash
drift sync push origin   # 推 refs/notes/drift 到 remote
drift sync pull origin   # 从 remote 拉 refs/notes/drift
```

drift 的 attribution 链接（哪个 session 对应哪个 commit）存在 `refs/notes/drift`。Push / pull 用 git 自带的 notes 机制，不会跟 main code 的 commit 对到一起。

---

## 10. 安全 / Privacy

drift **不会** scrub session content。任何你贴进 Claude / Codex chat 的内容（包括手滑贴上的 secret）会进 `.prompts/sessions/*.md` 跟 `events.db`，默认这些都会被 commit 到 git。

三条 mitigations：

1. **关闭 git side**：
   ```toml
   [attribution]
   db_in_git = false
   ```
   `events.db` 跟 markdown 留本机，team 失去共享 blame，你保有本地 view。

2. **手动 review 再 commit**：
   ```bash
   drift capture
   git diff --cached -- .prompts/
   git add .prompts/ && git commit
   ```

3. **配 secret scanner pre-commit hook**（drift 不附带）：
   ```bash
   # .git/hooks/pre-commit
   gitleaks protect --staged --redact -v || exit 1
   ```

完整 threat model + roadmap 见 [`docs/SECURITY.md`](../../SECURITY.md)。

---

## 11. 进阶：增加新 connector

drift 现支持 Claude Code + Codex；想加 Cursor / Cline / 自己的 agent？

实现 `SessionConnector` trait（在 `crates/drift-connectors/src/lib.rs`）：

```rust
pub trait SessionConnector {
    fn agent_slug(&self) -> &'static str;
    fn discover(&self) -> Result<Vec<SessionRef>>;
    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession>;
    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>>;
}
```

`crates/drift-connectors/src/aider.rs` 是 stub，[`CONTRIBUTING.md`](../../../CONTRIBUTING.md) 用 Aider 当 worked example 走过 add-a-connector 全流程。欢迎 PR。

---

## 12. 完整命令对照表

| Command | 用途 |
|---|---|
| `drift init` | scaffold `.prompts/` |
| `drift capture [--agent A] [--session ID] [--all-since DATE]` | 抓 session、跑 compaction |
| `drift watch` | 背景 daemon，event-driven 自动 capture |
| `drift handoff [--branch B \| --since ISO \| --session ID] [--to A] [--output P \| --print]` | **v0.2** 跨 agent task brief |
| `drift list [--agent A]` | 列出 captured sessions |
| `drift show <id>` | 显示 compacted session |
| `drift blame <file> [--line N \| --range A-B]` | 反查：哪个 session 改了这行 |
| `drift trace <session>` | 顺查：这个 session 改了什么 |
| `drift diff <event>` | 单个 event 的 unified diff |
| `drift rejected [--since DATE]` | 列出被驳回的 AI 建议 |
| `drift log [-- <git args>]` | git log + per-agent attribution |
| `drift cost [--since --until --model --by]` | 账务 |
| `drift bind <commit> <session>` | 手动绑 commit ↔ session |
| `drift auto-bind` | 自动配对 commit ↔ session by timestamp |
| `drift install-hook` | 装 post-commit hook 自动 auto-bind |
| `drift sync push\|pull <remote>` | 同步 `refs/notes/drift` |
| `drift config get\|set\|list` | 读写 config |
| `drift mcp` | 启动 stdio MCP server |

---

## 13. Troubleshooting

### "drift handoff: no sessions matched scope"

`--branch` scope 抓不到 sessions 的常见原因：

1. 你还没 `drift capture` — 跑一次
2. branch 名字不对 — 你的 git branch 是不是 `feat/x` 而不是 `feature/x`？
3. 那个 branch 还没有 commit 上去 — `--branch` 用 git log 找 divergent commit timestamp 当 lower bound；空 branch 会 fall back 到 14 天

退而求其次用 `--since 2026-04-22T00:00:00Z` 试。

### "ANTHROPIC_API_KEY not set — falling back to deterministic mock summary"

handoff 在没设 API key 时会跑 MockProvider，brief 会明显标 `[MOCK]`。设 env var 或在 `.prompts/config.toml` 把 `provider = "mock"` 改回 `"anthropic"`。

### `drift watch` 不触发

- 确认你的 agent 真的在写 jsonl 到 `~/.claude/projects/` 或 `~/.codex/sessions/`
- macOS: `Sandbox / Full Disk Access` 权限可能挡 FSEvents — 给 terminal app 完整磁盘存取
- Linux: `/proc/sys/fs/inotify/max_user_watches` 可能太低 — `sudo sysctl fs.inotify.max_user_watches=524288`

### crates.io 下载失败

- 检查 `crates.io` 帐号 email 已验证
- 确认你用 `cargo install drift-ai`（连字符）而不是 `drift_ai`（下划线）
- 从 `/tmp` 干净环境试一次：
  ```bash
  CARGO_HOME=/tmp/drift-clean cargo install drift-ai --locked
  /tmp/drift-clean/bin/drift --version
  ```

### Homebrew install 找不到 formula

- 确认你 `brew tap` 的是 `ShellFans-Kirin/drift`（大小写敏感与否依 OS / brew 版本而定 — 都试试）
- `brew update` 同步最新 tap state
- macOS Intel runner 已停 — Intel Mac 仍可装（Formula 带 x86_64-apple-darwin tarball）

---

## 14. 相关文档

| 文档 | 内容 |
|---|---|
| [README](../../../README.md) | 30 秒第一印象 |
| [CHANGELOG](../../../CHANGELOG.md) | 版本历史 |
| [docs/VISION.md](VISION.md) | 整个项目的 north star |
| [docs/SECURITY.md](SECURITY.md) | Threat model 完整版 |
| [docs/COMPARISON.md](COMPARISON.md) | vs Cursor / Copilot chat / Cody / git blame |
| [docs/V020-DESIGN.md](../../V020-DESIGN.md) | v0.2 `drift handoff` 设计提案 |
| [docs/V020-DEV-LOG.md](../../V020-DEV-LOG.md) | v0.2 开发周期 + 执行结果完整记录 |
| [CONTRIBUTING.md](../../../CONTRIBUTING.md) | 加新 connector 的逐步指南 |
