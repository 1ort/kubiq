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

- K8s fetch выполняет `list` батчами (`limit/continue`) и объединяет страницы
- Evaluator применяет `where`
- Projection применяет `select`/summary/describe
- Output рендерит `table|json|yaml`
