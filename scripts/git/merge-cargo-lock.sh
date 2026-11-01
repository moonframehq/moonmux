#!/usr/bin/env bash
# Git merge driver for Cargo.lock, installed by `just hooks`.
#
# A lockfile conflict is never resolved by hand: Cargo rebuilds it from the
# merged manifests. When the three-way merge is clean it is kept. Otherwise
# this keeps our side. A merge records that the lockfile is stale, and the
# pre-commit or pre-merge-commit hook runs Cargo before that merge is
# committed, once every Cargo.toml has its merged contents. Cherry-picks,
# reverts, and rebases commit without those hooks, so they stop with the
# lockfile conflicted instead.
set -euo pipefail

base=$1 ours=$2 theirs=$3
merged=$(mktemp)
trap 'rm -f "$merged"' EXIT

cp "$ours" "$merged"
if git merge-file --quiet "$merged" "$base" "$theirs"; then
  cp "$merged" "$ours"
  exit 0
fi

# Git names the operation in GIT_REFLOG_ACTION: `merge <branch>` for a merge
# or squash merge, `pull ...` for a pull. Cherry-picks and reverts leave it
# empty; a rebase, including `pull --rebase`, has a rebase directory.
git_dir=$(git rev-parse --git-dir)
merging=
case ${GIT_REFLOG_ACTION-} in
  merge* | pull*) merging=1 ;;
esac
if [[ -z $merging || -d "$git_dir/rebase-merge" || -d "$git_dir/rebase-apply" ]]; then
  echo "Cargo.lock: kept our side and left it conflicted. Run \`cargo metadata\`, then \`git add Cargo.lock\` and continue." >&2
  exit 1
fi

# The marker names the commit being merged into, so a hook can tell this
# merge from a later one after an abort.
git rev-parse --verify --quiet HEAD >"$(git rev-parse --git-path herdr-moonframe-cargo-lock-stale)" || true
echo "Cargo.lock: kept our side; it is rebuilt with Cargo before the merge commit." >&2
