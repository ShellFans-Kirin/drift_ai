# Medium / iThome 草稿 — v0.4.0(繁體中文,vendor-neutral handoff 角度)

# Drift v0.4 上線:跨 agent + 跨 LLM 的 AI coding handoff,真正不被廠商鎖定

## 痛點不是工具不夠多

最近 AI coding 工具多到爆 — Claude Code、Codex、Cursor、Aider、各種 IDE
plugin。但每個的 session 紀錄各存一處,切工具的瞬間 context 全丟。

我每天的 workflow 大致這樣:

1. 用 Claude Code 開了個 feature
2. 寫到一半被 rate limit 擋下,或者 model 突然 **變蠢** 了
3. 切到 Codex 或 Cursor,新 agent 不知道我做了什麼決定
4. 把 chat 貼過去 — 噪音太多,新 agent 把已經結案的問題又翻出來討論
5. 一週後 review commit,看不出哪行是哪個 agent 寫的

drift 解決的就是這條斷點。

## drift 是什麼

一個本地 CLI 工具(Rust + 單 binary)。在背景讀你 AI agent 寫到本地的
session log,LLM 壓成 markdown 摘要存進 `.prompts/`,用 `git notes` 綁到
對應 commit。然後:

```bash
drift handoff --branch feature/oauth --to claude-code
```

產出一份下一個 agent 能冷讀的 brief。貼進 Claude Code、Codex、Cursor 任一個,直接接著寫。

## v0.4 兩個維度的 vendor-neutral

**Source(讀哪些 agent 的 session)**:Claude Code、Codex、Cursor(新)、
Aider(從 stub 升級為完整實作)。任一切到任一,16 種組合。

**Target(handoff brief 用哪個 LLM 生成)**:Anthropic、OpenAI(gpt-5/4o/
o1/o3)、Gemini、Ollama(本地),加上 OpenAI-compatible generic 通吃
DeepSeek、Groq、Mistral、Together AI、vLLM、LM Studio。

這層 generic OpenAI-compatible 是這次 release 的關鍵 — 我們不維護第三方
價格表(那會永遠落後),使用者在 config 自己填 `cost_per_1m_input_usd` /
`cost_per_1m_output_usd`,drift 算成本給你看。

## 真實測試結果

同一份 4-turn fixture session 跑了 4 家:

| Provider | Cost (USD) | 對 Opus 倍數 |
|---|---:|---:|
| Anthropic claude-haiku-4-5 | $0.00133 | 1×(基準) |
| OpenAI gpt-4o-mini | $0.00015 | ~9× 便宜 |
| Gemini 2.5-flash | $0.00019 | ~7× 便宜 |
| DeepSeek deepseek-chat | $0.00023 | ~6× 便宜 |

跟 Anthropic Opus 對比的話,DeepSeek 的成本約是 1/30。叙事品質的差距?在
這個用例下肉眼看不出 — handoff brief 本質就是份摘要,不需要 frontier
model。完整 smoke 在
[docs/V030-V040-SMOKE.md](https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V030-V040-SMOKE.md)。

## 它不是什麼

- 不是 chat client(不取代 Cursor UI 或 Claude Code REPL)
- 不是隱私產品(預設把 events.db commit 進 git;設 `db_in_git = false` 保留本機)
- 不附屬任何 vendor — Apache 2.0,你的 prompt 紀錄是你的

## 安裝

```bash
# Homebrew
brew install ShellFans-Kirin/drift/drift

# crates.io(需 Rust 1.85+)
cargo install drift-ai
```

切換 LLM 改 `.prompts/config.toml` 的一行 `provider = "..."` 就好。

## 後續規劃

- v0.5:secret 自動 redaction、Cursor agent-mode 完整支援、Bedrock /
  Vertex / Azure OpenAI 企業版三家原生 wrapper
- v0.6:team handoff(notes sync UX)、跨 vendor 的 brief body 翻譯
- v1.0:schema 跟 format 穩定保證

## 倉庫

https://github.com/ShellFans-Kirin/drift_ai

歡迎 issue + PR。對 abstraction 哪裡有問題、connector 哪裡漏抓 — 都告訴我。
