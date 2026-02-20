# QueryPlan

```rust
struct QueryPlan {
    predicates: Vec<EnginePredicate>,
    selection: Option<EngineSelection>, // Paths(...) | Aggregations(...)
    sort_keys: Option<Vec<EngineSortKey>>,
}
```

`QueryPlan` строится из `QueryAst` и используется тремя частями пайплайна:

- `engine::evaluate` -> `predicates`
- `engine::sort_objects` -> `sort_keys`
- `engine::aggregate` -> `selection = Aggregations(...)`
- `output` -> `selection = Paths(...)`
