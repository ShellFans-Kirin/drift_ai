> 🌐 [English](CHANGELOG.md) · [日本語](CHANGELOG.ja.md) · **简体中文** · [繁體中文](CHANGELOG.zh-Hant.md)

# Changelog

drift_ai 的所有重大改动都记在这里。
格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)；
版本号遵循 [SemVer](https://semver.org/spec/v2.0.0.html)。

## [0.2.0] — 2026-04-25

「你不会被绑死在单一 LLM vendor」的 release。新增 **`drift handoff`** —
新的招牌命令 — 跟一份 v0.2 风格的 README，把 task transfer 推到最前面、把
blame 降为 supporting feature。

### Added

- **`drift handoff` CLI**。把进行中的 task（用 `--branch`、`--since`、或
  `--session` 过滤）打包成一份下一个 agent 能冷读的 markdown brief。Flag：
  `--to claude-code|codex|generic`、`--output <path>`、`--print`。默认输出：
  `.prompts/handoffs/<YYYY-MM-DD-HHMM>-<branch>-to-<agent>.md`。
- **`crates/drift-core/src/handoff.rs`** — orchestrator + 4 个小 collector
  （sessions、events-by-file、rejected approaches、file snippets）+ LLM 二段
  pass + 纯 Rust 的 `render_brief`。新单元测试覆盖 scope parsing、snippet
  extraction（full vs. modified-range 摘录）、JSON-from-LLM parsing（容忍
  code-fence + 周边叙述）、以及 per-`--to` footer 渲染。新增 15 个 test。
- **`crates/drift-core/templates/handoff.md`** — 二段 pass 的 LLM prompt
  template；要求 model 输出含 `summary` / `progress` / `key_decisions` /
  `open_questions` / `next_steps` 的 JSON。
- **`AnthropicProvider::complete_async`** + 同步 `complete` — 通用的
  system+user → text 补全，重用 `compact_async` 的 retry / streaming /
  token-usage 机制给需要不同 prompt 形状的 caller 用（handoff 用）。返回
  新的 `LlmCompletion` struct（text + per-call token / cost）。
- **`[handoff]` config section** 在 `.prompts/config.toml` 里。默认 model
  `claude-opus-4-7`。默认选 Opus — handoff brief 是 user-facing 产物，下一个
  agent 会逐字读，叙事质量就是 value，handoff 频率本来就低（一个工作日通常
  一两次）。要 ~30× 成本下降可以切 Haiku。
- **30 秒 demo** 在 `docs/demo/v020-handoff.gif`（用 fixture data 对着
  `drift handoff` 真实录制；`docs/demo/v020-handoff.cast` 是原 cast 文件）。
- **真实 Anthropic smoke 输出**收在
  [`docs/V020-SMOKE-OUTPUT.md`](docs/V020-SMOKE-OUTPUT.md)。
- **`docs/V020-DESIGN.md`** — Phase 0 设计提案，留 repo 里作为 `drift handoff`
  形状的参考。

### Changed

- README 第一屏改 `drift handoff` 为招牌，hero 位放 demo GIF。blame / log
  保留为「supporting feature」放在同一屏作为 reference。
- Quickstart 从 5 个命令增至 6 个（加了 `drift handoff`）。
- About section 加一句 dogfood 出身的小注脚。
- 预编 binary 安装 URL 升到 `drift-v0.2.0`。

### v0.1 带过来的稳定性保证

- `events.db` schema **不变**。从 v0.1.x 升上来纯粹是 binary 替换；不需要
  migration。
- MCP tool 列表**不变**。已有的 MCP client 照常运作。
- `SessionConnector` trait **不变**。已有 connector 照常运作。
- v0.1.2 的 first-run privacy notice 仍在第一次跑 `drift capture` 时触发；
  handoff 不需要重新 acknowledge。

### 已知限制（v0.2）

- `--branch <name>` scope 是 best-effort：它跑 `git log <branch> --not main
  --format=%aI` 找最早分歧的 commit 当 lower-bound filter。同一天落在多个并
  行 branch 的 session 可能会交叠 — 用 `--since` 收敛。
- handoff 的 LLM call 跟任何 Opus call 同样 cost profile（每份 brief ~$0.10）。
  重度使用的话设 `[handoff].model = "claude-haiku-4-5"`。
- 还没有 `drift handoff list` / `drift handoff show <id>` — 产生的 brief 就是
  `.prompts/handoffs/` 下的 markdown 文件。`ls` 跟 `cat` 是 v0.2 的查询接口。

## [0.1.2] — 2026-04-25

盖在 v0.1.1 上的文档 + 文案 patch。compaction / attribution / MCP 的 code
path 跟 v0.1.1 完全相同；唯一行为变动是用户第一次跑 `drift capture` 时的
一次性 privacy notice。

### Added
- **`docs/SECURITY.md`** — threat model、当前限制、可用 mitigation
  （db_in_git toggle、手动 review、gitleaks/trufflehog pre-commit）、
  v0.2 roadmap（regex redaction pass、交互式 review mode、`drift redact`
  事后 scrub）、安全披露通道。
- **README `## Privacy & secrets` section** — 直接、不软销、明确披露
  `drift capture` 会把 session content 镜像进 `.prompts/`，且默认把
  `events.db` commit 进 git。
- **`drift capture` 第一次的 notice** — 第一次调用会打印一段 privacy 立场
  提醒并等 stdin。`DRIFT_SKIP_FIRST_RUN=1` 跳过（CI-friendly）。状态记在
  `~/.config/drift/state.toml::first_capture_shown`。
- **`docs/COMPARISON.md`** — 对 Cursor / Copilot chat / Cody / `git blame`
  的功能比较。从 README 链过来。
- **README 痛点开场** — 一段（"47 prompts to Claude + 3 Codex fills + 12
  manual edits ..."）放在技术描述上方。
- **README `## About` section** — 明确声明 drift 是独立项目，不隶属于
  Anthropic、OpenAI 或任何 agent vendor。
- **README badges**：crates.io 版本 + CI 状态（限两个）。
- **Provider-switching 示例** 在 `## Configuration` 提到 v0.2 计划
  （ollama / vllm / openai-compatible）。

### Tests
- `tests/first_run_notice.rs` 涵盖 `DRIFT_SKIP_FIRST_RUN=1` 的 bypass 跟
  state-file persistence 路径。

### v0.1.1 带过来的已知限制
- Drift 仍然不主动 redact secret — 那是 v0.2 的事。
- 计价表是 hardcoded；当作正式 invoice 之前请核对 Anthropic 的 public
  pricing。

## [0.1.1] — 2026-04-23

### Added
- **Live Anthropic compaction.** `AnthropicProvider` 现在真的会打
  `POST /v1/messages?stream=true`，消费 SSE stream，CLI 跑时把 content
  delta echo 到 stderr，并在 `message_stop` 解析 `usage` block 做 billing。
- **Typed compaction error**（`CompactionError`）：`AuthInvalid`、
  `RateLimited { retry_after }`、`ModelNotFound`、`ContextTooLong`、
  `TransientNetwork`、`Stream`、`Other`。每个 variant 对应一个独立的、
  operator 看得到的 CLI 信息。
- **Model 切换靠 config**：`[compaction].model` 接受 `claude-opus-4-7`
  （默认）、`claude-sonnet-4-6`、`claude-haiku-4-5`。
- **Retry policy**：429 跑 5 次并遵守 `Retry-After`；5xx 跑 4 次配指数
  backoff（1s → 2s → 4s → 8s）；401/404 直接失败。
- **Context-window 截断**：char-based token 估计 + 80% 阈值；Strategy 1
  保留 head(8) + tail(8) turn，中间用明确 marker 省略。
- **`compaction_calls` table**（SQLite migration v2）：per-call 的
  input / output / cache-creation / cache-read token 数量加上算好的 USD
  cost。
- **`drift cost`** CLI：`--since <iso>` / `--until <iso>` /
  `--model <name>` / `--by model|session|date`。
- **`drift watch` 是 event-driven**：用 `notify`
  （FSEvents/inotify/ReadDirectoryChangesW）支撑，200ms debounce、按文件名
  推导出 session_id 来 per-session capture，状态存 `~/.config/drift/watch-state.toml`，
  SIGINT/SIGTERM 收尾完当前 capture 才退出。
- **Homebrew tap 上线**：`brew install ShellFans-Kirin/drift/drift` 对着
  公开的 [homebrew-drift](https://github.com/ShellFans-Kirin/homebrew-drift)
  tap；formula 每次 release 都通过 `release.yml` 的 `repository_dispatch`
  自动 regenerate。
- **发布到 crates.io**：`drift-core`、`drift-connectors`、`drift-mcp`、
  `drift-ai`。

### Changed
- `CompactionProvider::compact` 现在返回 `CompactionResult`
  （summary + 可选 usage）而不是只有 `CompactedSummary`，让 live provider
  能把 billing data round-trip 回来。
- `drift init` 是 idempotent：再跑不会覆盖已存在的 `config.toml`。
- `drift capture` 对单个 session 的 compaction error 采 soft-fail（log + 跳过），
  一个 oversized session 不会中断整批。
- `summary_to_markdown` 现在会输出真正的 section heading（`## Summary`、
  `## Key decisions`、`## Files touched`、`## Rejected approaches`、
  `## Open threads`），取代原本一行的 `[MOCK]` blurb。

### Fixed
- Workspace 内部依赖钉在 0.1.1（之前是 0.1.0），让 `cargo publish` 能对
  crates.io 解析。
- 不小心 check-in 进来的 ship-time smoke `.prompts/events.db` 现在会被
  ignore；`.prompts/` 加进 `.gitignore` 给干净 clone 用。

### Known limitations
- Context-window Strategy 2（分层 summarization）骨架完成但 feature flag
  关闭。默认行为是 Strategy 1。
- Cost 总额用 hardcoded 计价表（对着 Anthropic 截至 2026-04-23 的 public
  pricing 对过）；当 billing report 用之前再对着
  <https://www.anthropic.com/pricing> 核一次。

## [0.1.0] — 2026-04-22

### Added
- Cargo workspace 含四个 crate：`drift-core`、`drift-connectors`、
  `drift-cli`（binary：`drift`）、`drift-mcp`。
- Claude Code + Codex 的 first-class connector；Aider stub 在 feature flag
  后面（`aider`）。
- Attribution engine：`CodeEvent` row 落在 `.prompts/events.db`（SQLite），
  人类编辑检测用 SHA-256 ladder，rename 两层处理（session tool call +
  git-log-follow fallback），MultiEdit intra-call parent chain。
- Compaction engine 含 `MockProvider`（v0.1.0 默认，标 `[MOCK]`）跟一个
  `AnthropicProvider` skeleton（HTTP 调用在 v0.1.1 接通）。
- CLI：`init`、`capture`、`watch`、`list`、`show`、`blame`、`trace`、
  `diff`、`rejected`、`log`、`bind`、`auto-bind`、`install-hook`、
  `sync push/pull`、`config get/set/list`、`mcp`。
- Git notes 集成（`refs/notes/drift`）：手动 binding、按 timestamp 自动
  binding、non-blocking 的 post-commit hook。
- Stdio MCP server 含 5 个只读 tool：`drift_blame`、`drift_trace`、
  `drift_rejected`、`drift_log`、`drift_show_event`。
- Plugin skeleton（`plugins/claude-code/`、`plugins/codex/`）— v0.1.0 没
  publish；v0.2 才上 marketplace。
- CI（`.github/workflows/ci.yml`）跟 release（`release.yml`）矩阵覆盖
  Linux x86_64/aarch64 + macOS x86_64/aarch64。
- Apache-2.0 授权，CONTRIBUTING 走过新增 connector 流程，code-of-conduct。

### 已知限制
- Anthropic compaction HTTP call 还是 stub。Mock 是 shipping 默认；接通的
  说明留在 `crates/drift-core/src/compaction.rs`。
- 人类编辑检测只到 timeline — 不主张作者身份。
- Codex 的 `reasoning` item 是加密的；只计数，不 surface。
- `drift watch` 是 debounced polling daemon；v0.2 改成完全 event-driven。
- `cargo publish` 这次 cut 没跑；`0.1.1` 的 Cargo.toml metadata 都备齐了。
