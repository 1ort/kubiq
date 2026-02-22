#!/usr/bin/env bash
set -euo pipefail

branch_to_delete="${BRANCH:-${1:-}}"
current="$(git branch --show-current)"

if [[ -z "$(git symbolic-ref --short -q HEAD)" ]]; then
  echo "detached HEAD is not supported for sync-master; checkout a branch first" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "working tree is not clean; commit/stash before sync" >&2
  exit 1
fi

if ! git show-ref --verify --quiet refs/heads/master; then
  echo "local master branch does not exist" >&2
  exit 1
fi

git fetch --prune

if ! git show-ref --verify --quiet refs/remotes/origin/master; then
  echo "origin/master not found; run with configured origin remote" >&2
  exit 1
fi

git checkout master
git pull --ff-only

if [[ -n "$branch_to_delete" && "$branch_to_delete" != "master" ]]; then
  if ! git show-ref --verify --quiet "refs/heads/${branch_to_delete}"; then
    echo "branch '$branch_to_delete' does not exist locally; skip delete"
  elif git merge-base --is-ancestor "$branch_to_delete" master; then
    git branch -d "$branch_to_delete"
    echo "deleted merged branch: $branch_to_delete"
  else
    echo "skip deleting '$branch_to_delete': branch is not merged into master"
  fi
fi

if [[ -n "$branch_to_delete" ]]; then
  echo "synced master; delete target processed: $branch_to_delete"
else
  echo "synced master"
fi

echo "previous branch was: $current"
