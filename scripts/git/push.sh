#!/usr/bin/env bash
set -euo pipefail

branch="$(git branch --show-current)"
if [[ -z "$branch" ]]; then
  echo "cannot detect current branch" >&2
  exit 1
fi

git push -u origin "$branch"

echo "If this is a feature branch, open PR at:"
echo "https://github.com/1ort/kubiq/pull/new/$branch"
