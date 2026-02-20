# Evaluation

```rust
fn evaluate(plan: &QueryPlan, objects: &[DynamicObject]) -> Vec<DynamicObject>
fn sort_objects(plan: &QueryPlan, objects: &[DynamicObject]) -> Vec<DynamicObject>
```

Что делает pipeline:

1. Проходит по всем объектам
2. Применяет предикаты из `where` (`==`, `!=`, `AND`)
3. Сортирует результат по `order by` (если задан)
4. Передает результат в output layer

Важно:

- `evaluate()` и `sort_objects()` не знают про Kubernetes API
- `engine` не занимается `select` и рендером
- обе функции детерминированные и side-effect free
