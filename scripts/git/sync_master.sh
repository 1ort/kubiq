#!/usr/bin/env bash
set -euo pipefail

branch_to_delete="${BRANCH:-${1:-}}"
current="$(git branch --show-current)"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "working tree is not clean; commit/stash before sync" >&2
  exit 1
fi

git checkout master
git fetch --prune
git pull --ff-only

if [[ -n "$branch_to_delete" && "$branch_to_delete" != "master" ]]; then
  git branch -d "$branch_to_delete"
fi

if [[ -n "$branch_to_delete" ]]; then
  echo "synced master; deletion attempted for: $branch_to_delete"
else
  echo "synced master"
fi

echo "previous branch was: $current"
