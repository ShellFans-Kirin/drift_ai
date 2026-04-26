# v0.4.0 Demo Recording Guide

Three GIFs go on the v0.4.0 launch README. Recording requires a real
terminal so this can't be automated from inside Claude Code; this
document is the storyboard + the exact commands to play through.

Tooling:
- `asciinema rec output.cast`  → records a terminal session
- `agg --theme monokai --rows 24 --cols 110 output.cast output.gif`
  → renders to GIF

Check both tools are installed:
```bash
asciinema --version
agg --version
```

## GIF 1 — Bidirectional handoff (≤ 30s)

**File**: `docs/demo/v040-handoff-bidirectional.gif`
**Story**: Codex stalled mid-task → drift handoff → Claude Code resumes,
then Claude rate-limits → drift handoff back → Codex picks up.

**Setup**: a small fixture repo with two pre-canned sessions (one Codex,
one Claude Code) already captured. Create with:
```bash
rm -rf /tmp/drift-demo-bidi && mkdir -p /tmp/drift-demo-bidi && cd /tmp/drift-demo-bidi
git init -q && git config user.email "x@y" && git config user.name x
echo "// auth bones" > src/auth.ts && git add . && git commit -qm "init"
drift init
# Drop two pre-recorded NormalizedSessions into .prompts/sessions/ so
# `drift handoff` has something to summarise. (Recording host-only step.)
```

**Recording sequence** (lower-case = type, UPPER = on-screen output):
1. `clear`
2. `# Codex stalled mid-task. Switch to Claude Code:`
3. `drift handoff --branch feature/oauth --to claude-code`
   — wait for the four ⚡ progress lines + ✅ written line
4. `# Brief generated. Now Claude rate-limits → switch back to Codex:`
5. `drift handoff --branch feature/oauth --to codex`
   — same flow, different footer
6. `cat .prompts/handoffs/$(ls -t .prompts/handoffs/ | head -1) | head -25`
   — show the brief structure briefly

Stop recording.

```bash
asciinema rec docs/demo/v040-handoff-bidirectional.cast
agg --theme monokai --rows 24 --cols 110 \
  docs/demo/v040-handoff-bidirectional.cast \
  docs/demo/v040-handoff-bidirectional.gif
```

## GIF 2 — Multi-LLM cost comparison (≤ 45s)

**File**: `docs/demo/v040-multi-llm-comparison.gif`
**Story**: Same session, four briefs side-by-side from Claude / GPT /
Gemini / DeepSeek. Bottom right shows cost overlay. The headline number
to land: **DeepSeek ≈ 30× cheaper than Anthropic Opus** at similar
narrative quality.

**Setup**: re-use the bidi-demo repo. Need all four env vars exported.

**Recording sequence**:
1. `clear`
2. `# Same session, four LLMs. Cost is the eye-opener.`
3. `# 1/4 — Anthropic Haiku (default)`
4. `drift handoff --branch feature/oauth --to claude-code -o /tmp/brief-anthropic.md`
5. `# 2/4 — OpenAI gpt-4o-mini  (config switched in .prompts/config.toml)`
6. `sed -i 's/provider = "anthropic"/provider = "openai"/' .prompts/config.toml`
7. `drift handoff --branch feature/oauth --to claude-code -o /tmp/brief-openai.md`
8. `# 3/4 — Gemini 2.5-flash`
9. `sed -i 's/provider = "openai"/provider = "gemini"/' .prompts/config.toml`
10. `drift handoff --branch feature/oauth --to claude-code -o /tmp/brief-gemini.md`
11. `# 4/4 — DeepSeek (via OpenAI-compatible)`
12. `sed -i 's/provider = "gemini"/provider = "deepseek"/' .prompts/config.toml`
13. `drift handoff --branch feature/oauth --to claude-code -o /tmp/brief-deepseek.md`
14. `drift cost --by model`
    — table showing the 4 models × cost spread

```bash
asciinema rec docs/demo/v040-multi-llm-comparison.cast
agg --theme monokai --rows 28 --cols 120 \
  docs/demo/v040-multi-llm-comparison.cast \
  docs/demo/v040-multi-llm-comparison.gif
```

## GIF 3 — Cursor session → Claude Code handoff (≤ 30s)

**File**: `docs/demo/v040-cursor-handoff.gif`
**Story**: User has a Cursor session in flight. `drift capture` picks
it up. `drift handoff --to claude-code` produces a brief paste-able
into Claude Code's REPL.

**Setup**: needs a real Cursor session at
`~/Library/Application Support/Cursor/User/workspaceStorage/<hash>/state.vscdb`
(macOS) or the Linux equivalent. Pick a small recent session.

**Recording sequence**:
1. `clear`
2. `# I was using Cursor for an OAuth feature; ran out of context.`
3. `cd ~/repos/my-oauth-project`
4. `drift capture --agent cursor`
   — shows "Captured 1 session(s)" or similar
5. `drift handoff --branch feature/oauth --to claude-code`
6. `# Now switch to Claude Code, paste the brief:`
7. `cat .prompts/handoffs/$(ls -t .prompts/handoffs/ | head -1) | head -25`

```bash
asciinema rec docs/demo/v040-cursor-handoff.cast
agg --theme monokai --rows 24 --cols 110 \
  docs/demo/v040-cursor-handoff.cast \
  docs/demo/v040-cursor-handoff.gif
```

## After recording

1. Optimise GIFs to ≤ 2 MB each:
   ```bash
   gifsicle --optimize=3 --colors 96 docs/demo/v040-*.gif --batch
   ```
2. Verify visually each GIF loops cleanly and is legible at GitHub
   README's default width (~700 px).
3. Commit `*.cast` (small, useful for re-rendering) + `*.gif`
   (large but the user-facing artifact).
4. README hero block in the next section automatically references all
   three.

## What to do if you don't have all four LLM keys

GIF 2 needs all four. Without one of them, you can substitute with two
Anthropic models (Haiku vs Opus) — the cost spread story still holds
(Opus ≈ 15× Haiku) but is less broad. Note this limitation in the
launch post if you take that route.
