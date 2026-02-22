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
- `just verify` - fmt + clippy + tests.
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
- `just feature` and `just sync-master` require a clean working tree.
- Optional file-scope guard for `just ship`:
  - `EXPECTED_FILES` can define allowed changed files.
  - Use `ALLOW_EXTRA=1` only when intentional.
