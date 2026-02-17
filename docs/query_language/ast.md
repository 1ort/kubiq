# AST

struct Query {
    resource: String,
    filter: Option<Expr>,
}

enum Expr {
    Eq(Path, Value),
    Ne(Path, Value),
    And(Box<Expr>, Box<Expr>),
}

struct Path(Vec<String>);
