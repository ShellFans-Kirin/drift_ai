# Show HN draft — drift v0.2.0

**Status**: draft. Do **not** submit until pre-launch checklist (see
`v020-pre-launch-checklist.md`) is fully green.

---

## Title (≤ 80 chars; HN cap is ~80, anything longer truncates)

**Primary**:

> Show HN: drift – Hand off your in-progress task between Claude and Codex

**Backup** (if the title above is too on-the-nose):

> Show HN: drift handoff – when your AI agent stalls, transfer to another in 10s

---

## Post body

> Hi HN — `drift` is a small CLI I built for myself when I kept losing
> context every time Codex stalled (refused / rate-limited / got dumb)
> and I had to re-paste 30 minutes of work into Claude. The new v0.2
> command, `drift handoff`, packages your in-progress task into a
> markdown brief any LLM can pick up cold:
>
> ```
> $ drift handoff --branch feature/oauth --to claude-code
> ⚡ scanning .prompts/events.db
> ⚡ extracting file snippets and rejected approaches
> ⚡ compacting brief via claude-opus-4-7
> ✅ written to .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md
> ```
>
> The brief lists what you've decided, what you tried and rejected,
> what's open, and where to resume. Paste it into the next agent and
> they pick up mid-task without you re-explaining.
>
> [30-second demo GIF: https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/demo/v020-handoff.gif]
>
> The infra layer underneath (built in v0.1) watches the local session
> logs of Claude Code / Codex / Aider, builds a line-level attribution
> store in `.prompts/events.db` inside your repo, and exposes
> `drift blame` / `drift trace` / a stdio MCP server for the
> "who-wrote-this-line" use case. v0.2's handoff is built on top of
> that — it reads your existing session history and re-renders it
> through Opus into the brief shape.
>
> Stack: Rust, SQLite, single-binary distribution. Local-first — no
> drift server, the only network egress is the Anthropic API call
> (which you can opt out of with `provider = "mock"` in
> `.prompts/config.toml`). MIT? No, **Apache-2.0**.
>
> Install:
>
> ```
> brew install ShellFans-Kirin/drift/drift     # macOS / Linux
> # or
> cargo install drift-ai                       # Rust 1.85+
> ```
>
> What I'd love feedback on:
>
> - Which agents do you most want a connector for next? (Cursor, Cline,
>   Aider beyond stub, custom in-house?)
> - The handoff brief's narrative quality at default (Opus). Does
>   "switch to Haiku for 30× cheaper" feel right, or should the default
>   start cheaper?
> - Use cases I haven't covered. Concrete: the v0.2 scope is solo
>   developer mid-stall; team-handoff and post-hoc audit are deliberately
>   out-of-scope until v0.3.
>
> Repo + docs: https://github.com/ShellFans-Kirin/drift_ai
>
> Independent project. Not affiliated with Anthropic, OpenAI, or any
> agent vendor. Built on top of their session logs, not by them.

---

## Cross-link list (paste into early reply if relevant)

- Threat model + secret handling (the v0.1.2 audit follow-up):
  <https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/SECURITY.md>
- vs Cursor / Copilot chat / Cody / git blame:
  <https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/COMPARISON.md>
- The 30-second demo as an asciinema cast (replayable):
  Upload `docs/demo/v020-handoff.cast` to <https://asciinema.org/> and
  link the resulting URL — pre-launch checklist step.
- v0.2.0 design proposal (for the "why this shape" question):
  <https://github.com/ShellFans-Kirin/drift_ai/blob/main/docs/V020-DESIGN.md>

---

## Likely high-frequency questions (pre-baked answers)

**1. "How is this different from Cursor / Copilot chat history?"**

Those store sessions in their cloud, indexed for replay within their
UI; they don't expose attribution at line level, and they're locked to
one vendor. drift stores everything locally in your repo, indexed by
file + line + commit + agent + decision-state, and is connector-
agnostic — Cursor, Cline, custom agents are all connector PRs.
[`docs/COMPARISON.md`] has the table.

**2. "Why not just write better commit messages?"**

Commit granularity is too coarse. The same 50-line commit can hide
"chose NextAuth, rejected Auth0 because of vendor lock-in, hit a
GitHub-OAuth refresh edge case at line 42, paused mid-flight." None
of that survives in `git log -1`. drift keeps prompts ↔ code attached
at the line level so handoff briefs can cite specific decisions.

**3. "Will my session content leak secrets to GitHub?"**

Yes by default. drift mirrors raw session content into `.prompts/`
and commits `events.db` to git unless you opt out with
`[attribution].db_in_git = false`. SECURITY.md walks through
mitigations (gitleaks/trufflehog as a pre-commit hook is the
recommended pair). v0.2 does **not** scrub secrets; that's v0.3
roadmap. We surface this on first run via a stdin-Enter prompt.

**4. "I don't use Claude Code or Codex. Useful?"**

If you use Aider, the stub is in `crates/drift-connectors/src/aider.rs`
— ~50 LOC of work to wire it up against `~/.aider/chat-history.jsonl`.
CONTRIBUTING.md walks through it as the worked example. For Cursor
chat, Cline, and others — same `SessionConnector` trait, send a PR.

**5. "$0.10 per handoff is a lot."**

Default is Opus because the brief is what the next agent reads
verbatim — narrative quality is what makes handoff actually work. Set
`[handoff].model = "claude-haiku-4-5"` for ~30× reduction (and
shorter, less polished briefs). v0.3 will probably add Ollama / vLLM
support for fully local handoff — depends on what the receiving agent
needs from the brief.

**6. "Why JSON output from the LLM?"**

The renderer is pure Rust string formatting against a structured
intermediate (`HandoffBrief`). JSON is reliable enough at Opus tier
that we get robust extraction + clean re-rendering, vs. trying to
parse free-form markdown sections. Code is in
`crates/drift-core/src/handoff.rs` if you want to see the prompt +
parser.

**7. "Is this going to lock me into Anthropic?"**

No. The compaction is `pluggable` — `MockProvider` for offline /
testing, `AnthropicProvider` is one impl. v0.3 plans Ollama + OpenAI-
compatible. The `events.db` and the rendered brief are both local
files; there's no network round-trip outside the LLM call you opt
into.

**8. "Are you Anthropic / OpenAI?"**

No, independent. README has the explicit "not affiliated" line. The
project is one person (me) doing dogfood-driven open source.
