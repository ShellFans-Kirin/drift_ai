You are preparing a HANDOFF BRIEF: an in-progress AI-assisted coding task
needs to be transferred to another agent (or another session of the same
agent). Your job is to read the materials below and produce a JSON
object that captures *what's being worked on*, *what's done*, *what's
open*, and *what to do next*.

The receiving agent will read the rendered brief cold — they have not seen
the previous sessions. Optimise for "another competent engineer (or LLM)
can pick this up in the next 5 minutes and resume effective work."

## Source materials

### Branch
{{branch}}

### Source sessions (oldest first)
{{session_metas}}

### Files in scope (path, lines added/deleted, what changed)
{{file_summaries}}

### Rejected approaches (already pre-extracted from session tool_result errors)
{{rejected_approaches}}

### Recent turn excerpts (last few turns from each session, may be truncated)
{{recent_turn_excerpts}}

## Output format

Output ONLY a single JSON object. No prose around it, no code fence.
Just the JSON, starting with `{` and ending with `}`. Schema:

{
  "summary": "3-5 sentences. The task in human terms — what is this branch about? Describe intent and current state. Don't enumerate files; lift the narrative.",
  "progress": [
    {"status": "done", "item": "Wired NextAuth provider config (src/auth/config.ts)"},
    {"status": "in_progress", "item": "Token refresh edge case in callback handler"},
    {"status": "not_started", "item": "Session storage strategy"}
  ],
  "key_decisions": [
    {"text": "Chose NextAuth over hand-rolled JWT.", "citation": "codex 7c2..., turn 4"}
  ],
  "open_questions": [
    "How to handle GitHub returning the same access_token on refresh when not expired?"
  ],
  "next_steps": [
    "Resume src/auth/callback.ts:L40-L65 — implement three-response-shape handling.",
    "Decide: cookie vs JWT session storage."
  ]
}

Hard constraints on output:
- "summary": 3-5 sentences. No bullet lists inside. No filename enumeration.
- "progress": 3-5 items max. status MUST be one of: "done" / "in_progress" / "not_started".
- "key_decisions": 1-3 items max. Each "text" is one sentence. Citations are best-effort: a session-id-short + turn-number if you can identify one, or null otherwise.
- "open_questions": 1-3 items max. Phrase as actual questions. Skip if none.
- "next_steps": 1-4 items max. Phrase as imperatives. Be specific: cite file paths and line ranges when the source materials mention them.
- Do NOT invent decisions, files, or turns that are not in the source materials.
- Do NOT output anything other than the JSON object.
