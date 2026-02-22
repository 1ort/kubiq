#!/usr/bin/env bash
set -euo pipefail

echo "[hygiene 1/4] ship blocks detached HEAD"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

mkdir -p "$tmp_dir/detached/scripts/git"
cp scripts/git/ship.sh "$tmp_dir/detached/scripts/git/ship.sh"
chmod +x "$tmp_dir/detached/scripts/git/ship.sh"

(
  cd "$tmp_dir/detached"
  git init -q -b master
  git config user.email "smoke@example.com"
  git config user.name "Smoke"
  echo "x" > file.txt
  git add file.txt
  git commit -qm "init"
  git checkout -q --detach
  echo "y" >> file.txt
  if MSG="smoke" SKIP_VERIFY=1 ./scripts/git/ship.sh >/dev/null 2>&1; then
    echo "ship guard failed: detached HEAD must be blocked" >&2
    exit 1
  fi
)

echo "[hygiene 2/4] push blocks protected branch"
mkdir -p "$tmp_dir/push/scripts/git"
cp scripts/git/push.sh "$tmp_dir/push/scripts/git/push.sh"
chmod +x "$tmp_dir/push/scripts/git/push.sh"

(
  cd "$tmp_dir/push"
  git init -q -b master
  if ./scripts/git/push.sh >/dev/null 2>&1; then
    echo "push guard failed: push from master must be blocked" >&2
    exit 1
  fi
)

echo "[hygiene 3/4] sync-master blocks dirty tree"
mkdir -p "$tmp_dir/sync/scripts/git"
cp scripts/git/sync_master.sh "$tmp_dir/sync/scripts/git/sync_master.sh"
chmod +x "$tmp_dir/sync/scripts/git/sync_master.sh"

(
  cd "$tmp_dir/sync"
  git init -q -b master
  touch dirty.txt
  if ./scripts/git/sync_master.sh >/dev/null 2>&1; then
    echo "sync-master guard failed: dirty tree must be blocked" >&2
    exit 1
  fi
)

echo "[hygiene 4/4] sync-master delete skips unmerged branch"
mkdir -p "$tmp_dir/sync_unmerged/scripts/git"
cp scripts/git/sync_master.sh "$tmp_dir/sync_unmerged/scripts/git/sync_master.sh"
chmod +x "$tmp_dir/sync_unmerged/scripts/git/sync_master.sh"

(
  cd "$tmp_dir/sync_unmerged"
  git init -q -b master
  git config user.email "smoke@example.com"
  git config user.name "Smoke"
  git add scripts/git/sync_master.sh
  echo "base" > base.txt
  git add base.txt
  git commit -qm "base"
  git clone -q --bare . ../origin.git
  git remote add origin ../origin.git
  git push -q -u origin master

  git checkout -qb feature/unmerged
  echo "feature" > feature.txt
  git add feature.txt
  git commit -qm "feature work"
  git checkout -q master

  ./scripts/git/sync_master.sh feature/unmerged >/dev/null
  if ! git show-ref --verify --quiet refs/heads/feature/unmerged; then
    echo "sync-master guard failed: unmerged branch should not be deleted" >&2
    exit 1
  fi
)

echo "hygiene smoke: OK"
