#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryAst {
    pub predicates: Vec<Predicate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Predicate {
    pub path: String,
    pub op: Operator,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operator {
    Eq,
    Ne,
}

pub fn parse_query(input: &str) -> Result<QueryAst, String> {
    let trimmed = input.trim();
    let where_prefix = "where ";

    if !trimmed.to_ascii_lowercase().starts_with(where_prefix) {
        return Err("query must start with WHERE".to_string());
    }

    let expr = trimmed[where_prefix.len()..].trim();
    if expr.is_empty() {
        return Err("WHERE clause is empty".to_string());
    }

    let mut predicates = Vec::new();
    for segment in expr.split(" AND ") {
        let segment = segment.trim();

        if let Some((left, right)) = segment.split_once("==") {
            predicates.push(Predicate {
                path: left.trim().to_string(),
                op: Operator::Eq,
                value: right.trim().trim_matches('\'').to_string(),
            });
            continue;
        }

        if let Some((left, right)) = segment.split_once("!=") {
            predicates.push(Predicate {
                path: left.trim().to_string(),
                op: Operator::Ne,
                value: right.trim().trim_matches('\'').to_string(),
            });
            continue;
        }

        return Err(format!("unsupported predicate: {segment}"));
    }

    Ok(QueryAst { predicates })
}

#[cfg(test)]
mod tests {
    use super::{Operator, parse_query};

    #[test]
    fn parses_and_chain() {
        let ast = parse_query("where metadata.namespace == default AND spec.nodeName != worker-1")
            .expect("must parse valid query");

        assert_eq!(ast.predicates.len(), 2);
        assert_eq!(ast.predicates[0].op, Operator::Eq);
        assert_eq!(ast.predicates[1].op, Operator::Ne);
    }
}
