use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while, take_while1},
    character::complete::{char, multispace0, multispace1},
    combinator::{all_consuming, map, opt, recognize, value},
    error::{Error, ErrorKind},
    multi::{many0, separated_list1},
    sequence::{delimited, preceded, terminated, tuple},
};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct QueryAst {
    pub predicates: Vec<Predicate>,
    pub select_paths: Option<Vec<String>>,
    pub order_by: Option<Vec<SortKey>>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortKey {
    pub path: String,
    pub direction: SortDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

pub fn parse_query(input: &str) -> Result<QueryAst, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("WHERE clause is empty".to_string());
    }
    if !starts_with_where_keyword(trimmed) {
        return Err("query must start with WHERE".to_string());
    }

    match all_consuming(delimited(multispace0, query_ast, multispace0)).parse(trimmed) {
        Ok((_, ast)) => Ok(ast),
        Err(_) => Err("invalid query syntax".to_string()),
    }
}

pub fn parse_query_args(args: &[String]) -> Result<QueryAst, String> {
    if args.is_empty() {
        return Err("WHERE clause is empty".to_string());
    }
    if !args[0].eq_ignore_ascii_case("where") {
        return Err("query must start with WHERE".to_string());
    }

    let normalized_args: Vec<String> = args.iter().map(|arg| normalize_arg(arg)).collect();
    parse_query(&normalized_args.join(" "))
}

fn starts_with_where_keyword(input: &str) -> bool {
    input
        .split_whitespace()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("where"))
}

fn normalize_arg(arg: &str) -> String {
    if arg.chars().any(char::is_whitespace) {
        format!("'{}'", arg)
    } else {
        arg.to_string()
    }
}

fn query_ast(input: &str) -> IResult<&str, QueryAst> {
    let (input, predicates) = where_clause(input)?;
    let (input, clauses) = many0(preceded(multispace1, query_suffix_clause)).parse(input)?;

    let mut select_paths = None;
    let mut order_by = None;

    for clause in clauses {
        match clause {
            QuerySuffixClause::Select(paths) => {
                if select_paths.is_some() {
                    return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
                }
                select_paths = Some(paths);
            }
            QuerySuffixClause::OrderBy(keys) => {
                if order_by.is_some() {
                    return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
                }
                order_by = Some(keys);
            }
        }
    }

    Ok((
        input,
        QueryAst {
            predicates,
            select_paths,
            order_by,
        },
    ))
}

#[derive(Clone, Debug, PartialEq)]
enum QuerySuffixClause {
    Select(Vec<String>),
    OrderBy(Vec<SortKey>),
}

fn query_suffix_clause(input: &str) -> IResult<&str, QuerySuffixClause> {
    alt((
        map(order_by_clause, QuerySuffixClause::OrderBy),
        map(select_clause, QuerySuffixClause::Select),
    ))
    .parse(input)
}

fn where_clause(input: &str) -> IResult<&str, Vec<Predicate>> {
    preceded(
        terminated(tag_no_case("where"), multispace1),
        separated_list1(and_separator, predicate),
    )
    .parse(input)
}

fn and_separator(input: &str) -> IResult<&str, ()> {
    value((), tuple((multispace1, tag_no_case("and"), multispace1))).parse(input)
}

fn predicate(input: &str) -> IResult<&str, Predicate> {
    let (input, path) = path(input)?;
    let (input, _) = multispace0(input)?;
    let (input, op) = operator(input)?;
    let (input, _) = multispace0(input)?;
    let (input, value) = predicate_value(input)?;

    Ok((input, Predicate { path, op, value }))
}

fn operator(input: &str) -> IResult<&str, Operator> {
    alt((
        value(Operator::Eq, tag("==")),
        value(Operator::Ne, tag("!=")),
    ))
    .parse(input)
}

fn select_clause(input: &str) -> IResult<&str, Vec<String>> {
    preceded(
        terminated(tag_no_case("select"), multispace1),
        separated_list1(select_separator, path),
    )
    .parse(input)
}

fn select_separator(input: &str) -> IResult<&str, ()> {
    value((), delimited(multispace0, char(','), multispace0)).parse(input)
}

fn order_by_clause(input: &str) -> IResult<&str, Vec<SortKey>> {
    preceded(
        tuple((
            tag_no_case("order"),
            multispace1,
            tag_no_case("by"),
            multispace1,
        )),
        separated_list1(order_key_separator, sort_key),
    )
    .parse(input)
}

fn order_key_separator(input: &str) -> IResult<&str, ()> {
    value((), delimited(multispace0, char(','), multispace0)).parse(input)
}

fn sort_key(input: &str) -> IResult<&str, SortKey> {
    let (input, path) = path(input)?;
    let (input, direction) = opt(preceded(multispace1, sort_direction)).parse(input)?;

    Ok((
        input,
        SortKey {
            path,
            direction: direction.unwrap_or(SortDirection::Asc),
        },
    ))
}

fn sort_direction(input: &str) -> IResult<&str, SortDirection> {
    alt((
        value(SortDirection::Asc, tag_no_case("asc")),
        value(SortDirection::Desc, tag_no_case("desc")),
    ))
    .parse(input)
}

fn path(input: &str) -> IResult<&str, String> {
    map(
        recognize(tuple((ident, many0(preceded(char('.'), ident))))),
        str::to_string,
    )
    .parse(input)
}

fn ident(input: &str) -> IResult<&str, &str> {
    recognize(tuple((
        take_while1(is_ident_start),
        take_while(is_ident_char),
    )))
    .parse(input)
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn predicate_value(input: &str) -> IResult<&str, Value> {
    alt((quoted_string_value, bare_value)).parse(input)
}

fn quoted_string_value(input: &str) -> IResult<&str, Value> {
    map(
        delimited(char('\''), take_while(|c| c != '\''), char('\'')),
        |s: &str| Value::String(s.to_string()),
    )
    .parse(input)
}

fn bare_value(input: &str) -> IResult<&str, Value> {
    map(
        take_while1(|c: char| !c.is_ascii_whitespace()),
        parse_scalar_value,
    )
    .parse(input)
}

fn parse_scalar_value(token: &str) -> Value {
    if token.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if token.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }

    if let Ok(number) = token.parse::<i64>() {
        return Value::from(number);
    }

    if let Ok(number) = token.parse::<u64>() {
        return Value::from(number);
    }

    if let Ok(number) = token.parse::<f64>()
        && let Some(number) = serde_json::Number::from_f64(number)
    {
        return Value::Number(number);
    }

    Value::String(token.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{Operator, SortDirection, parse_query, parse_query_args};

    #[test]
    fn parses_and_chain() {
        let ast = parse_query("where metadata.namespace == default AND spec.nodeName != worker-1")
            .expect("must parse valid query");

        assert_eq!(ast.predicates.len(), 2);
        assert_eq!(ast.predicates[0].op, Operator::Eq);
        assert_eq!(ast.predicates[1].op, Operator::Ne);
        assert_eq!(ast.select_paths, None);
        assert_eq!(ast.order_by, None);
    }

    #[test]
    fn parses_lowercase_and() {
        let ast = parse_query("where metadata.namespace == default and spec.nodeName != worker-1")
            .expect("must parse valid query");
        assert_eq!(ast.predicates.len(), 2);
        assert_eq!(ast.select_paths, None);
    }

    #[test]
    fn does_not_split_and_inside_quoted_value() {
        let ast = parse_query("where metadata.name == 'a AND b' and metadata.namespace == demo-a")
            .expect("must parse valid query");
        assert_eq!(ast.predicates.len(), 2);
        assert_eq!(
            ast.predicates[0].value,
            Value::String("a AND b".to_string())
        );
        assert_eq!(ast.select_paths, None);
    }

    #[test]
    fn parses_bool_and_number_literals() {
        let ast = parse_query("where spec.replicas == 2 AND spec.enabled == true")
            .expect("must parse valid query");

        assert_eq!(ast.predicates[0].value, Value::from(2));
        assert_eq!(ast.predicates[1].value, Value::Bool(true));
        assert_eq!(ast.select_paths, None);
    }

    #[test]
    fn parses_where_from_args() {
        let args = vec![
            "where".to_string(),
            "metadata.namespace".to_string(),
            "==".to_string(),
            "demo-a".to_string(),
        ];
        let ast = parse_query_args(&args).expect("must parse valid args");
        assert_eq!(ast.predicates.len(), 1);
        assert_eq!(ast.predicates[0].value, Value::String("demo-a".to_string()));
        assert_eq!(ast.select_paths, None);
    }

    #[test]
    fn parses_select_in_string_query() {
        let ast = parse_query(
            "where metadata.namespace == demo-a select metadata.name, metadata.namespace",
        )
        .expect("must parse valid query");
        assert_eq!(
            ast.select_paths,
            Some(vec![
                "metadata.name".to_string(),
                "metadata.namespace".to_string()
            ])
        );
    }

    #[test]
    fn parses_select_in_args_query() {
        let args = vec![
            "where".to_string(),
            "metadata.namespace".to_string(),
            "==".to_string(),
            "demo-a".to_string(),
            "select".to_string(),
            "metadata.name,metadata.namespace".to_string(),
        ];
        let ast = parse_query_args(&args).expect("must parse valid args");
        assert_eq!(
            ast.select_paths,
            Some(vec![
                "metadata.name".to_string(),
                "metadata.namespace".to_string()
            ])
        );
    }

    #[test]
    fn parses_select_single_path_in_args_query() {
        let args = vec![
            "where".to_string(),
            "metadata.namespace".to_string(),
            "==".to_string(),
            "demo-a".to_string(),
            "select".to_string(),
            "metadata.name".to_string(),
        ];
        let ast = parse_query_args(&args).expect("must parse valid args");
        assert_eq!(ast.select_paths, Some(vec!["metadata.name".to_string()]));
    }

    #[test]
    fn rejects_select_paths_separated_by_spaces() {
        let err = parse_query(
            "where metadata.namespace == demo-a select metadata.name metadata.namespace",
        )
        .expect_err("must reject whitespace-separated select paths");
        assert_eq!(err, "invalid query syntax");
    }

    #[test]
    fn parses_order_by_single_key_with_default_direction() {
        let ast = parse_query("where metadata.namespace == demo-a order by metadata.name")
            .expect("must parse valid query");

        let keys = ast.order_by.expect("order keys must be parsed");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].path, "metadata.name");
        assert_eq!(keys[0].direction, SortDirection::Asc);
    }

    #[test]
    fn parses_order_by_multiple_keys() {
        let ast = parse_query(
            "where metadata.namespace == demo-a order by spec.priority desc, metadata.name asc",
        )
        .expect("must parse valid query");

        let keys = ast.order_by.expect("order keys must be parsed");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].path, "spec.priority");
        assert_eq!(keys[0].direction, SortDirection::Desc);
        assert_eq!(keys[1].path, "metadata.name");
        assert_eq!(keys[1].direction, SortDirection::Asc);
    }

    #[test]
    fn parses_order_by_before_select() {
        let ast = parse_query(
            "where metadata.namespace == demo-a order by metadata.name desc select metadata.name",
        )
        .expect("must parse valid query");

        assert_eq!(ast.select_paths, Some(vec!["metadata.name".to_string()]));
        assert_eq!(
            ast.order_by.expect("must parse order")[0].direction,
            SortDirection::Desc
        );
    }

    #[test]
    fn parses_select_before_order_by() {
        let ast = parse_query(
            "where metadata.namespace == demo-a select metadata.name order by metadata.name desc",
        )
        .expect("must parse valid query");

        assert_eq!(ast.select_paths, Some(vec!["metadata.name".to_string()]));
        assert_eq!(
            ast.order_by.expect("must parse order")[0].direction,
            SortDirection::Desc
        );
    }

    #[test]
    fn rejects_duplicate_select_clause() {
        let err =
            parse_query("where metadata.name == pod-a select metadata.name select spec.nodeName")
                .expect_err("must reject duplicate select");
        assert_eq!(err, "invalid query syntax");
    }

    #[test]
    fn rejects_duplicate_order_by_clause() {
        let err = parse_query(
            "where metadata.name == pod-a order by metadata.name order by spec.nodeName",
        )
        .expect_err("must reject duplicate order by");
        assert_eq!(err, "invalid query syntax");
    }

    #[test]
    fn rejects_order_by_without_path() {
        let err = parse_query("where metadata.name == pod-a order by")
            .expect_err("must reject empty order by");
        assert_eq!(err, "invalid query syntax");
    }

    #[test]
    fn rejects_unknown_sort_direction() {
        let err = parse_query("where metadata.name == pod-a order by metadata.name upward")
            .expect_err("must reject unknown direction");
        assert_eq!(err, "invalid query syntax");
    }

    #[test]
    fn rejects_non_where_prefix_keyword() {
        let err = parse_query("wherever metadata.name == pod-a")
            .expect_err("must reject non-WHERE prefix");
        assert_eq!(err, "query must start with WHERE");
    }

    #[test]
    fn parse_query_args_preserves_values_with_spaces() {
        let args = vec![
            "where".to_string(),
            "metadata.name".to_string(),
            "==".to_string(),
            "api pod".to_string(),
        ];
        let ast = parse_query_args(&args).expect("must parse spaced value from args");
        assert_eq!(
            ast.predicates[0].value,
            Value::String("api pod".to_string())
        );
    }
}
