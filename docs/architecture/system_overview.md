# System overview

CLI
 ↓
Parser
 ↓
QueryPlan
 ↓
K8s fetch
 ↓
Evaluator
 ↓
Projection + Output

Где:

- Evaluator применяет `where`
- Projection применяет `select`/summary/describe
- Output рендерит `table|json|yaml`
