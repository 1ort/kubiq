#!/usr/bin/env bash
set -euo pipefail

msg="${MSG:-${1:-}}"
if [[ -z "$msg" ]]; then
  echo "usage: MSG=\"<commit message>\" ./scripts/git/ship.sh" >&2
  exit 1
fi

if [[ -z "$(git symbolic-ref --short -q HEAD)" ]]; then
  echo "detached HEAD is not supported for ship; checkout a feature branch first" >&2
  exit 1
fi

branch="$(git branch --show-current)"
if [[ "$branch" == "master" || "$branch" == "main" ]]; then
  echo "refusing to commit on protected branch '$branch'" >&2
  exit 1
fi

if [[ -z "$(git status --porcelain)" ]]; then
  echo "no changes to commit" >&2
  exit 1
fi

if [[ "${SKIP_VERIFY:-0}" != "1" ]]; then
  ./scripts/verify.sh
fi

if [[ -n "${EXPECTED_FILES:-}" ]]; then
  mapfile -t changed < <(
    {
      git diff --name-only
      git diff --name-only --cached
      git ls-files --others --exclude-standard
    } | sort -u
  )
  expected_input="$(printf '%s' "$EXPECTED_FILES" | tr ',' '\n')"
  mapfile -t expected < <(printf '%s\n' "$expected_input" | sed '/^\s*$/d')
  for path in "${changed[@]}"; do
    allowed=0
    for e in "${expected[@]}"; do
      if [[ "$path" == "$e" ]]; then
        allowed=1
        break
      fi
    done
    if [[ "$allowed" -eq 0 && "${ALLOW_EXTRA:-0}" != "1" ]]; then
      echo "unexpected changed file: $path" >&2
      echo "EXPECTED_FILES allows newline/comma separated paths; set ALLOW_EXTRA=1 to bypass this check" >&2
      exit 1
    fi
  done
fi

git add -A
git commit -m "$msg"
echo "commit created on $branch"
