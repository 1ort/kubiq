# Kubiq

A lightweight CLI to run SQL-like queries against the Kubernetes API (core resources and CRDs).

## Release Status

**Current release baseline: `v0.2.0`**.

MVP (`v0.1.0`) is complete, and milestone `v0.2.x` query-completeness features are available.

Implemented:

- Dynamic resource discovery (core + CRD)
- `list` queries
- Automatic pagination/batching for large `list` responses
- `where` filtering with `==`, `!=`, `AND`
- `order by` sorting with multi-key support and `asc|desc`
- Best-effort server-side filter pushdown for technically feasible subset of `where` (`==`/`!=` on `metadata.name`, `metadata.namespace`, `metadata.labels.*`)
- `select` projection
- Global aggregations in `select` (`count`, `sum`, `min`, `max`, `avg`) without `group by`
- Output formats: `table`, `json`, `yaml`
- Default summary output (`name` only)
- Full output via `--describe`
- End-to-end tests on Minikube
- Typed error hierarchy with actionable CLI tips (`CliError`/`K8sError`/`EngineError`/`OutputError`)

## Features

- Works with any plural Kubernetes resource name (`pods`, `deployments`, `widgets`, ...)
- Typed predicate values (`bool`, `number`, `string`)
- Nested reconstruction for `describe` and parent `select` paths (for example `select metadata`)
- Helpful CLI diagnostics (`--help`, `--version`, actionable error tips)
- Pushdown transparency via stderr warnings for non-pushable predicates and selector fallback

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
kubiq [--output table|json|yaml] [--describe] <resource> where <predicates> [order by <keys>] [select <paths>|<aggregations>]
```

Options:

- `-o, --output <format>`: `table` (default), `json`, `yaml`
- `-d, --describe`: print full nested object
- `--no-pushdown-warnings`: suppress pushdown/fallback warnings in stderr
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

### Aggregation

- Supported aggregate expressions in `select`:
  - `count(*)`
  - `count(path)`
  - `sum(path)`, `min(path)`, `max(path)`, `avg(path)`
- Aggregation returns a single aggregated row
- In v0.2 baseline:
  - `order by` is not supported with aggregation
  - `--describe` is not supported with aggregation
  - projection paths and aggregations cannot be mixed in one `select`

### Order by

- Sorts filtered objects before output
- Supports multi-key sorting (`order by spec.priority desc, metadata.name asc`)
- Default direction is `asc`

## Examples

```bash
# Basic filter
kubiq pods where metadata.namespace == demo-a

# Filter + projection
kubiq pods where metadata.namespace == demo-a select metadata.name,metadata.namespace

# Filter + sorting
kubiq pods where metadata.namespace == demo-a order by metadata.name desc

# Parent projection (nested object in json/yaml)
kubiq -o json pods where metadata.name == worker-a select metadata

# Full nested output
kubiq -o yaml -d pods where metadata.name == worker-a

# CRD example
kubiq -o json widgets where spec.enabled == true select metadata.name,spec.owner

# Aggregation examples
kubiq -o json pods where metadata.namespace == demo-a select count(*)
kubiq -o json pods where metadata.namespace == demo-a select sum(metadata.generation),avg(metadata.generation)
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

`CLI -> Parser (nom) -> AST -> QueryPlan -> K8s discovery/paged-list -> Evaluator -> (Aggregate | Sort) -> Projection -> Output`

## Project Layout

```text
src/
  cli/
  parser/
  engine/
  error.rs
  k8s/
  output/
  dynamic_object.rs
tests/
  e2e_minikube.rs
```

## Documentation

See `docs/` for full details:

- `docs/overview.md`
- `docs/documentation_policy.md`
- `docs/product/cli_spec.md`
- `docs/query_language/grammar.md`
- `docs/query_language/semantics.md`
- `docs/development/testing.md`
- `docs/plans/roadmap_v1.md`
