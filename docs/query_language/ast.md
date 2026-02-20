# AST

```rust
struct QueryAst {
    predicates: Vec<Predicate>,
    select: Option<SelectClause>,
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

enum SelectClause {
    Paths(Vec<String>),
    Aggregations(Vec<AggregationExpr>),
}

struct AggregationExpr {
    function: AggregationFunction,
    path: Option<String>, // None only for count(*)
}

enum AggregationFunction {
    Count,
    Sum,
    Min,
    Max,
    Avg,
}
```
