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
K8s fetch (discovery + list)
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
- Dynamic list fetch
- Преобразование в внутренний `DynamicObject`

### Output
- Summary по умолчанию (`name`)
- `--describe` (nested)
- `select`-проекция
- `table|json|yaml`

## MVP constraints

- Только list
- where + select
- ==, !=, AND
- Без aggregation/watch/sort/join
