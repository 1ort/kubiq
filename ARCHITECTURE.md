# Architecture

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
Output (summary/describe/select, table/json/yaml)

## Main modules

### CLI
- Парсинг argv
- Флаги output/detail
- Запуск пайплайна

### Parser
- DSL grammar (`where`, `and`, `select`)
- Typed AST (`serde_json::Value`)

### Engine
- Build QueryPlan
- Evaluate predicates (`==`, `!=`, `AND`)

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
- Единая иерархия: `CliError`, `K8sError`, `OutputError`
- Сохранение source chain (через `thiserror`)
- Actionable tips на уровне CLI-ошибок

## MVP constraints

- Только list
- where + select
- ==, !=, AND
- Без aggregation/watch/sort/join
