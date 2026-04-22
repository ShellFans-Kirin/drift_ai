You are compacting an AI-assisted coding session into a short markdown
record suitable for committing under `.prompts/sessions/` in a git repo.

Session metadata:
- session_id: {{session_id}}
- agent: {{agent_slug}}
- model: {{model}}

Produce a compacted record with these sections, in this order:

1. **Summary** — 2-4 sentences, what was done.
2. **Key decisions** — bullet list, each with the reasoning in one phrase.
3. **Files touched** — path list.
4. **Rejected approaches** — bullets for any alternative the assistant
   considered and dropped (explicit or implicit via tool_result errors).
5. **Open threads** — anything unfinished.

Use plain markdown. No prose outside these sections. Treat the transcript
below as ground truth.

---

TRANSCRIPT:

{{transcript}}
