# v0.4.0 Pre-launch checklist

Recommended posting window: **Tue / Wed / Thu, 8–10 PM Asia/Taipei**
(corresponds to ~7–9 AM US Eastern, peak HN traffic).

## T-60 min — install verification

```bash
# Clean cargo install from /tmp (no cached state)
rm -rf /tmp/drift-clean-cargo-v040
CARGO_HOME=/tmp/drift-clean-cargo-v040 cargo install drift-ai --locked
/tmp/drift-clean-cargo-v040/bin/drift --version
# expect: drift 0.4.0
```

## T-50 min — brew verification on macOS

Run on any macOS host (Apple Silicon or Intel) you control:

```bash
brew untap ShellFans-Kirin/drift 2>/dev/null || true
brew tap ShellFans-Kirin/drift
brew install drift
drift --version
brew uninstall drift
# expect: drift 0.4.0
```

## T-40 min — MCP server smoke

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  | drift mcp | head -1
# expect a JSON-RPC response with capabilities
```

## T-30 min — README/asset audit

- [ ] All 3 GIFs load on the GitHub README rendered preview
- [ ] All `https://github.com/ShellFans-Kirin/drift_ai/releases/download/v0.4.0/...`
      tarball URLs return 200
- [ ] CHANGELOG `## [0.4.0]` section is accurate (cross-check vs
      `git log v0.2.0..v0.4.0`)
- [ ] crates.io `drift-ai` page shows 0.4.0
- [ ] Homebrew tap formula has `version "0.4.0"` and matching SHA-256s

## T-20 min — final read-through

- [ ] `docs/launch/v040-show-hn.md` final review (typos, claims, links)
- [ ] `docs/launch/v040-twitter-thread.md` link substitution
      (`<gif-url>` → real release asset URLs)
- [ ] Confirm no Anthropic / OpenAI / Gemini / DeepSeek API key
      appears in any committed file (`grep -rE "sk-[a-zA-Z0-9]{20,}"`
      should return nothing)

## T-10 min — confirm timing

- Asia/Taipei time: 8–10 PM Tue/Wed/Thu
- Avoid: weekends (HN drops engagement), Monday US holidays,
  major Anthropic/OpenAI launch days (we don't want to noise their
  signal)

## T-0 — submit

1. Submit Show HN: paste body from `v040-show-hn.md`. Title is the
   "recommended" line at the top.
2. Within 60 seconds, fire the Twitter/X thread from
   `v040-twitter-thread.md`. Tweet 1 should link to the HN submission.
3. Optional: cross-post tweet 1 to Mastodon / Bluesky / LinkedIn.

## T+0–2 h — comment-section watch

Open the HN post, refresh once a minute. Goals:
- Reply within 5 min to substantive technical questions
- Don't argue with sceptics — answer the underlying question, link to
  the smoke / vision doc, move on
- Don't reply to obvious troll bait
- Save links to interesting questions for follow-up posts

## T+24 h — post-mortem

Write `docs/launch/v040-post-mortem.md` covering:
- HN rank curve (front-page minutes, peak position)
- Submission → install conversion (crates.io + brew install counts
  before/after)
- Top 3 questions that came up — feed these into v0.5 design
- Top 3 things people missed in the README — fix them in a v0.4.1
  docs patch

## What if something breaks at launch

- Found a critical bug: tag `v0.4.1`, rebuild release.yml, push to
  `homebrew-drift`, edit Show HN to add a `(v0.4.1 patches X)` note.
- crates.io upload failed: don't panic. The Homebrew + GitHub Release
  binary downloads are the launch surface; cargo install can come
  later. Note in HN comments.
- Tailscale Mac brew test fails: don't ship until fixed. Brew is the
  recommended install path on macOS — broken brew on launch day is
  worse than a 24h delay.
