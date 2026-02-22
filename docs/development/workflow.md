# Development workflow automation

This document defines the recommended local workflow for feature delivery.

## Prerequisites

- `git`
- `cargo`
- `just` (recommended)

Install `just`:

```bash
cargo install just
```

## Standard flow

1. Create a branch:

```bash
just feature v0.3-discovery-cache
```

2. Implement changes.

3. Run checks:

```bash
just verify
```

4. Commit with guardrails:

```bash
just ship "feat: add discovery cache"
```

5. Push branch:

```bash
just push
```

6. Generate PR draft text (English):

```bash
just pr-draft feat "add discovery cache" "k8s"
```

7. After merge, sync local repo:

```bash
just sync-master feature/v0.3-discovery-cache
```

## Command reference

- `just bootstrap` - check required tools.
- `just verify` - rustfmt check on changed `.rs` files + clippy + tests.
- `just automation-smoke` - smoke checks for automation scripts and guards.
- `just hygiene-check` - git workflow hygiene smoke checks.
- `just docs-check` - docs/workflow consistency checks.
- `just verify-fast` - quick local test pass.
- `just e2e` - run minikube e2e test suite.
- `just run <args>` - run kubiq with proxy env vars unset.
- `just feature <name>` - create `feature/<name>` branch from current HEAD.
- `just ship <msg>` - run checks (unless `SKIP_VERIFY=1`) and commit.
- `just push` - push current branch with upstream tracking.
- `just pr-draft <type> <title> [scope]` - generate PR title/body in `.tmp/`.
- `just sync-master [branch]` - fast-forward `master` and optionally delete merged branch.

## Guardrails

- `just ship` refuses to run on `master`/`main`.
- `just push` refuses to run on `master`/`main` and detached HEAD.
- `just feature` and `just sync-master` require a clean working tree.
- `just sync-master` deletes only merged branches; unmerged branches are kept.
- Optional file-scope guard for `just ship`:
  - `EXPECTED_FILES` can define allowed changed files.
  - Use `ALLOW_EXTRA=1` only when intentional.

## GitHub integration policy

- Project scripts and `just` recipes do not call GitHub CLI/API directly.
- `just push` may print an URL hint for manual PR opening.
- Agent-level GitHub automation (for example `gh pr create`, CI triage) stays outside project scripts.

## Post-merge and CI triage policy

- If a PR is already merged and new commits are added to the same feature branch, open a new PR for those commits.
- After each merge, sync local `master` and delete the local merged feature branch.
- For CI failures, always report check/job name, failing step, root cause, and run/job URL.
- If check watch output appears stale, use run-level status as source of truth.

## CI alignment

- CI runs `./scripts/automation-smoke.sh` to validate workflow tooling integrity.
- CI runs `./scripts/verify.sh` to enforce the same local quality gate (`fmt + clippy + tests + docs checks`).
