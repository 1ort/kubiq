# Architecture

## High-level flow

CLI
 ↓
Parser
 ↓
AST
 ↓
QueryPlan
 ↓
K8s fetch
 ↓
Evaluator
 ↓
Output

## Main modules

### CLI
- Парсинг аргументов
- Инициализация клиента

### Parser
- DSL грамматика
- AST

### Engine
- Path resolution
- Expr evaluation

### K8s layer
- Discovery
- Resource resolution
- Fetch

### Output
- Table
- JSON

## MVP constraints

- Только list
- Только where
- ==, !=, AND
