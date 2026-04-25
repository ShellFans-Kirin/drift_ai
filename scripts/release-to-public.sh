#!/usr/bin/env bash
# release-to-public.sh — push a release commit + tag to public drift_ai
#
# Usage: ./scripts/release-to-public.sh <tag-name>
# Example: ./scripts/release-to-public.sh v0.3.0
#
# Pre-requisite:
# - You're on main with a clean working tree
# - The tag exists locally (created via git tag -a vX.Y.Z ...)
# - The release commit on main is the squashed final form (no WIP commits)

set -euo pipefail

TAG="${1:?Usage: $0 <tag-name>}"

BRANCH=$(git rev-parse --abbrev-ref HEAD)
[ "$BRANCH" = "main" ] || { echo "must be on main, got $BRANCH"; exit 1; }

git diff --quiet || { echo "working tree dirty"; exit 1; }
git diff --cached --quiet || { echo "staged changes present"; exit 1; }

git rev-parse "$TAG" >/dev/null 2>&1 || { echo "tag $TAG does not exist locally"; exit 1; }

git merge-base --is-ancestor "$TAG" HEAD || { echo "tag $TAG is not on current main"; exit 1; }

# Confirm dev_only is fetched and main is in sync (release should be the squashed
# version that's ready, no surprise commits)
git fetch dev_only main --quiet
LOCAL=$(git rev-parse main)
DEV=$(git rev-parse dev_only/main)
[ "$LOCAL" = "$DEV" ] || {
  echo "WARN: local main ($LOCAL) differs from dev_only/main ($DEV)"
  echo "Push to dev_only first, or pull from dev_only, before releasing to public."
  exit 1
}

echo "→ Pushing main + $TAG to public drift_ai..."
git push public main
git push public "$TAG"

echo ""
echo "✓ Done. release.yml on public drift_ai will now build binaries + dispatch"
echo "  homebrew tap update."
echo ""
echo "Next:"
echo "  - Watch:  GH_TOKEN=\$SHELLFANS_KIRIN_PAT gh run list --repo ShellFans-Kirin/drift_ai --limit 3"
echo "  - When green, run cargo publish on 4 crates (drift-core → drift-connectors → drift-mcp → drift-ai)"
