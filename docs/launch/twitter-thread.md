# X/Twitter thread — drift_ai v0.1.0

Target: 7 tweets. Embed the repo URL in tweet 1 and the last tweet.

---

**1/** `git blame` has a problem in 2026.

A single commit is 80% Claude Code, 15% Codex, a few human touch-ups.
"committed by user@foo last Tuesday" isn't the truth anymore.

I built **Drift AI** — local-first line-level blame for the AI era.

https://github.com/ShellFans-Kirin/drift_ai

---

**2/** It reads your local session logs:
- `~/.claude/projects/**/*.jsonl` (Claude Code)
- `~/.codex/sessions/**/*.jsonl` (Codex)

Compacts each session, writes to `.prompts/` in your git repo, binds
to the matching commit via `git notes`.

`drift blame file.rs --line 42` returns the full timeline.

---

**3/** Example output (real, from this host):

```
hi.txt
├─ 2026-04-21 06:59  💭 [codex]       session 019daed6
│   +hello
│   +drift
├─ 2026-04-21 07:00  💭 [claude-code] session 40c15914
│   +hello
│   (rejected suggestion)
```

Two different agents, one file, rejected suggestions preserved.

---

**4/** It's Rust + SQLite + git notes. Zero cloud. Your prompts and
your code stay on your box.

`db_in_git = true` by default so teams can share blame via the repo;
flip it off in privacy-sensitive repos and the store stays local.

---

**5/** Ships with its own MCP server. One command to give Claude Code
back its own memory:

```
claude mcp add drift -- drift mcp
```

Five read-only tools:
`drift_blame`, `drift_trace`, `drift_rejected`, `drift_log`,
`drift_show_event`.

Plugins for Codex marketplace follow in v0.2.

---

**6/** **What I deliberately don't do.** I don't claim "this human
wrote line 42". Only "no AI session produced it" — the SHA drift is
the signal, authorship inference isn't.

Commit-granularity honesty beats made-up line-granularity metadata.

---

**7/** v0.1.0 ships today. Pre-compiled binaries for Linux +
macOS (both archs), Apache-2.0, Rust.

Looking for: folks to beat on the attribution layer across
**multi-origin**, **human edit**, **rejected suggestion**, **rename**.

https://github.com/ShellFans-Kirin/drift_ai
