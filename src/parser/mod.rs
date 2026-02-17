use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct QueryAst {
    pub predicates: Vec<Predicate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Predicate {
    pub path: String,
    pub op: Operator,
    pub value: Value,
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
    for segment in split_and_predicates(expr)? {
        let segment = segment.trim();

        if let Some((left, right)) = segment.split_once("==") {
            predicates.push(Predicate {
                path: left.trim().to_string(),
                op: Operator::Eq,
                value: parse_value(right.trim())?,
            });
            continue;
        }

        if let Some((left, right)) = segment.split_once("!=") {
            predicates.push(Predicate {
                path: left.trim().to_string(),
                op: Operator::Ne,
                value: parse_value(right.trim())?,
            });
            continue;
        }

        return Err(format!("unsupported predicate: {segment}"));
    }

    Ok(QueryAst { predicates })
}

fn parse_value(input: &str) -> Result<Value, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("predicate value is empty".to_string());
    }

    if trimmed.starts_with('\'') {
        if !trimmed.ends_with('\'') || trimmed.len() < 2 {
            return Err("unterminated string literal".to_string());
        }
        return Ok(Value::String(trimmed[1..trimmed.len() - 1].to_string()));
    }

    if trimmed.eq_ignore_ascii_case("true") {
        return Ok(Value::Bool(true));
    }

    if trimmed.eq_ignore_ascii_case("false") {
        return Ok(Value::Bool(false));
    }

    if let Ok(number) = trimmed.parse::<i64>() {
        return Ok(Value::from(number));
    }

    if let Ok(number) = trimmed.parse::<u64>() {
        return Ok(Value::from(number));
    }

    if let Ok(number) = trimmed.parse::<f64>()
        && let Some(number) = serde_json::Number::from_f64(number)
    {
        return Ok(Value::Number(number));
    }

    Ok(Value::String(trimmed.to_string()))
}

fn split_and_predicates(input: &str) -> Result<Vec<&str>, String> {
    let bytes = input.as_bytes();
    let mut segments = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut in_single_quote = false;

    while index < bytes.len() {
        if bytes[index] == b'\'' {
            in_single_quote = !in_single_quote;
            index += 1;
            continue;
        }

        if !in_single_quote
            && index + 3 <= bytes.len()
            && bytes[index..index + 3].eq_ignore_ascii_case(b"and")
        {
            let left_ws = index > 0 && bytes[index - 1].is_ascii_whitespace();
            let right_ws = index + 3 < bytes.len() && bytes[index + 3].is_ascii_whitespace();
            if left_ws && right_ws {
                segments.push(input[start..index].trim());
                index += 3;
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                start = index;
                continue;
            }
        }

        index += 1;
    }

    if in_single_quote {
        return Err("unterminated string literal".to_string());
    }

    segments.push(input[start..].trim());
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{Operator, parse_query};

    #[test]
    fn parses_and_chain() {
        let ast = parse_query("where metadata.namespace == default AND spec.nodeName != worker-1")
            .expect("must parse valid query");

        assert_eq!(ast.predicates.len(), 2);
        assert_eq!(ast.predicates[0].op, Operator::Eq);
        assert_eq!(ast.predicates[1].op, Operator::Ne);
    }

    #[test]
    fn parses_lowercase_and() {
        let ast = parse_query("where metadata.namespace == default and spec.nodeName != worker-1")
            .expect("must parse valid query");
        assert_eq!(ast.predicates.len(), 2);
    }

    #[test]
    fn does_not_split_and_inside_quoted_value() {
        let ast = parse_query("where metadata.name == 'a AND b' and metadata.namespace == demo-a")
            .expect("must parse valid query");
        assert_eq!(ast.predicates.len(), 2);
        assert_eq!(ast.predicates[0].value, Value::String("a AND b".to_string()));
    }

    #[test]
    fn parses_bool_and_number_literals() {
        let ast = parse_query("where spec.replicas == 2 AND spec.enabled == true")
            .expect("must parse valid query");

        assert_eq!(ast.predicates[0].value, Value::from(2));
        assert_eq!(ast.predicates[1].value, Value::Bool(true));
    }
}
