# drift_ai — Phase 0 Proposal

**Status**: User-resolved naming/auth on 2026-04-21 (project name `drift_ai`,
GitHub `shellfans-dev/drift_ai`); tech stack still pending explicit approval.
**Author**: drift_ai bootstrap automation
**Date**: 2026-04-21

drift_ai is a local-first CLI that captures completed AI coding sessions
(Claude Code, Codex, Aider, ...), LLM-compacts them, stores the result in
`.prompts/` inside the user's git repo, and binds each compacted prompt to
its matching commit via `git notes`. The thesis being validated in v0.1.0
is that **a single connector abstraction can serve at least two materially
different agents on day one** — Claude Code and Codex.

The name reflects the problem: prompts and the code they produce drift
apart unless something binds them. The CLI binary is `drift`.

This proposal is the only checkpoint where bootstrap stops for human review.

---

## A. Host Environment Inventory

| Tool | Version | Notes |
|------|---------|-------|
| `git` | 2.43.0 | OK |
| `gh` | 2.45.0 | Installed during Phase 0 (was missing). **Not authenticated.** [NEEDS-INPUT] |
| `node` | 18.19.1 | OK |
| `python3` | 3.12.3 | OK |
| `rustc` | 1.75.0 | Installed during Phase 0 (was missing) |
| `cargo` | (with rustc) | OK |
| `go` | 1.22.2 | Installed during Phase 0 (was missing) |
| `claude` (Claude Code) | 2.1.116 | OK |
| `codex` (Codex CLI) | codex-cli 0.122.0 | OK, authenticated (auth.json present) |

**Git config**: was empty; set during Phase 0 to `kirin / kirin@shell.fans`
(taken from harness `userEmail` context).

**`gh auth status`**: `not logged into any GitHub hosts`. Required before
Phase 0 step D can push the repo. **[NEEDS-INPUT]: run `gh auth login`.**

**`ANTHROPIC_API_KEY`**: **not set**. The default compaction provider needs
this for Phase 3 smoke tests. The OAuth credential at `~/.claude/.credentials.json`
is bound to the Claude Code CLI and is not a substitute. **[NEEDS-INPUT]:
export `ANTHROPIC_API_KEY` before Phase 3 smoke test, or accept that Phase 3
runs only against `MockProvider`.**

**Existing sessions found** (the "host has no sessions" assumption in the
brief was outdated — real session data is already on disk, so no `seed`
step was needed):

```
~/.claude/projects/-home-kirin/
   3d132809-...jsonl   (10 lines, very short Claude session)
   80bfcde5-...jsonl   (26 lines, sudo + netplan, contains tool_use cycle)
   3e3646df-...jsonl   (51753 bytes — THIS bootstrap conversation;
                        excluded from analysis to avoid recursion)

~/.codex/sessions/2026/04/21/
   rollout-2026-04-21T05-46-12-019dae93-...jsonl   (22 lines)
```

Two real, distinct-format JSONLs are sufficient to anchor the schema work.
Phase 1 fixtures will derive synthetic variants (tool-call branch, error
branch) by hand-editing copies, since neither real session covers all the
shapes the parser must handle.

---

## B. Technical Stack — Three Options

### Option 1: Rust

**Pros**
- Single static binary per OS — best installer story for a CLI that ends up
  on contributors' machines who may not have node/python.
- `notify` crate is a battle-tested cross-platform file watcher for the
  Phase 1 daemon (`promptkeep watch`).
- Zero-cost abstractions are useful when streaming large session JSONLs.
- Sets up a viable backend story for the future "step 3" hub without a
  language switch.

**Cons**
- Slowest MVP velocity of the three.
- Smaller drive-by-contributor pool than Go or TypeScript.
- `serde` derive macros for two distinct envelope shapes adds boilerplate.

### Option 2: Go

**Pros**
- Single static binary, trivial cross-compile (`GOOS=linux GOARCH=arm64 go build`).
- Large CLI ecosystem (cobra, viper); idiomatic patterns for daemons.
- Faster MVP velocity than Rust, only marginally slower than TypeScript.
- Wide contributor pool comfortable with infra-style projects.

**Cons**
- Generics are present but verbose; the connector trait will be slightly
  noisier than in Rust.
- Slightly heavier binaries than Rust.

### Option 3: TypeScript (Node)

**Pros**
- Fastest MVP iteration. `npm install -g promptkeep` is the most natural
  install path for the AI-tooling crowd, who all already have Node.
- Easiest LLM SDK story — `@anthropic-ai/sdk` is first-class.
- Largest pool of "I'll send a PR after work" contributors.

**Cons**
- Distribution requires a Node runtime — locks out polyglot users running
  only Rust/Go agents.
- `chokidar` is fine for `watch`, but daemonization is awkward on macOS/Linux.
- Pkgwrap binaries (`pkg`, `nexe`) feel like a workaround.

### Recommendation

**Rust.** Reasoning in one line: _drift_ai's value is being a quietly
running daemon that also publishes a single binary contributors can drop
into CI — Rust optimises for both, and the velocity hit is acceptable for
a tool whose v0.1.0 surface is six commands._

If post-Phase-0 feedback says "MVP velocity is the bigger risk", **Go** is
the safer fallback (same distribution story, faster shipping). TypeScript
is the right pick only if the project is reframed as "AI-dev tooling for
people who already live in npm" — and would force a rewrite later if the
hub backend becomes serious.

---

## C. MVP Scope (v0.1.0)

### C.1 Connector inventory

Day-one connectors are **non-negotiable two**:

| Connector | Source path | Status in MVP |
|-----------|-------------|---------------|
| `claude-code` | `~/.claude/projects/<encoded-cwd>/<session-uuid>.jsonl` | Implemented |
| `codex` | `~/.codex/sessions/YYYY/MM/DD/rollout-<ts>-<uuid>.jsonl` | Implemented |
| `aider` | `<repo>/.aider.chat.history.md` | Stub + TODO; documented in CONTRIBUTING |

The `SessionConnector` trait (Rust) / interface (Go/TS):

```rust
trait SessionConnector {
    fn agent_slug(&self) -> &'static str;          // "claude-code" | "codex"
    fn discover(&self) -> Vec<SessionRef>;          // scan source paths
    fn parse(&self, r: SessionRef) -> RawSession;   // read raw JSONL
    fn normalize(&self, raw: RawSession) -> NormalizedSession;
}

struct NormalizedSession {
    session_id: String,
    agent_slug: String,
    model: Option<String>,
    cwd: PathBuf,
    git_branch_at_capture: Option<String>,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    turns: Vec<Turn>,        // ordered, not nested by parent_uuid
}

struct Turn {
    role: Role,                          // User | Assistant | Tool
    content: String,                     // flattened text view for compactor
    tool_calls: Vec<ToolCall>,           // empty for user/assistant text-only
    tool_results: Vec<ToolResult>,       // empty unless this is a tool turn
    timestamp: DateTime<Utc>,
}
```

The schema analysis below proves both formats can map to this shape without
losing what compaction needs.

### C.2 JSONL schema analysis (the abstraction stress-test)

**Claude Code** — flat lines, discriminated by `type` field:

| `type` | Meaning | Compactor uses? |
|--------|---------|-----------------|
| `permission-mode` | session-scoped metadata | metadata only |
| `file-history-snapshot` | tracked-files diff bookkeeping | drop |
| `user` | user prompt OR tool_result wrapper | yes |
| `assistant` | assistant message; nested `message.content[]` includes `text`, `thinking`, `tool_use` | yes |
| `attachment` | skill listing / deferred-tools delta | drop |
| `last-prompt` | denormalised cache | drop |

Threading: `parentUuid` -> `uuid` chain (a tree). Linearised by walking
chronologically — `timestamp` is monotonic in the files seen.

Session-scope fields appear on every line: `sessionId`, `cwd`, `gitBranch`,
`version`, `entrypoint`, `userType`. We snapshot them from the first
`user`-typed line.

**Codex** — uniform envelope `{ timestamp, type, payload }`:

| `type` | Payload shape | Compactor uses? |
|--------|---------------|-----------------|
| `session_meta` | `{ id, cwd, originator, cli_version, model_provider, base_instructions }` | metadata only |
| `turn_context` | `{ turn_id, cwd, model, sandbox_policy, ... }` | metadata only |
| `event_msg` (`task_started`/`task_complete`/`token_count`/`user_message`/`agent_message`) | turn lifecycle + token accounting | drop most, keep `agent_message` for fallback text |
| `response_item` | `{ type: "message" \| "reasoning" \| "function_call" \| ..., role, content[] }` where content uses `input_text` / `output_text` | yes |

Threading: every meaningful event carries `turn_id`. Turns are
chronologically ordered and self-contained — no parent-pointer reconstruction
needed.

**Differences that the abstraction must hide:**

1. Envelope vs. flat — Codex requires unwrapping `payload`; Claude doesn't.
2. Tree vs. linear threading — Claude needs a topological-by-timestamp pass;
   Codex is already linear.
3. Tool-call placement — Claude inlines `tool_use` blocks inside an
   assistant `message.content`; Codex emits `function_call` and
   `function_call_output` as separate `response_item`s.
4. Where session ID lives — Claude on every line; Codex only in the first
   `session_meta`.
5. Reasoning / thinking content — Claude `thinking` blocks include encrypted
   signatures we drop; Codex `reasoning` items are encrypted opaque blobs we
   also drop. Both signal "the model paused to think" — useful as a turn
   metric, not as content.

**Verdict**: the abstraction is honest. There is no hack — both formats
linearise into the same `NormalizedSession`. Stress-testing this on Aider's
markdown-history format is the right next-connector to gut-check whether
the trait survives a non-JSONL source. (Stub it, do not block on it.)

### C.3 Compaction

**Default provider**: Anthropic Claude (`claude-opus-4-7` for quality;
`claude-haiku-4-5` for low-cost path), reading `ANTHROPIC_API_KEY` from
env. Prompt template lives at `templates/compaction.md`, never inlined in
code.

**Output schema** (TOML frontmatter + Markdown body):

```yaml
---
session_id: 80bfcde5-3658-4449-ae7b-334acd49762b
agent: claude-code
model: claude-opus-4-7
working_dir: /home/kirin
git_head_at_capture: <sha>
captured_at: 2026-04-21T06:12:00Z
turn_count: 12
---

## Summary
<one paragraph>

## Key decisions
- ...

## Files touched
- /etc/netplan/50-cloud-init.yaml

## Open threads
- (none)
```

**Token cost estimate** (per session compaction, Claude Opus 4.7):

| Session size | Input tokens (raw transcript) | Output tokens (summary) | $ at list price |
|--------------|-------------------------------|--------------------------|-----------------|
| Tiny (10 turns) | ~3K | ~400 | ~$0.05 |
| Medium (40 turns) | ~25K | ~700 | ~$0.39 |
| Large (200 turns) | ~150K | ~1.2K | ~$2.30 |

A "1 commit a day" developer pays well under $1/day. We document
`compaction.model = "claude-haiku-4-5"` as the cheap default in the
sample config; users opt into Opus.

**Pluggable interface**: `CompactionProvider` trait with `MockProvider`
for tests (returns deterministic canned summary, never calls API).

### C.4 Git integration: notes vs branch

**Recommendation: `git notes` under `refs/notes/promptkeep`.**

| | git notes | hidden branch |
|--|-----------|---------------|
| Pollutes branch list | no | yes (`refs/heads/promptkeep-prompts`) |
| Show in `git log --notes=promptkeep` | native | needs custom `git log` formatter |
| Survives rebase / cherry-pick | yes (notes track sha) | no |
| One commit ↔ multiple sessions | append-friendly | clumsy |
| Cross-team sync | opt-in (`promptkeep sync push`) | always shows up |
| Standard tooling (`git notes show`) | works | n/a |

The downside of notes is they don't transfer with `git push` by default —
this is a feature for a privacy-sensitive tool, but we document the
opt-in `promptkeep sync push/pull` wrapper in the README.

### C.5 Repo structure (Phase 1 deliverable shape)

```
drift_ai/
├── Cargo.toml                    ([package].name = "drift_ai", [[bin]].name = "drift")
├── LICENSE                       (Apache 2.0)
├── README.md
├── CHANGELOG.md
├── CONTRIBUTING.md               (incl. "How to add a new connector")
├── CODE_OF_CONDUCT.md
├── docs/
│   ├── PHASE0-PROPOSAL.md        (this file)
│   └── STEP1-2-COMPLETION-REPORT.md  (Phase 4 deliverable)
├── templates/
│   └── compaction.md
├── src/
│   ├── main.rs                   (CLI dispatch via clap)
│   ├── lib.rs
│   ├── connector/
│   │   ├── mod.rs                (trait SessionConnector + NormalizedSession)
│   │   ├── claude_code.rs        ← first-class
│   │   ├── codex.rs              ← first-class (deliberately parallel)
│   │   └── aider.rs              (stub + TODO + #[cfg(feature = "aider")])
│   ├── compactor/
│   │   ├── mod.rs                (trait CompactionProvider)
│   │   ├── anthropic.rs          (default)
│   │   └── mock.rs               (test only)
│   ├── store/
│   │   ├── mod.rs                (.prompts/ filesystem layout)
│   │   └── frontmatter.rs
│   ├── git/
│   │   ├── notes.rs              (refs/notes/promptkeep ops)
│   │   ├── bind.rs               (auto-bind logic)
│   │   └── log.rs                (`promptkeep log` wrapper)
│   ├── watch.rs                  (notify-based daemon)
│   ├── config.rs                 (global + project TOML merge)
│   └── cli/
│       ├── init.rs
│       ├── capture.rs
│       ├── list.rs
│       ├── show.rs
│       ├── bind.rs
│       ├── auto_bind.rs
│       ├── log.rs
│       ├── watch.rs
│       ├── sync.rs
│       └── install_hook.rs
└── tests/
    ├── fixtures/
    │   ├── claude-code/
    │   │   ├── 01-plain-chat.jsonl
    │   │   ├── 02-with-tool-calls.jsonl
    │   │   └── 03-failed-retry.jsonl
    │   └── codex/
    │       ├── 01-plain-chat.jsonl
    │       ├── 02-with-tool-calls.jsonl
    │       └── 03-failed-retry.jsonl
    ├── connector_claude_code.rs
    ├── connector_codex.rs
    ├── compactor_mock.rs
    ├── git_notes_binding.rs
    └── e2e_capture_compact_bind.rs
```

Note `connector/claude_code.rs` and `connector/codex.rs` sit side by side
as peers in the directory listing — that's the visual reminder that the
abstraction must treat them as equals. Aider sits below them as a stub
that's clearly aspirational.

### C.6 Test strategy

Because the host has only one real session per agent, the strategy is:

1. **Unit (per connector)**: each connector has 3 fixtures
   (plain / with tool-calls / with failed retry) hand-built from the real
   sessions plus synthetic edge cases. Asserts roundtrip → `NormalizedSession`
   → expected canonical shape.
2. **Compactor**: `MockProvider` with golden-file output; covers the
   prompt-template rendering path without API calls.
3. **Git notes binding**: in-process tempdir git repo; covers single bind,
   multi-session bind to one commit (the "claude+codex on same commit"
   case), and rebase-survival.
4. **End-to-end** (per agent, with `MockProvider`): fixture in →
   `.prompts/sessions/...md` out → bound to a fake commit → `git notes show`
   returns the expected text.
5. **Smoke** (Phase 3, gated by `ANTHROPIC_API_KEY`): one real Anthropic
   call per agent against the actual seed sessions, with the output
   committed to `docs/screenshots/` for the dual-agent demo.

CI runs 1–4 unconditionally; 5 runs only when the API key is in the
environment (skipped in PRs from forks, present in maintainer pushes).

---

## D. Open Items / [NEEDS-INPUT]

| # | Item | Blocks | Suggested resolution |
|---|------|--------|----------------------|
| 1 | GitHub user/org for `<user>/drift_ai` | Phase 0 step D push, Phase 4 release | ✅ Resolved 2026-04-21: `shellfans-dev/drift_ai` |
| 2 | `gh auth status` not logged in | `gh repo create`, `gh release create` | ✅ Resolved 2026-04-21: PAT supplied, auth complete |
| 3 | `ANTHROPIC_API_KEY` not set | Phase 3 real-API smoke test | Export before Phase 3, or accept Mock-only |
| 4 | Tech-stack approval | Phase 1 start | Pending; default Rust unless overridden |
| 5 | Project name confirmation | Everything (rename late = painful) | ✅ Resolved 2026-04-21: `drift_ai` (CLI binary `drift`) |

Items 4 and 5 are the only ones that would force re-doing Phase 0 work.
1, 2, 3 can be supplied at any point before the phase that needs them.

---

## Appendix: Phase 0 deliverable checklist

- [x] Host inventory (versions, auth state)
- [x] Both agents' session JSONLs located and analyzed
- [x] Connector abstraction sketched and stress-tested against both formats
- [x] Tech stack proposal with one-line recommendation
- [x] MVP scope (connectors, compaction, git, repo tree, tests)
- [x] Local repo with LICENSE / README / .gitignore
- [x] This proposal file committed
- [ ] Repo pushed to GitHub (blocked on item 1 + 2)
- [ ] `phase0-proposal` PR opened as draft (blocked on push)
