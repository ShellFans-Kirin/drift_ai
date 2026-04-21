# drift_ai

> AI-native blame for the post-prompt era. Local-first.

`drift` watches the local session logs of your AI coding agents
(Claude Code, Codex, Aider...), LLM-compacts each completed session,
stores the result in `.prompts/` inside your git repo, **builds a
line-level attribution layer that links every code event back to its
originating prompt**, and binds each session to its matching commit
via `git notes`.

After installation, `git log` shows multi-agent attribution per commit:

```
commit abc1234 — Add OAuth login
   [claude-code] 7 turns, decided NextAuth over manual JWT
   [codex]      3 turns, fixed callback URL edge case
   [human]      2 manual edits (src/auth/session.ts L45-47)
```

…and `drift blame` resolves any line back to its full timeline:

```
$ drift blame src/auth/login.ts --line 42
src/auth/login.ts:42
├─ 2026-04-15 14:03  [claude-code]  session abc123  prompt: "add rate limiting"
│  diff: +  if (attempts > 5) throw new RateLimitError()
├─ 2026-04-15 15:20  [human]        post-commit manual edit
│  diff: -  if (attempts > 5)
│         +  if (attempts > MAX_ATTEMPTS)
└─ 2026-04-16 09:12  [codex]        session def456  prompt: "extract magic numbers"
   diff: +  const MAX_ATTEMPTS = 5
```

The thesis: commit granularity is too coarse to be the source of truth in
the AI era. drift_ai keeps prompts and the code they produce stitched
together at the line level — across multiple agents, across human edits,
across renames, and including the suggestions you rejected.

## Status

Pre-release. Phase 0 proposal (revision 2 — adds the line-level
attribution layer) is in [`docs/PHASE0-PROPOSAL.md`](docs/PHASE0-PROPOSAL.md).

## Quickstart

_(filled in once Phase 1 lands)_

## License

Apache 2.0 — see [LICENSE](LICENSE).
