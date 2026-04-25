# v0.2.0 Design Proposal — `drift handoff`

**Status**: Phase 0 design proposal. **Not yet approved.** Implementation
(Phase 1+) is gated on user sign-off.
**Date**: 2026-04-25
**Branch**: `v0.2.0`

---

## 0. The thesis (one paragraph)

v0.1 sells "AI-era git blame". v0.2 sells "you don't get locked into one
LLM vendor". The wedge is **task transfer between agents**: when Codex
hits a guardrail or Claude rate-limits mid-session, you should be able to
package the in-progress work into a brief that any other agent can pick
up in seconds. `drift` already has the materials (per-session events,
compacted summaries, rejected suggestions, file diffs); v0.2 adds one
command — `drift handoff` — that re-renders those materials as a
*handoff brief* a target agent can absorb cold.

The release window is the Show HN; the demo in §6 is the asset that
makes the post sing.

---

## 1. CLI surface (demo-driven)

### 1.1 The 30-second demo transcript

This is the user-visible script the implementation has to make true. Each
line lists wallclock seconds and what the audience sees:

```
[00:00]  terminal A: a codex session refuses, rate-limits, or just stops
         making progress on `feature/oauth`.
         (Visual: codex prints something like "I'll need to pause...")

[00:10]  user hits Ctrl-C, switches to terminal B (split pane).

[00:13]  $ drift handoff --branch feature/oauth --to claude-code

[00:15]  drift prints to stderr (yellow / blue / green "spinner" lines):
            ⚡ scanning .prompts/events.db on feature/oauth (2 codex, 2 claude-code sessions, 47 events)
            ⚡ extracting file snippets (3 files in scope, +156 -23 across the branch)
            ⚡ compacting brief via claude-opus-4-7 (≈3.4k input, ~600 output, ~$0.10)
            ✅ written to .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md

         next:
           claude
           # then paste:
           "I'm continuing work on feature/oauth. Read the handoff brief"
           "and resume from 'Next steps' #1."
           "$(cat .prompts/handoffs/2026-04-25-1530-feature-oauth-to-claude-code.md)"

[00:23]  user copies suggested incantation, switches to Claude Code,
         pastes.

[00:26]  Claude responds (real, not faked):
            "Understood — you're implementing OAuth with NextAuth on
             feature/oauth. Last decision was to use Authorization Code
             flow (PKCE). The token-refresh edge case at
             src/auth/callback.ts:42 is open. Resuming there now."

[00:30]  END.
```

### 1.2 CLI inferred from the transcript

```
drift handoff [OPTIONS]

Scope (one of, in priority order — first non-empty wins):
  --branch <name>           git branch to gather sessions for (recommended).
                            Looks at all sessions whose `working_dir` matches
                            this repo AND whose timestamps fall inside any
                            commit on this branch's history vs. main.
  --since <iso>             time-range fallback (no branch needed).
  --session <id>            single-session handoff (debugging / unit tests).

Target:
  --to <agent>              one of: claude-code | codex | generic
                            (default: claude-code)

Output:
  --output <path>           override file path
                            (default: .prompts/handoffs/<ts>-<branch>-to-<agent>.md)
  --print                   write to stdout instead of file
                            (mutually exclusive with --output)

Repo override (global):
  --repo <path>             default = cwd (already standard for drift)
```

Hard demo constraints baked into the design:

- **Total wall time from invocation to file written: < 10 seconds.** Opus
  is the default — handoffs are infrequent, the brief drives an entire
  context-transfer, and quality of the "What I'm working on" summary is
  the user-visible signal of value. Cost-sensitive users can drop to
  Haiku via `[handoff].model = "claude-haiku-4-5"` for a ~30× cost
  reduction at the cost of a less polished narrative.
- **stderr structure**: scan-progress lines use ⚡ (work-in-progress,
  cyan/yellow), the final write line uses ✅ (green). The "next:" hint
  block uses no colour (works under `NO_COLOR=1`).
- **Output filename is human-readable**: `<YYYY-MM-DD-HHMM>-<branch>-to-<agent>.md`
  with `/` in branch normalised to `-` and unsafe chars stripped.

### 1.3 What `--to` does (concretely)

`--to` is **not decorative**. It changes the brief's footer ("How to
continue") and the suggested-prompt copy in the stderr "next:" block:

| `--to`        | Footer prompt suggestion                                                     |
|---------------|------------------------------------------------------------------------------|
| `claude-code` | `claude` then a paste-friendly resume prompt                                 |
| `codex`       | `codex` with codex's slightly more imperative phrasing                       |
| `generic`     | No suggested incantation — pure markdown brief, paste anywhere               |

Body of the brief is identical across `--to` values; only the footer
varies. v0.2 explicitly does **not** translate tool-call schemas or do
prompt-engineering tweaks per agent — that's v0.3+ territory.

---

## 2. Handoff brief — markdown shape (mock)

The brief itself, mocked from a hypothetical OAuth task. Real output is
LLM-compacted from `events.db` data; this fixture is what the renderer
must produce given representative input:

```markdown
# Handoff Brief — `feature/oauth`

| Field      | Value                                                       |
|------------|-------------------------------------------------------------|
| From       | codex (2 sessions, 31 turns) + claude-code (2 sessions, 16) |
| To         | claude-code                                                 |
| Generated  | 2026-04-25 15:30 UTC                                        |
| Repo       | ShellFans-Kirin/drift_ai @ feature/oauth                    |
| Branch dif | +156 / -23 across 3 files                                   |

## What I'm working on

Adding OAuth login to the Drift website using NextAuth (App Router).
The sign-in route + callback handler are in place; the open work is the
token-refresh edge case in the callback handler and a session-storage
decision (cookie vs JWT). All decisions were made over four AI sessions
on this branch; this brief is the "you can pick up where I left off"
view.

## Progress so far

- ✅ NextAuth provider config — `src/auth/config.ts`
- ✅ `/api/auth/login` route handler — `src/auth/login.ts`
- ⏳ `/api/auth/callback` — partially done; token-refresh edge case open
- ⏸ Session storage — strategy not yet chosen

## Files in scope

### `src/auth/config.ts` (created)

```typescript
import NextAuth from "next-auth";
import GitHub from "next-auth/providers/github";

export const { handlers, signIn, signOut, auth } = NextAuth({
  providers: [
    GitHub({
      clientId: process.env.GITHUB_ID!,
      clientSecret: process.env.GITHUB_SECRET!,
      authorization: { params: { scope: "read:user user:email" } },
    }),
  ],
  // session: ... (see open question 2)
});
```

### `src/auth/callback.ts` (modified, +47 / -3)

```typescript
// L40-L65 — token refresh path; this is where codex stalled
async function refreshAccessToken(token: JWT): Promise<JWT> {
  // ⚠ Edge case open: GitHub returns the same access_token on refresh
  //   if it hasn't expired yet, but doesn't include refresh_token in
  //   the response — so we mustn't overwrite our existing one.
  const res = await fetch("https://github.com/login/oauth/access_token", {
    method: "POST",
    body: new URLSearchParams({
      client_id: process.env.GITHUB_ID!,
      client_secret: process.env.GITHUB_SECRET!,
      grant_type: "refresh_token",
      refresh_token: token.refreshToken as string,
    }),
  });
  // TODO: handle (1) 200 with new token, (2) 200 same token, (3) 401 expired
}
```

## Key decisions made

- **NextAuth over hand-rolled JWT** *(codex session 7c2…, turn 4)*
  Reason: less boilerplate, mature ecosystem, well-understood by both
  Claude and Codex.
- **Authorization Code + PKCE** *(codex session 7c2…, turn 6)*
  Reason: PKCE is the modern recommendation; Implicit flow is
  deprecated.
- **Single GitHub provider for v0.1 ship** *(claude-code session 9f1…, turn 2)*
  Reason: ship one thing; add Google / Apple later.

## Approaches tried but rejected

- **Hand-rolled JWT with custom refresh** *(rejected — codex session 7c2…, turn 3)*
  > "too much token-refresh boilerplate; we'd be reimplementing NextAuth poorly"
- **Auth0 SDK** *(rejected — codex session 7c2…, turn 8)*
  > "vendor lock-in concern + cost"

## Open questions / blockers

1. Token refresh edge case in `src/auth/callback.ts:L40-65` — three
   response shapes from GitHub need to be handled distinctly. (codex
   was on this when it stalled.)
2. Session storage strategy: cookie sessions vs JWT sessions. Cookie =
   simpler, requires DB; JWT = stateless, harder to revoke.

## Next steps (suggested)

1. Resume `src/auth/callback.ts:L40-65` — implement the three-response-
   shape handling with explicit tests.
2. Decide question (2). Lean toward JWT for v0.1 (no DB dependency).
3. Wire the picked strategy into `src/auth/config.ts`'s `session: ...`.
4. Integration test: full login → callback → session round-trip.

---

*This brief was generated by `drift handoff --branch feature/oauth --to claude-code`.
Source data is in `.prompts/events.db` on this repo.*

## How to continue (paste this to claude-code)

> I'm picking up an in-progress task documented in the handoff brief
> above. Read it end-to-end, then resume from "Next steps #1" — the
> token-refresh edge case in `src/auth/callback.ts:L40-65`. Codebase is
> at the current working directory, branch `feature/oauth`. Don't
> revisit decisions in "Key decisions made"; treat them as settled.
```

The footer paragraph **changes per `--to`**:

- `--to codex` swaps "claude-code" → "codex" and uses imperative phrasing
- `--to generic` drops the footer entirely; pure brief

---

## 3. Implementation architecture

### 3.1 New module: `crates/drift-core/src/handoff.rs`

```rust
pub struct HandoffOptions {
    pub repo: PathBuf,
    pub scope: HandoffScope,        // Branch | Since | Session
    pub target_agent: TargetAgent,  // ClaudeCode | Codex | Generic
}

pub enum HandoffScope {
    Branch(String),
    Since(DateTime<Utc>),
    Session(String),
}

pub struct HandoffBrief {       // intermediate (unrendered) data
    pub source_sessions: Vec<SessionSlim>,
    pub branch: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub repo_full_name: Option<String>,
    pub files_in_scope: Vec<FileSnippet>,
    pub progress: ProgressBlock,
    pub key_decisions: Vec<Decision>,
    pub rejected_approaches: Vec<RejectedApproach>,
    pub open_questions: Vec<String>,
    pub next_steps: Vec<String>,
    pub llm_summary: String,        // "What I'm working on" — LLM-compacted
}

pub fn build_handoff(
    store: &EventStore,
    provider: &dyn CompactionProvider,
    opts: &HandoffOptions,
) -> CompactionRes<HandoffBrief>;

pub fn render_brief(brief: &HandoffBrief, target: TargetAgent) -> String;
```

`build_handoff` orchestrates the four collectors below. `render_brief`
is pure string formatting — no LLM, no I/O. This split makes golden-file
testing trivial (mock the LLM, fix everything else).

### 3.2 Four collectors (each ~50-100 LOC)

1. **`collect_sessions(store, scope)`** — pull `SessionRow` rows that
   match the scope. For `Branch`, intersect with commits on that branch
   vs. `main`; for `Since`, simple date filter; for `Session`, a single
   ID lookup.
2. **`collect_events(store, sessions)`** — pull `CodeEvent` rows for the
   matched sessions. Group by file. Track which lines moved.
3. **`collect_rejected(store, sessions)`** — already a query in v0.1.
4. **`extract_file_snippets(repo, events)`** — for each file in scope:
   read the file from the working tree (not git history; the brief is
   for the *current* state). If file < 50 lines, embed full text. If ≥
   50, embed every modified line range ± 5 lines of context, separated
   by `...` ellipses.

### 3.3 LLM second-pass compaction

A single Anthropic call per `drift handoff` invocation produces:

- The "What I'm working on" 3-5 sentence summary (high-level intent,
  not low-level diff)
- The "Progress so far" status emoji list
- The "Key decisions" extraction (with session+turn citations)
- The "Open questions" extraction
- The "Next steps" inferred suggestions

Prompt template lives at `crates/drift-core/templates/handoff.md`
(filename next to existing `compaction.md`). Template variables:

- `{{branch}}`, `{{repo}}`, `{{generated_at}}`
- `{{session_metas}}` — table of `(agent, session_id, turn_count, started_at)`
- `{{rejected_approaches}}` — pre-extracted rejected events
- `{{file_summaries}}` — `(path, +N -M lines, change-summary)` per file
- `{{recent_turn_excerpts}}` — last K turns from each session, capped
  to 30 turns total to control input cost

Default model: `claude-opus-4-7`. Opus is the right default because
(a) handoff briefs are user-facing artifacts the next agent reads
verbatim — narrative quality matters far more than for the per-session
compaction in v0.1; (b) handoffs are infrequent (a few per workday at
most), so cost concentrates rather than amortises poorly; (c) the
"What I'm working on" 3-5-sentence opener is the first thing a HN
viewer sees, and Opus's narrative coherence sells the demo. Users
willing to trade narrative for ~30× cost can set
`[handoff].model = "claude-haiku-4-5"` in `.prompts/config.toml`.

Token budget cap: input ≤ 4 000 tokens (truncate excerpts hardest);
output ≤ 1 200 tokens. Estimated cost per handoff at default settings:
**~\$0.15** (Opus at \$15 / \$75 per MTok). At Haiku, ~\$0.005.

### 3.4 Rendering

Pure Rust `write!` / `format!`-driven. No new dependency. The renderer
takes `(HandoffBrief, TargetAgent)` and emits the markdown shape from §2.
Body templating is just string concat; only the footer differs per
target agent.

---

## 4. Agent-specific output (what `--to` actually changes)

| Section                       | claude-code | codex   | generic |
|-------------------------------|-------------|---------|---------|
| Header / table                | identical   | id.     | id.     |
| What I'm working on           | identical   | id.     | id.     |
| Progress / Files / Decisions  | identical   | id.     | id.     |
| Open questions / Next steps   | identical   | id.     | id.     |
| `## How to continue` footer   | claude-flavoured paste-prompt | codex-flavoured | **omitted** |

Why so conservative: the body is the value (it captures the *task*).
Tweaking it per agent risks coupling drift to vendor-specific prompt
quirks that change every six months. The footer is a one-paragraph
"copy this to your agent" hint — that's the only place agent identity
matters in v0.2.

---

## 5. Testing strategy

### 5.1 Unit tests (in `crates/drift-core/src/handoff.rs`)

- `collect_sessions_by_branch_intersects_commits`
- `collect_sessions_since_filters_correctly`
- `collect_sessions_by_session_id_returns_one`
- `extract_file_snippets_short_file_returns_full`  — file ≤ 50 lines verbatim
- `extract_file_snippets_long_file_extracts_around_modified_ranges`
- `render_brief_to_claude_code_includes_continue_footer`
- `render_brief_to_generic_omits_footer`
- `render_brief_includes_branch_table_header`

### 5.2 Golden-file tests (new fixture)

`crates/drift-core/tests/handoff_golden.rs` loads a fixed (committed)
in-memory `events.db` snapshot, calls `build_handoff` with a
`MockProvider` that returns canned LLM output, calls `render_brief`,
and asserts byte-for-byte equality with `tests/golden/handoff_*.md`.

This lets us refactor renderer code freely — if golden file changes,
the diff is reviewable.

### 5.3 Integration

`crates/drift-cli/tests/handoff_e2e.rs`: end-to-end against fixture
`events.db`, using `MockProvider` (no API calls). Verifies:

- File written to `.prompts/handoffs/<expected name>`
- `--print` writes to stdout, not a file
- `--to claude-code` vs `--to generic` produces different footers
- Mutually-exclusive `--print` and `--output` errors cleanly

### 5.4 Real Anthropic smoke

Once unit + integration are green, run **once** against
`/home/kirin/drift_ai/.prompts/events.db` (the author's dogfood data),
target agent claude-code, default model (Opus). Capture stdout + the
generated brief into `docs/V020-SMOKE-OUTPUT.md` for the README demo
and ship report.

Cost cap: **\$1.00** hard ceiling for the entire v0.2 dev cycle's
smokes (Opus default raises the per-call cost; allow ~5-10 dev smokes
plus the ship-report smoke). If the cap is approached, drop to Haiku
for any subsequent dev runs and note it in the ship report.

---

## 6. Demo asset planning

### 6.1 Recording

Two-pane terminal recording, 30 seconds, no audio (a Show HN GIF should
auto-play).

- **Tool**: `asciinema rec` for the cast file (replayable, version-control-
  friendly), then `agg` (asciinema-agg) to convert to GIF for inline
  README rendering. Both tools brew-installable on macOS.
- **Cast file**: `docs/demo/v020-handoff.cast` (committed; ~5 KB)
- **GIF**: `docs/demo/v020-handoff.gif` (~1-2 MB; committed)
- **Public asciinema URL**: uploaded to <https://asciinema.org/>; URL
  pasted into V020 ship report and Show HN post.

### 6.2 Script (per the §1.1 transcript)

The demo recording uses **prerecorded** session data — we don't try to
make codex actually stall on camera. Steps:

1. Pre-populate a fixture `feature/oauth` branch with realistic-looking
   `events.db` rows (4 sessions, 47 events, real-ish file diffs).
2. In terminal A, scrolling fake codex output to "set the scene".
3. Cut to terminal B, type the `drift handoff` command live.
4. Real `drift handoff` runs against the fixture. Real Anthropic call
   (Haiku, ~\$0.005). Real wall time (~6-8 s).
5. Show the generated brief (`cat | head -30`).
6. Simulate paste-to-Claude with a screen recording of Claude's actual
   response to the brief content.

### 6.3 Acceptance gate

Before commit-and-push of the GIF: it must show the entire flow in
≤ 30 s of replay time, no edits / cuts / sped-up sections (HN audience
is allergic to over-produced demos).

---

## 7. v0.2 non-goals (stay disciplined)

- ❌ **Cross-agent prompt schema translation** — adapting one agent's
  tool-call JSON to another's. v0.3+.
- ❌ **`drift handoff list` / `drift handoff show <id>`** — handoff
  history queries. Generated briefs are just files in `.prompts/handoffs/`;
  for v0.2 the user can `ls` / `cat` them.
- ❌ **Team handoff** — colleague A → colleague B with sanitisation.
  Adjacent feature; ship after v0.2 stabilises.
- ❌ **Storing handoffs in `git notes`** — v0.1 uses `refs/notes/drift`
  for blame attribution; v0.2 keeps handoffs as plain `.prompts/handoffs/*.md`
  files. If they get committed, they survive in the repo's git history,
  which is enough.
- ❌ **Anything to `events.db` schema** — frozen at v0.1.
- ❌ **Anything to MCP tool list** — frozen at v0.1.
- ❌ **Anything to `SessionConnector` trait** — frozen at v0.1.

---

## 8. Risks & open design questions for reviewer

These are the calls I'd flag for a sceptic before signing off Phase 0:

1. **Branch scoping is loose.** `--branch feature/oauth` finds sessions
   whose `working_dir` matches the repo and whose timestamps fall inside
   any commit's range on that branch vs. `main`. If a developer worked
   on `feature/oauth` *and* `feature/billing` on the same day, sessions
   may bleed across. Mitigation: add a heuristic "score" (intersect with
   files-touched on the branch) and warn if confidence is low.
2. **LLM second-pass cost at Opus default.** A power user running
   `drift handoff` 10×/day pays ~\$30/month at Opus, vs ~\$1.50/month
   at Haiku. The release notes + README must call out this math
   explicitly and give the one-line config-toggle to drop to Haiku
   (and the trade-off: shorter, less narrative briefs). The default is
   Opus because the brief's narrative quality is what makes the demo
   land; users who already love drift and want to run it casually
   should know the dial exists.
3. **`--to <agent>` footer might be over-engineering.** A single
   "next steps" hint plus the existing brief might be enough. Keeping
   the dial because it's cheap (~30 LOC) and gives us a future-extension
   path for v0.3 prompt translation.
4. **Demo has a "fake set the scene" step.** Codex doesn't really stall
   on cue. Acceptable as long as the `drift handoff` part itself is real
   and audience-visibly so. We pre-record codex screen and clearly show
   the live `drift handoff` invocation.

---

## 9. What review would unblock

After the reviewer reads this and either says "go" or flags changes,
Phase 1 implementation can start. Specifically the Phase 1 entry
condition is: **§1 (CLI surface), §2 (markdown shape), §3 (architecture),
§4 (per-agent footer policy) are all approved or the diffs are listed
explicitly.** §5-§8 are nice to confirm but won't block coding.

---

*If reviewing this for sign-off, the question to ask is: "If `drift
handoff --branch feature/oauth --to claude-code` produced exactly the
markdown in §2 in under 10 seconds, and the demo in §6 made that visible
to a Show HN reader in 30 seconds, is that the v0.2 release I want?"*
