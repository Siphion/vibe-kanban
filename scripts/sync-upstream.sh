#!/usr/bin/env bash
set -euo pipefail

# Sync custom/patches branch with upstream main via rebase.
# After a successful rebase, force-pushes to keep origin in sync
# (prevents duplicate commits from merge-on-pull).
#
# Usage: ./scripts/sync-upstream.sh

UPSTREAM_REMOTE="upstream"
UPSTREAM_BRANCH="main"
LOCAL_BRANCH="custom/patches"
ORIGIN_REMOTE="origin"

echo "==> Fetching ${UPSTREAM_REMOTE}..."
git fetch "${UPSTREAM_REMOTE}"

CURRENT_BRANCH=$(git branch --show-current)
if [ "${CURRENT_BRANCH}" != "${LOCAL_BRANCH}" ]; then
  echo "==> Switching to ${LOCAL_BRANCH}..."
  git checkout "${LOCAL_BRANCH}"
fi

echo "==> Rebasing ${LOCAL_BRANCH} onto ${UPSTREAM_REMOTE}/${UPSTREAM_BRANCH}..."
if git rebase "${UPSTREAM_REMOTE}/${UPSTREAM_BRANCH}"; then
  echo "==> Rebase successful."
else
  echo "!!! Rebase conflicts detected. Resolve them, then run:"
  echo "    git rebase --continue"
  echo "    git push --force-with-lease ${ORIGIN_REMOTE} ${LOCAL_BRANCH}"
  exit 1
fi

echo ""
echo "==> Done. Review with: git log --oneline ${UPSTREAM_REMOTE}/${UPSTREAM_BRANCH}..${LOCAL_BRANCH}"
echo ""
echo "==> IMPORTANT: Now force-push to sync the remote:"
echo "    git push --force-with-lease ${ORIGIN_REMOTE} ${LOCAL_BRANCH}"
