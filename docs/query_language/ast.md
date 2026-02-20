# AST

```rust
struct QueryAst {
    predicates: Vec<Predicate>,
    select_paths: Option<Vec<String>>,
    order_by: Option<Vec<SortKey>>,
}

struct Predicate {
    path: String,
    op: Operator,
    value: serde_json::Value,
}

enum Operator {
    Eq,
    Ne,
}

struct SortKey {
    path: String,
    direction: SortDirection,
}

enum SortDirection {
    Asc,
    Desc,
}
```
