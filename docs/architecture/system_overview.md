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
(Aggregate | Sort)
 ↓
Projection + Output

Где:

- K8s fetch выполняет `list` батчами (`limit/continue`) и объединяет страницы
- Evaluator применяет `where`
- Aggregate считает `count/sum/min/max/avg` для aggregation-запросов
- Sort применяется для `order by` в non-aggregation запросах
- Projection применяет `select`/summary/describe
- Output рендерит `table|json|yaml`
- Ошибки типизированы (`CliError`/`K8sError`/`EngineError`/`OutputError`) и сохраняют source chain
