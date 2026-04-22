# Phase 0 — Execution Report

**Date**: 2026-04-21
**Status**: Phase 0 (rev 2) complete; awaiting human approval before Phase 1.
**Branch**: `phase0-proposal`
**PR**: [#1](https://github.com/shellfans-dev/drift_ai/pull/1)

---

## 1. Outcome at a glance

| Item | Result |
|------|--------|
| GitHub repo | https://github.com/shellfans-dev/drift_ai (created) |
| Draft PR | https://github.com/shellfans-dev/drift_ai/pull/1 |
| Proposal raw | [docs/PHASE0-PROPOSAL.md](https://github.com/shellfans-dev/drift_ai/blob/phase0-proposal/docs/PHASE0-PROPOSAL.md) |
| Recommended stack (one line) | **Rust** — daemon perf + single-binary distribution + diff/patch and `rusqlite` are hot paths for the attribution engine |
| Recommended package names | crate `drift-ai`, npm `drift-ai`, go `github.com/shellfans-dev/drift_ai`, **CLI binary `drift`**, notes ref `refs/notes/drift` |

---

## 2. Host environment inventory

| Tool | Version | Notes |
|------|---------|-------|
| `git` | 2.43.0 | OK |
| `gh` | 2.45.0 | Installed during Phase 0; ✅ now authenticated as `shellfans-dev` |
| `node` | 18.19.1 | OK |
| `python3` | 3.12.3 | OK |
| `rustc` / `cargo` | 1.75.0 | Installed during Phase 0 |
| `go` | 1.22.2 | Installed during Phase 0 |
| `claude` (Claude Code) | 2.1.117 | OK |
| `codex` (Codex CLI) | codex-cli 0.122.0 | OK; sandbox writes blocked by missing bubblewrap permissions on this host |
| `git config user.name / user.email` | empty originally | Set to `kirin / kirin@shell.fans` during Phase 0 |
| `ANTHROPIC_API_KEY` | **not set** | [NEEDS-INPUT] — only blocks Phase 3 real-API smoke |

---

## 3. Sessions on disk after seeding

```
~/.claude/projects/-home-kirin/3d132809-...jsonl              pre-existing,  10 lines, plain chat
~/.claude/projects/-home-kirin/80bfcde5-...jsonl              pre-existing,  26 lines, Bash tool_use cycle
~/.claude/projects/-home-kirin/3e3646df-...jsonl              THIS bootstrap conversation (excluded from analysis)
~/.claude/projects/-tmp-drift-seed-claude/40c15914-...jsonl   seed: Write + Edit attempts (perms denied; tool_use shape captured)

~/.codex/sessions/2026/04/21/rollout-...05-46-12-...jsonl     pre-existing, 22 lines (exit/exit)
~/.codex/sessions/2026/04/21/rollout-...06-58-59-...jsonl     seed: read-only blocked
~/.codex/sessions/2026/04/21/rollout-...06-59-16-...jsonl     seed: apply_patch + exec_command shape captured
```

Sandbox blocked the actual writes, but the **tool-call envelopes are
present in the JSONL**, which is what the attribution layer parses. Both
agents' file-op schemas are now confirmed from real evidence.

---

## 4. JSONL schema evidence

### 4.1 Claude Code — Write / Edit tool_use (from `40c15914-...jsonl`)

```json
{"type":"tool_use","id":"toolu_011RPxtWBytoLAPommcxyEqC","name":"Write",
 "input":{"file_path":"/tmp/drift-seed-claude/hi.txt","content":"hello\n"}}

{"type":"tool_use","id":"toolu_0125mnqY52cX9h3dcj5NRAtR","name":"Edit",
 "input":{"replace_all":false,"file_path":"/tmp/drift-seed-claude/hi.txt",
          "old_string":"hello","new_string":"hello\ndrift"}}
```

The matching `tool_result` carries `is_error: true` when permissions are
denied — this is the attribution layer's **rejected-suggestion** signal.

### 4.2 Codex — apply_patch and exec_command (from `019daed6-...jsonl`)

```json
{"type":"custom_tool_call","name":"apply_patch","status":"completed",
 "call_id":"call_286p1JSPls8m4pHbXt6tYtGw",
 "input":"*** Begin Patch\n*** Add File: hi.txt\n+hello\n+drift\n*** End Patch\n"}

{"type":"function_call","name":"exec_command",
 "arguments":"{\"cmd\":\"ls -la\",\"workdir\":\"/tmp/drift-seed-codex\",\"max_output_tokens\":400}",
 "call_id":"call_ldCcyi4SYzsdig5qThuihckJ"}
```

`apply_patch.input` is a literal patch envelope (the `*** Begin Patch` /
`Add File` / `Update File` / `Delete File` / `Move File` grammar) — already
diff-shaped, just parse it. `exec_command.arguments` is a JSON string
containing the shell command — the shared `shell_lexer.rs` module will
recognise `mv` / `cp` / `rm` / redirects / `sed -i` as file-op signals.

---

## 5. Data model — four-requirement self-evaluation

| Requirement | Model handling | Honest? |
|-------------|----------------|---------|
| Multi-origin (one line, multiple agents over time) | One `CodeEvent` per touch; `parent_event_id` chains them; `drift blame --line N` walks the chain in timestamp order | ✅ Native |
| Human-edit detection | SHA-256 ladder — each successful AI event records `content_sha256_after`; on next sync re-hash and emit a `human`-slug event if drifted | ✅ But **does not claim authorship**; the slug `human` means "no AI session produced this" — only honest claim available |
| Rejected suggestions | `rejected: bool` set when `tool_result.is_error = true` (Claude) or `function_call_output` indicates failure (Codex) — observed in seed | ✅ Native |
| Rename lineage | Tier 1: parse `apply_patch *** Move File`, `Bash mv`, `git mv`. Tier 2: `git log --follow` fallback | ✅ Tier 2 is best-effort (git uses 50% similarity threshold) |

**Honest gaps documented in proposal §F**: `Bash python -c open()` invisible
to lexer (caught by SHA ladder, attributed to `human`); encrypted Codex
`reasoning` items only counted, not surfaced.

The model expresses all four requirements without forcing strings into
columns they don't belong to.

---

## 6. Repo state

```
$ git log --oneline --all --decorate
9a3afb8 (HEAD -> phase0-proposal, origin/phase0-proposal)
        docs: phase 0 rev 2 — add line-level attribution data model
aba36b2 docs: add Phase 0 proposal (host inventory, stack, MVP scope, JSONL schema analysis)
6bb7e44 (origin/main, main)
        chore: rename project to drift_ai (CLI binary: drift)
2f1f5c1 chore: scaffold repo (Apache 2.0 license, README stub, .gitignore)
```

Files in repo:

```
drift_ai/
├── LICENSE                       Apache 2.0
├── README.md                     positioning + drift blame example output
├── .gitignore
└── docs/
    ├── PHASE0-PROPOSAL.md        rev 2: full proposal with attribution data model
    └── PHASE0-EXECUTION-REPORT.md  this file
```

---

## 7. [NEEDS-INPUT] before Phase 1 starts

| # | Item | Blocks | Default if no override |
|---|------|--------|------------------------|
| 1 | Tech-stack approval | Phase 1 start | **Rust** |
| 2 | `ANTHROPIC_API_KEY` | Phase 3 real-API smoke + human-edit demo screenshot | run Mock-only, skip the demo |
| 3 | `attribution.db_in_git` default | Phase 1 config schema | **`true`** (team-blame-friendly) |
| 4 | "human" slug semantics | Phase 4 README copy | **"no AI session produced this"** (event timeline, not authorship) |

Item 1 is the only one that blocks Phase 1 starting. Items 2–4 can be
supplied later with smaller blast radius.

---

## 8. Next gate

Reply **`approve`** (uses defaults above) or **`approve, stack=go,
db_in_git=false`** etc. to start Phase 1. Phase 1–4 then run to completion
without further stops.
