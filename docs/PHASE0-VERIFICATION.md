# Phase 0 — Verification Report

**Date**: 2026-04-22
**Status**: Phase 0 deliverables verified against revised spec. Awaiting `approve` before Phase 1.
**Branch**: `phase0-proposal`
**Relates to**: [PHASE0-PROPOSAL.md](PHASE0-PROPOSAL.md), [PHASE0-EXECUTION-REPORT.md](PHASE0-EXECUTION-REPORT.md)

This report is the human-readable summary of the Phase 0 verification pass
on 2026-04-22: a re-measurement of the host environment, a diff of the
on-disk schemas against what the existing proposal claims, and an
acknowledgement of where current reality has drifted from the previously
committed docs.

---

## 1. Deliverables — live links

| Item | URL |
|---|---|
| Repo | https://github.com/shellfans-dev/drift_ai |
| Draft PR #1 | https://github.com/shellfans-dev/drift_ai/pull/1 |
| PHASE0-PROPOSAL.md | https://github.com/shellfans-dev/drift_ai/blob/phase0-proposal/docs/PHASE0-PROPOSAL.md |
| PHASE0-EXECUTION-REPORT.md | https://github.com/shellfans-dev/drift_ai/blob/phase0-proposal/docs/PHASE0-EXECUTION-REPORT.md |
| This file | https://github.com/shellfans-dev/drift_ai/blob/phase0-proposal/docs/PHASE0-VERIFICATION.md |

---

## 2. Recommended stack — one line

**Rust** — single-binary distribution, `notify`-based daemon, and
`similar` + `rusqlite` keep the two attribution hot paths (diff computation
and `events.db`) clean.

## 3. Package naming

| Layer | Name |
|---|---|
| Cargo crate | `drift-ai` (hyphen; auto-maps to `drift_ai` module) |
| Go module | `github.com/shellfans-dev/drift_ai` |
| npm | `drift-ai` |
| **CLI binary** | **`drift`** |
| git notes ref | `refs/notes/drift` |
| SQLite app id | `drift_ai` |

---

## 4. Seed results — both agents

| Agent | Seed | Evidence on disk | What we got |
|---|---|---|---|
| Claude Code | ✅ | `~/.claude/projects/-tmp-drift-seed-claude/40c15914-...jsonl` | `Write` + `Edit` tool_use envelopes captured. Sandbox denied actual writes, but the matching `tool_result.is_error: true` **is exactly the `rejected` signal** the attribution layer needs. |
| Codex | ✅ | `~/.codex/sessions/2026/04/21/rollout-...06-59-16-...jsonl` | `apply_patch` (custom_tool_call) + `exec_command` (function_call) envelopes captured. `*** Begin Patch / Add File / Update File / Delete File / Move File` grammar confirmed — already diff-shaped, direct parse. |

Both agents' file-op schemas are real-evidence-confirmed, not derived from docs.

---

## 5. Data model — 4-requirement self-evaluation

| Requirement | How the model handles it | Honest? |
|---|---|---|
| **Multi-origin** (one line, multiple agents over time) | One `CodeEvent` per touch; `parent_event_id` chains them; `drift blame` walks the chain in timestamp order. | ✅ Native |
| **Human-edit detection** | SHA-256 ladder: every successful AI event records `content_sha256_after`; on next sync the file is re-hashed and a `human`-slug event is emitted if drifted. **We do not claim authorship** — `human` means "no AI session produced this", which is the only honest claim available. | ✅ With documented semantic limit |
| **Rejected suggestions** | `rejected: bool` column. Set when Claude's `tool_result.is_error = true` or Codex's `function_call_output` indicates failure — both observed in the seed sessions. | ✅ Native |
| **Rename lineage** | Tier 1: parse `apply_patch *** Move File`, `Bash mv`, `git mv` via `shell_lexer.rs`. Tier 2: `git log --follow` fallback. `drift blame` chases `parent_event_id` across renames. | ✅ With the explicit caveat that Tier 2 is best-effort (git uses a 50% similarity threshold) |

**No string-hacking**: no requirement forces a value into a column it
doesn't belong to.

Honest gaps documented in [PROPOSAL §F](PHASE0-PROPOSAL.md#f-self-evaluation-does-the-data-model-honor-the-four-requirements):

- `Bash python -c "open(...).write(...)"` is invisible to the shell lexer.
  The SHA ladder still catches the file change, but attributes it to
  `human` rather than the AI that ran the python. Acceptable for v0.1.0.
- Codex `reasoning` items are encrypted; we count them, we do not surface
  them.
- Claude `MultiEdit` emits one `CodeEvent` per inner edit with an
  intra-call `parent_event_id` chain — slightly stretches
  "one event per tool call" but keeps per-line attribution correct.

---

## 6. [NEEDS-INPUT] — four items, ranked by blast radius

| # | Item | Blocks | Default if you just say `approve` |
|---|---|---|---|
| 1 | Tech-stack approval | Phase 1 start | **Rust** |
| 2 | `ANTHROPIC_API_KEY` | Phase 3 real-API smoke + the human-edit demo screenshot | Run Mock-only, skip the demo screenshot |
| 3 | `attribution.db_in_git` default | Phase 1 config schema | **`true`** (team-blame-friendly) |
| 4 | `human` slug semantics | Phase 4 README copy | **"no AI session produced this"** (event timeline, not authorship) |

Only item 1 blocks Phase 1 from starting. Items 2–4 can be supplied later
with smaller blast radius.

---

## 7. Host environment — re-measured 2026-04-22

| Tool | Version | Status |
|---|---|---|
| `git` | 2.43.0 | OK |
| `gh` | 2.45.0 | ✅ authenticated as `shellfans-dev` |
| `node` | 18.19.1 | OK |
| `python3` | 3.12.3 | OK |
| `rustc` / `cargo` | 1.75.0 | OK |
| `go` | 1.22.2 | OK |
| `claude` (Claude Code) | **2.1.117** | Minor drift: PROPOSAL/EXECUTION-REPORT state `2.1.116` |
| `codex` | codex-cli 0.122.0 | OK |
| `git config user.name / user.email` | kirin / kirin@shell.fans | OK |
| `ANTHROPIC_API_KEY` | **NOT SET** | `[NEEDS-INPUT]` — blocks Phase 3 real-API smoke only |
| `gh repo view shellfans-dev/drift_ai` | exists | Pre-existing (not created here) |

**Drift from prior docs**:
- Claude Code bumped `2.1.116 → 2.1.117`. Trivial, does not affect the
  schema analysis. Will be corrected when Phase 1 touches these docs.

**Repo state on disk**:
- `/home/kirin/drift_ai` is already cloned and tracking `origin/phase0-proposal`.
- `git status`: clean.
- Branches: `main`, `phase0-proposal` (both local + origin).
- Commits on `phase0-proposal`:
  ```
  5443cf7 docs: add Phase 0 execution report
  9a3afb8 docs: phase 0 rev 2 — add line-level attribution data model
  aba36b2 docs: add Phase 0 proposal (host inventory, stack, MVP scope, JSONL schema analysis)
  6bb7e44 chore: rename project to drift_ai (CLI binary: drift)
  2f1f5c1 chore: scaffold repo (Apache 2.0 license, README stub, .gitignore)
  ```

---

## 8. Verification against the revised spec

This pass checked that every Phase 0 deliverable named in the revised
spec is covered somewhere in the committed docs.

| Spec requirement | Covered by |
|---|---|
| A. Host environment (versions, auth, seed paths) | [PROPOSAL §A](PHASE0-PROPOSAL.md#a-host-environment-inventory), [EXECUTION §2–3](PHASE0-EXECUTION-REPORT.md#2-host-environment-inventory), §7 above |
| A. File-op tool call fields per agent | [PROPOSAL §C](PHASE0-PROPOSAL.md#c-jsonl-schema-analysis-file-op-focused), [EXECUTION §4](PHASE0-EXECUTION-REPORT.md#4-jsonl-schema-evidence) |
| A. Drift_ai repo reachable, no new repo created | §1 + §7 above |
| B. Tech stack 3-option comparison + recommendation | [PROPOSAL §B](PHASE0-PROPOSAL.md#b-technical-stack--three-options), §2 above |
| B. Package naming proposal | [PROPOSAL §B](PHASE0-PROPOSAL.md#package--binary-naming), §3 above |
| C. MVP scope (connectors, compaction, git, file tree) | [PROPOSAL §E](PHASE0-PROPOSAL.md#e-mvp-scope) |
| D.1 NormalizedSession | [PROPOSAL §D.1](PHASE0-PROPOSAL.md#d1-normalizedsession-session-layer) |
| D.2 CodeEvent (all required fields) | [PROPOSAL §D.2](PHASE0-PROPOSAL.md#d2-codeevent-line-layer--the-new-core-record) |
| D.3 Human-edit detection strategy | [PROPOSAL §D.3](PHASE0-PROPOSAL.md#d3-human-edit-detection-sha-256-ladder) |
| D.4 Rename handling (2-tier) | [PROPOSAL §D.4](PHASE0-PROPOSAL.md#d4-rename-handling) |
| D.5 Storage layout + `db_in_git` switch | [PROPOSAL §D.5](PHASE0-PROPOSAL.md#d5-storage-layout) |
| D. Schema diagram | [PROPOSAL §D schema picture](PHASE0-PROPOSAL.md#schema-picture-mermaid) (Mermaid ER) |
| E. Clone → write proposal → branch → draft PR | §1 above (PR #1 open + draft) |

All spec items are accounted for. No gap found.

---

## 9. Next gate

Reply **`approve`** (uses the defaults in §6) to start Phase 1.
Or override, e.g. **`approve, stack=go, db_in_git=false`**.

Phase 1 → 2 → 3 → 4 run to completion without further stops once approved.
