# Evaluation

```rust
fn evaluate(plan: &QueryPlan, objects: &[DynamicObject]) -> Vec<DynamicObject>
fn sort_objects(plan: &QueryPlan, objects: &[DynamicObject]) -> Vec<DynamicObject>
fn aggregate(plan: &QueryPlan, objects: &[DynamicObject]) -> Result<Vec<DynamicObject>, EngineError>
```

Что делает pipeline:

1. Проходит по всем объектам
2. Применяет предикаты из `where` (`==`, `!=`, `AND`)
3. Если запрос aggregation -> считает агрегаты и формирует один row
4. Иначе сортирует результат по `order by` (если задан)
5. Передает результат в output layer

Важно:

- `evaluate()` и `sort_objects()` не знают про Kubernetes API
- `engine` не занимается рендером
- `evaluate()`/`sort_objects()`/`aggregate()` детерминированные и side-effect free
