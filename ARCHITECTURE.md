# Architecture

Текущий baseline: **`v0.2.0`**.

`ARCHITECTURE.md` — краткий обзор. Детальная спецификация архитектуры находится в `docs/architecture/`.

## High-level flow

CLI
 ↓
Parser (`nom`)
 ↓
AST
 ↓
QueryPlan
 ↓
K8s fetch (discovery + paged list)
 ↓
Evaluator (`where`)
 ↓
(Aggregate | Sort)
 ↓
Projection + Output (summary/describe/select, table/json/yaml)

## Main modules

### CLI
- Парсинг argv
- Флаги output/detail
- Запуск пайплайна

### Parser
- DSL grammar (`where`, `and`, `select`, `order by`, aggregation expressions)
- Typed AST (`serde_json::Value`)

### Engine
- Build QueryPlan
- Evaluate predicates (`==`, `!=`, `AND`)
- Sort (`order by`, multi-key)
- Aggregate (`count`, `sum`, `min`, `max`, `avg`)

### K8s layer
- Discovery ресурсов (core + CRD)
- Dynamic paged list fetch (`limit/continue`)
- Safe selector pushdown (`fieldSelector`/`labelSelector`) для подмножества `where ==`
- Преобразование в внутренний `DynamicObject`

### Output
- Summary по умолчанию (`name`)
- `--describe` (nested)
- `select`-проекция
- `table|json|yaml`

### Error model
- Единая иерархия: `CliError`, `K8sError`, `EngineError`, `OutputError`
- Сохранение source chain (через `thiserror`)
- Actionable tips на уровне CLI-ошибок

## Current constraints (`v0.2.0`)

- Только list
- where + select + order by + global aggregation
- ==, !=, AND
- Без watch/join/group by
- Нельзя смешивать projection paths и aggregation в одном `select`
- Aggregation не поддерживает `order by` и `--describe`
