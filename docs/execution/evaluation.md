# Evaluation

```rust
fn evaluate(plan: &QueryPlan, objects: &[DynamicObject]) -> Vec<DynamicObject>
```

Что делает:

1. Проходит по всем объектам
2. Применяет предикаты из `where` (`==`, `!=`, `AND`)
3. Возвращает только подходящие объекты

Важно:

- `evaluate()` не знает про Kubernetes API
- `evaluate()` не занимается `select` и рендером
- `evaluate()` детерминированная и side-effect free
