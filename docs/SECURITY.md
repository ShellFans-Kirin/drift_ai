# Security & Privacy

Drift AI is a local-first tool. The data it captures — your AI coding
sessions — can include things you'd rather not commit to a repo. This
document spells out the threat model honestly.

## Threat model

Drift has **no server**. Nothing is uploaded anywhere by Drift itself.
The data flows are:

1. Your AI agent (Claude Code, Codex, ...) writes session JSONL to its
   own directory under `~/.claude/projects/` or `~/.codex/sessions/`.
   That's the agent's behaviour, not Drift's.
2. `drift capture` reads those files and writes:
   - `code_events` rows into `<repo>/.prompts/events.db` (SQLite)
   - One Markdown file per session into `<repo>/.prompts/sessions/`
3. If `[compaction].provider = "anthropic"` (default), `drift capture`
   sends each session transcript to `api.anthropic.com/v1/messages` to
   produce the Markdown summary. **This is the only network egress.**
   Switch to `provider = "mock"` to skip it entirely.

Anything outside step 3 stays on your machine.

## Current limitations (v0.1.x)

These are *known* and *documented*, not bugs:

1. **`drift capture` does not scrub session content.** Whatever you
   typed into the Claude/Codex chat — including secrets you may have
   pasted — is mirrored into `events.db` and `.prompts/sessions/*.md`
   as-is.
2. **`events.db` is committed to git by default**
   (`[attribution].db_in_git = true`). The intent is team-shared blame;
   the side effect is that a leaked secret in your session ends up in
   the public repo.
3. **`.prompts/sessions/*.md` is human-readable**: the compacted
   summary preserves filenames, decisions, and (often) verbatim
   diff hunks. Anthropic's compactor does not actively redact secrets
   either.

If you ever pasted, e.g., `export AWS_SECRET_ACCESS_KEY=AKIA...` into a
Claude session, that string will land in `events.db` and possibly in
the compacted Markdown.

## Mitigations available today

Pick the strongest one that fits your workflow:

1. **Disable the git side**:
   ```toml
   # .prompts/config.toml
   [attribution]
   db_in_git = false
   ```
   `events.db` and the markdown files stay local. Your team loses
   shared blame; you keep the local view.

2. **Manual review before commit**:
   ```bash
   drift capture
   git diff --cached -- .prompts/
   # actually read the summaries; redact in-place if needed
   git add .prompts/ && git commit
   ```

3. **Pair with a secret scanner as a pre-commit hook**. Drift does not
   ship one, but [gitleaks](https://github.com/gitleaks/gitleaks) and
   [trufflehog](https://github.com/trufflesecurity/trufflehog) catch
   most patterns. Example:
   ```bash
   # .git/hooks/pre-commit
   gitleaks protect --staged --redact -v || exit 1
   ```

4. **Run offline**: set `[compaction].provider = "mock"` and unset
   `ANTHROPIC_API_KEY`. You lose the LLM-generated summary; you keep
   `events.db` purely as a local index.

5. **Don't enable Drift on a repo where chat-pasted secrets are
   expected** until the v0.2 redaction pass lands. There is no shame
   in waiting.

## Roadmap (v0.2+)

These are tracked as planned work, not promises with dates:

- **Regex-based redaction pass** in `drift capture` — recognise
  high-confidence patterns (Anthropic / OpenAI / AWS / GitHub PAT /
  Slack / private key blob) and replace with `<redacted>` placeholders
  before they reach `events.db`.
- **Interactive review mode**: `drift capture --review` opens each
  generated Markdown in `$EDITOR` for confirmation before persisting.
- **Pluggable detector**: optional integration with `trufflehog` /
  `gitleaks` rules so you don't reinvent the regex.
- **`drift redact <session-id>`**: post-hoc scrub of an already-
  captured session, with a clear undo path.

If you want any of these sooner, file a feature request — concrete use
cases bump priority.

## Reporting security issues

Please use [GitHub Security
Advisories](https://github.com/ShellFans-Kirin/drift_ai/security/advisories/new)
for anything that could lead to credential leakage, supply-chain risk,
or remote code execution. Don't open a public issue for those.

For documentation gaps, threat-model corrections, or "you missed
mitigation X" — a regular issue or PR is fine.
