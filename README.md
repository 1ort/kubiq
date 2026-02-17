# Kubiq

A lightweight CLI to run SQL-like queries against the Kubernetes API (core resources and CRDs).

## MVP Status

**MVP is ready** (`v0.1.0`).

Implemented:

- Dynamic resource discovery (core + CRD)
- `list` queries
- Automatic pagination/batching for large `list` responses
- `where` filtering with `==`, `!=`, `AND`
- `select` projection
- Output formats: `table`, `json`, `yaml`
- Default summary output (`name` only)
- Full output via `--describe`
- End-to-end tests on Minikube

## Features

- Works with any plural Kubernetes resource name (`pods`, `deployments`, `widgets`, ...)
- Typed predicate values (`bool`, `number`, `string`)
- Nested reconstruction for `describe` and parent `select` paths (for example `select metadata`)
- Helpful CLI diagnostics (`--help`, `--version`, actionable error tips)

## Installation

### Prerequisites

- Rust stable
- `kubectl`
- Access to a Kubernetes cluster

### Build

```bash
cargo build --release
```

Run from source:

```bash
cargo run -- <args>
```

## Usage

```bash
kubiq [--output table|json|yaml] [--describe] <resource> where <predicates> [select <paths>]
```

Options:

- `-o, --output <format>`: `table` (default), `json`, `yaml`
- `-d, --describe`: print full nested object
- `-h, --help`: show help
- `-V, --version`: show version

## Query Language

### Where

- Operators: `==`, `!=`
- Logical conjunction: `AND`

Semantics:

- Missing field -> `false`
- Type mismatch -> `false`
- `null` in comparison -> `false`

### Select

- Limits output to selected fields
- Supports comma or whitespace-separated paths
- Parent path selection reconstructs nested output (`select metadata`)

## Examples

```bash
# Basic filter
kubiq pods where metadata.namespace == demo-a

# Filter + projection
kubiq pods where metadata.namespace == demo-a select metadata.name,metadata.namespace

# Parent projection (nested object in json/yaml)
kubiq -o json pods where metadata.name == worker-a select metadata

# Full nested output
kubiq -o yaml -d pods where metadata.name == worker-a

# CRD example
kubiq -o json widgets where spec.enabled == true select metadata.name,spec.owner
```

## Local E2E Test Cluster (Minikube)

Start a clean local cluster with fixtures:

```bash
./scripts/minikube-up.sh
```

Re-apply fixtures:

```bash
./scripts/minikube-reset-data.sh
```

Delete the cluster:

```bash
./scripts/minikube-down.sh
```

Run end-to-end tests:

```bash
KUBIQ_E2E=1 cargo test --test e2e_minikube -- --nocapture
```

## Development Checks

```bash
cargo test -q
cargo run -- --help
```

## Architecture (High Level)

`CLI -> Parser (nom) -> AST -> QueryPlan -> K8s discovery/list -> Evaluator -> Projection -> Output`

## Project Layout

```text
src/
  cli/
  parser/
  engine/
  k8s/
  output/
  dynamic_object.rs
tests/
  e2e_minikube.rs
```

## Documentation

See `docs/` for full details:

- `docs/product/cli_spec.md`
- `docs/query_language/grammar.md`
- `docs/query_language/semantics.md`
- `docs/development/testing.md`
- `docs/plans/mvp_plan.md`
