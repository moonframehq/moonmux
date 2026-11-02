#!/usr/bin/env bash
# Rebuild Cargo.lock after the merge driver kept one side of a conflict.
# Run by the pre-commit and pre-merge-commit hooks; a no-op otherwise.
# Git does not reread the index after pre-merge-commit, so that hook passes
# --stop: the rebuilt lockfile is staged and the merge waits for `git commit`.
set -euo pipefail

marker=$(git rev-parse --git-path moonmux-cargo-lock-stale)
[[ -e "$marker" ]] || exit 0

# Only the merge that left the marker may rebuild the lockfile (MERGE_HEAD, or
# SQUASH_MSG for --squash, which has no MERGE_HEAD), still at the commit it
# started from. Git removes these files when the merge is committed or
# aborted, so a marker without one is discarded rather than applied to an
# unrelated commit.
stop=${1-}
in_progress() {
  local name
  for name in MERGE_HEAD SQUASH_MSG; do
    [[ -e "$(git rev-parse --git-path "$name")" ]] && return 0
  done
  return 1
}
if [[ "$stop" != --stop ]] && ! in_progress ||
  [[ "$(cat "$marker")" != "$(git rev-parse --verify --quiet HEAD || true)" ]]; then
  rm -f "$marker"
  echo "Cargo.lock: dropped a rebuild left by an earlier merge. If that merge was not aborted, run \`cargo metadata\` and commit Cargo.lock." >&2
  exit 0
fi

# Cargo resolves from the working tree, but the commit records the index. A
# manifest or lockfile edit left unstaged would end up in the staged lockfile.
if ! git diff --quiet -- Cargo.lock ':(glob)**/Cargo.toml'; then
  echo "Cargo.lock: stage or discard the unstaged Cargo.toml and Cargo.lock changes, then commit again; the lockfile is rebuilt from the staged manifests." >&2
  exit 1
fi

# Resolving adds only what the merged manifests need, at the versions our side
# already locks; it reaches the network only when a new package is missing.
if ! cargo metadata --format-version 1 --offline >/dev/null 2>&1 &&
  ! cargo metadata --format-version 1 >/dev/null; then
  echo "Cargo.lock: could not rebuild it. Run \`cargo metadata\`, stage Cargo.lock, and commit again." >&2
  exit 1
fi
git add Cargo.lock
rm -f "$marker"
if [[ "$stop" == --stop ]]; then
  echo "Cargo.lock: rebuilt from the merged manifests and staged. Run \`git commit --no-edit\` to finish the merge." >&2
  exit 1
fi
echo "Cargo.lock: rebuilt from the merged manifests and staged." >&2
