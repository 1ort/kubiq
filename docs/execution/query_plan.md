# QueryPlan

```rust
struct QueryPlan {
    predicates: Vec<Predicate>,
    select_paths: Option<Vec<String>>,
    sort_keys: Option<Vec<SortKey>>,
}
```

`QueryPlan` строится из `QueryAst` и используется тремя частями пайплайна:

- `engine::evaluate` -> `predicates`
- `engine::sort_objects` -> `sort_keys`
- `output` -> `select_paths`
