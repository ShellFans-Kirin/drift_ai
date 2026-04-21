# drift_ai

> Local-first CLI to capture, compact, and bind AI coding sessions to your git history.

`drift` watches the local session logs of your AI coding agents (Claude
Code, Codex, Aider...), runs an LLM-driven compaction over each completed
session, stores the result in `.prompts/` inside your git repo, and binds it
to the matching commit via `git notes`.

After installation, `git log` can show, for every commit:

```
commit abc1234 — Add OAuth login
   [claude-code] 7 turns, decided NextAuth over manual JWT
   [codex]      3 turns, fixed callback URL edge case
```

The name reflects the problem: prompts and the code they produce **drift**
apart unless something binds them. `drift_ai` keeps them stitched together.

## Status

Pre-release. Phase 0 proposal is in [`docs/PHASE0-PROPOSAL.md`](docs/PHASE0-PROPOSAL.md).

## Quickstart

_(filled in once Phase 1 lands)_

## License

Apache 2.0 — see [LICENSE](LICENSE).
