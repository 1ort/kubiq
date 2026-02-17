# AST

```rust
struct QueryAst {
    predicates: Vec<Predicate>,
    select_paths: Option<Vec<String>>,
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
```
