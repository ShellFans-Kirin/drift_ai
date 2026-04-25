# v0.2.0 pre-launch checklist

**Run all of this in the 30 minutes before you submit Show HN.** If
anything is red, hold the launch.

---

## T-30 min · sanity rebuild from scratch (each item ~5 min)

```bash
# 1. cargo install drift-ai from a clean cargo home
export TMPCARGO=/tmp/drift-launch-check
rm -rf "$TMPCARGO" && CARGO_HOME="$TMPCARGO" cargo install drift-ai --locked
"$TMPCARGO/bin/drift" --version
# expect: drift 0.2.0

# 2. brew install on the Mac (Tailscale SSH or local)
ssh kirin@rueimac-mini bash -lc '
  eval "$(/opt/homebrew/bin/brew shellenv)"
  brew untap ShellFans-Kirin/drift 2>/dev/null || true
  brew tap ShellFans-Kirin/drift
  brew install drift
  drift --version
  brew uninstall drift
  brew untap ShellFans-Kirin/drift
'
# expect: drift 0.2.0 in the middle, clean uninstall at the end

# 3. cargo install does NOT pull in any vulnerability flagged crate
# (we don't run cargo audit by default, but check 0 advisories on push):
GH_TOKEN="$SHELLFANS_KIRIN_PAT" gh run list --repo ShellFans-Kirin/drift_ai --limit 3 \
  --json status,conclusion,name --jq '.[] | [.name, .status, .conclusion] | @tsv'
# expect: latest CI run = completed/success
```

## T-30 min · public URL spot-check

Anonymous fetches (no auth) — same suite as v0.1.1/v0.1.2 audit:

```bash
for url in \
  "https://github.com/ShellFans-Kirin/drift_ai" \
  "https://github.com/ShellFans-Kirin/drift_ai/releases/tag/v0.2.0" \
  "https://github.com/ShellFans-Kirin/drift_ai/releases/download/v0.2.0/drift-v0.2.0-aarch64-apple-darwin.tar.gz" \
  "https://raw.githubusercontent.com/ShellFans-Kirin/homebrew-drift/main/Formula/drift.rb"; do
  printf "%-3s  %s\n" "$(curl -sI -L -o /dev/null -w '%{http_code}' "$url")" "$url"
done
# expect: all 200
```

## T-25 min · upload demo to asciinema.org for inline HN linking

```bash
# Upload the cast (one-time per release).
# asciinema upload prints a public URL on success. Save the URL.
asciinema upload docs/demo/v020-handoff.cast
# Save the printed URL into the show-hn.md draft as the demo link.
```

If `asciinema upload` errors on auth, run `asciinema auth` once and
retry. The asciinema URL lets HN viewers click-through and replay
without downloading the GIF.

## T-20 min · README sanity

```bash
# Open README in a browser (https://github.com/ShellFans-Kirin/drift_ai)
# Visual checks (eyeball, not scriptable):
#   ✓ demo GIF auto-plays in the hero spot
#   ✓ pain copy in first 8 lines is 'agent stalled / transfer 30 min ...'
#   ✓ install commands are copy-pasteable, no v0.1.x ghosts
#   ✓ Honest limitations heading says (v0.2.0)
#   ✓ About section visible at end
```

## T-15 min · pre-baked answers ready

Open these in tabs so you can paste-quote during the first 2 hours:

- [`docs/V020-DESIGN.md`](../V020-DESIGN.md) — design rationale
- [`docs/SECURITY.md`](../SECURITY.md) — secret handling
- [`docs/COMPARISON.md`](../COMPARISON.md) — vs Cursor / Copilot / Cody
- [`docs/LAUNCH-READINESS-AUDIT.md`](../LAUNCH-READINESS-AUDIT.md) — earlier audit
- [`docs/launch/v020-show-hn.md`](v020-show-hn.md) — pre-baked Q&A

## T-10 min · post body final review

- Title ≤ 80 chars (HN cuts at ~80)
- First paragraph reads dogfood-origin, not promotional
- Embedded GIF link works (right-click open in new tab in browser)
- "Independent" disclaimer present
- No `[CTA]` placeholders left in
- No "AI-generated" tells (em-dashes are fine; "leveraging", "comprehensive
  solution", "enterprise-grade" are not)

## T-5 min · timing

- **Window**: Tue / Wed / Thu, 8 PM – 10 PM 台北 = Tue / Wed / Thu morning US Pacific
- **Avoid**: Mondays (US slow ramp), Fridays late (US weekend)
- **Don't**: submit and immediately go to bed. The first 2 hours need
  active reply presence to defend front-page placement

---

## T-0 — submit

- Submit to <https://news.ycombinator.com/submit> with the title from
  `v020-show-hn.md`. URL field: the GitHub repo URL.
  Text field: the post body from `v020-show-hn.md` (HN supports plain
  text only, paste raw).
- Immediately after submit, post the Twitter thread from
  `v020-twitter-thread.md`. HN traffic and Twitter traffic compound.
- For the next 2 hours: **stay on the thread**. Reply to every top-level
  comment. Don't argue; quote the relevant doc paragraph and link.

---

## T+2 hr · monitor

- Position on `news.ycombinator.com`. Front-page or not.
- Comment volume vs. upvotes — if upvotes lag comments by 3-5×, the
  post is contentious; reply faster and shorter.
- crates.io / GitHub Releases download counts. Spike means the demo is
  landing.

---

## T+24 hr · post-mortem

Create `docs/launch/v020-post-launch-notes.md` (after launch only):

- Final HN rank
- Comment Q's not in the pre-baked list
- Issues / PRs filed in the first 24 hr
- Decisions about v0.3 priorities driven by feedback

---

## Hold conditions (red lights — do NOT launch)

- Any of the T-30 install checks fail (cargo or brew)
- `release.yml` last run is failed/cancelled
- Anyone on the team flagged a security concern in the last 24 hr
- A v0.1.x install path is still in the README
- Mac Cellar still has a stale `drift` from a prior verify (means
  someone forgot to clean up)
