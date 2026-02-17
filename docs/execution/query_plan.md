# QueryPlan

```rust
struct QueryPlan {
    predicates: Vec<Predicate>,
    select_paths: Option<Vec<String>>,
}
```

`QueryPlan` строится из `QueryAst` и используется двумя частями пайплайна:

- `engine::evaluate` -> `predicates`
- `output` -> `select_paths`
