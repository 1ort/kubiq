use clap::{Parser, ValueEnum, error::ErrorKind};

use crate::{dynamic_object::DynamicObject, engine, error::CliError, k8s, output, parser};

#[derive(Clone, Debug, ValueEnum)]
enum OutputArg {
    Table,
    Json,
    Yaml,
}

#[derive(Parser, Debug)]
#[command(name = "kubiq")]
#[command(about = "Query Kubernetes resources with where/order by/select")]
#[command(version)]
struct CliArgs {
    #[arg(
        short = 'o',
        long = "output",
        default_value = "table",
        value_enum,
        ignore_case = true
    )]
    output: OutputArg,

    #[arg(short = 'd', long = "describe")]
    describe: bool,

    #[arg(long = "no-pushdown-warnings")]
    no_pushdown_warnings: bool,

    #[arg(value_name = "resource")]
    resource: String,

    #[arg(value_name = "query", required = true, num_args = 1..)]
    query: Vec<String>,
}

pub fn run() -> Result<(), CliError> {
    let Some(args) = parse_cli_args()? else {
        return Ok(());
    };
    let ast = parse_query_tokens(&args.query)?;
    let pushdown_plan = k8s::planner::plan_pushdown(&ast.predicates);
    let plan = ast_to_engine_plan(&ast);

    if !args.no_pushdown_warnings {
        for diagnostic in &pushdown_plan.diagnostics {
            eprintln!("{}", format_planner_diagnostic(diagnostic));
        }
    }

    let list_result = k8s::list(&args.resource, &pushdown_plan.options).map_err(CliError::K8s)?;
    if !args.no_pushdown_warnings {
        for diagnostic in &list_result.diagnostics {
            eprintln!("{}", format_k8s_diagnostic(diagnostic));
        }
    }

    let filtered = engine::evaluate(&plan, &list_result.objects);
    let is_aggregation = matches!(plan.selection, Some(engine::EngineSelection::Aggregations(_)));
    if args.describe && is_aggregation {
        return Err(CliError::InvalidArgs(
            "`--describe` is not supported for aggregation queries".to_string(),
        ));
    }

    let rows = if is_aggregation {
        engine::aggregate(&plan, &filtered).map_err(CliError::Engine)?
    } else {
        engine::sort_objects(&plan, &filtered)
    };

    let detail = if args.describe {
        output::DetailLevel::Describe
    } else {
        output::DetailLevel::Summary
    };

    let output_paths = output_paths_for_rows(&plan, &rows);

    output::print(
        &rows,
        map_output_format(args.output),
        detail,
        output_paths.as_deref(),
    )
    .map_err(CliError::Output)?;

    Ok(())
}

fn parse_cli_args() -> Result<Option<CliArgs>, CliError> {
    match CliArgs::try_parse() {
        Ok(args) => Ok(Some(args)),
        Err(error) => {
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                print!("{error}");
                return Ok(None);
            }
            Err(CliError::InvalidArgs(error.to_string()))
        }
    }
}

fn parse_query_tokens(tokens: &[String]) -> Result<parser::QueryAst, CliError> {
    if tokens
        .first()
        .is_some_and(|token| token.eq_ignore_ascii_case("where"))
    {
        parser::parse_query_args(tokens).map_err(CliError::Parse)
    } else {
        parser::parse_query(&tokens.join(" ")).map_err(CliError::Parse)
    }
}

fn ast_to_engine_plan(ast: &parser::QueryAst) -> engine::QueryPlan {
    engine::QueryPlan {
        predicates: ast.predicates.iter().map(predicate_to_engine).collect(),
        selection: ast.select.as_ref().map(select_clause_to_engine),
        sort_keys: ast
            .order_by
            .as_ref()
            .map(|keys| keys.iter().map(sort_key_to_engine).collect()),
    }
}

fn select_clause_to_engine(clause: &parser::SelectClause) -> engine::EngineSelection {
    match clause {
        parser::SelectClause::Paths(paths) => engine::EngineSelection::Paths(paths.clone()),
        parser::SelectClause::Aggregations(expressions) => engine::EngineSelection::Aggregations(
            expressions.iter().map(aggregation_to_engine).collect(),
        ),
        parser::SelectClause::Mixed { .. } => {
            unreachable!("mixed select clause must be rejected by parser validation")
        }
    }
}

fn aggregation_to_engine(expression: &parser::AggregationExpr) -> engine::EngineAggregationExpr {
    engine::EngineAggregationExpr {
        function: aggregation_function_to_engine(&expression.function),
        path: expression.path.clone(),
    }
}

fn aggregation_function_to_engine(
    function: &parser::AggregationFunction,
) -> engine::EngineAggregationFunction {
    match function {
        parser::AggregationFunction::Count => engine::EngineAggregationFunction::Count,
        parser::AggregationFunction::Sum => engine::EngineAggregationFunction::Sum,
        parser::AggregationFunction::Min => engine::EngineAggregationFunction::Min,
        parser::AggregationFunction::Max => engine::EngineAggregationFunction::Max,
        parser::AggregationFunction::Avg => engine::EngineAggregationFunction::Avg,
    }
}

fn predicate_to_engine(predicate: &parser::Predicate) -> engine::EnginePredicate {
    engine::EnginePredicate {
        path: predicate.path.clone(),
        op: operator_to_engine(&predicate.op),
        value: predicate.value.clone(),
    }
}

fn operator_to_engine(op: &parser::Operator) -> engine::EngineOperator {
    match op {
        parser::Operator::Eq => engine::EngineOperator::Eq,
        parser::Operator::Ne => engine::EngineOperator::Ne,
    }
}

fn sort_key_to_engine(key: &parser::SortKey) -> engine::EngineSortKey {
    engine::EngineSortKey {
        path: key.path.clone(),
        direction: sort_direction_to_engine(key.direction),
    }
}

fn sort_direction_to_engine(direction: parser::SortDirection) -> engine::EngineSortDirection {
    match direction {
        parser::SortDirection::Asc => engine::EngineSortDirection::Asc,
        parser::SortDirection::Desc => engine::EngineSortDirection::Desc,
    }
}

fn output_paths_for_rows(
    plan: &engine::QueryPlan,
    rows: &[DynamicObject],
) -> Option<Vec<String>> {
    match &plan.selection {
        Some(engine::EngineSelection::Paths(paths)) => Some(paths.clone()),
        Some(engine::EngineSelection::Aggregations(_)) => rows
            .first()
            .map(|row| row.fields.keys().cloned().collect())
            .or_else(|| Some(Vec::new())),
        None => None,
    }
}

fn map_output_format(format: OutputArg) -> output::OutputFormat {
    match format {
        OutputArg::Table => output::OutputFormat::Table,
        OutputArg::Json => output::OutputFormat::Json,
        OutputArg::Yaml => output::OutputFormat::Yaml,
    }
}

fn format_planner_diagnostic(diagnostic: &k8s::planner::PlannerDiagnostic) -> String {
    format!(
        "[pushdown] predicate `{}` {} was not pushed: {}",
        diagnostic.path,
        format_operator(&diagnostic.op),
        format_not_pushable_reason(&diagnostic.reason)
    )
}

fn format_k8s_diagnostic(diagnostic: &k8s::K8sDiagnostic) -> String {
    match diagnostic {
        k8s::K8sDiagnostic::SelectorFallback { reason, attempted } => {
            format!(
                "[pushdown] API rejected selectors ({}); retried without selectors (field_selector={:?}, label_selector={:?})",
                format_selector_fallback_reason(reason),
                attempted.field_selector,
                attempted.label_selector
            )
        }
    }
}

fn format_selector_fallback_reason(reason: &k8s::SelectorFallbackReason) -> &'static str {
    match reason {
        k8s::SelectorFallbackReason::ApiRejectedBadRequest => "bad request",
    }
}

fn format_not_pushable_reason(reason: &k8s::planner::NotPushableReason) -> &'static str {
    match reason {
        k8s::planner::NotPushableReason::UnsupportedPath => "unsupported path",
        k8s::planner::NotPushableReason::UnsupportedOperator => "unsupported operator",
        k8s::planner::NotPushableReason::NonStringValue => "non-string value",
        k8s::planner::NotPushableReason::UnsafeSelectorValue => "unsafe selector value",
        k8s::planner::NotPushableReason::UnsafeLabelKey => "unsafe label key",
    }
}

fn format_operator(operator: &parser::Operator) -> &'static str {
    match operator {
        parser::Operator::Eq => "==",
        parser::Operator::Ne => "!=",
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::error::{CliError, K8sError, OutputError, boxed_error};

    use super::{
        CliArgs, OutputArg, ast_to_engine_plan, format_k8s_diagnostic,
        format_planner_diagnostic, output_paths_for_rows, parse_query_tokens,
    };
    use crate::{
        dynamic_object::DynamicObject,
        engine::{
            EngineAggregationFunction, EngineOperator, EngineSelection, EngineSortDirection,
        },
        k8s::{
            K8sDiagnostic, ListQueryOptions, SelectorFallbackReason, planner::NotPushableReason,
        },
        parser::{Operator, SelectClause},
    };

    #[test]
    fn parses_flags_with_clap() {
        let args = CliArgs::parse_from([
            "kubiq",
            "-o",
            "json",
            "-d",
            "pods",
            "where",
            "metadata.name",
            "==",
            "pod-a",
        ]);

        assert!(matches!(args.output, OutputArg::Json));
        assert!(args.describe);
        assert!(!args.no_pushdown_warnings);
        assert_eq!(args.resource, "pods");
        assert_eq!(args.query.first().map(String::as_str), Some("where"));
    }

    #[test]
    fn parses_output_enum_case_insensitive() {
        let args = CliArgs::parse_from([
            "kubiq",
            "--output",
            "YAML",
            "pods",
            "where",
            "metadata.name",
            "==",
            "pod-a",
        ]);
        assert!(matches!(args.output, OutputArg::Yaml));
    }

    #[test]
    fn parses_no_pushdown_warnings_flag() {
        let args = CliArgs::parse_from([
            "kubiq",
            "--no-pushdown-warnings",
            "pods",
            "where",
            "metadata.name",
            "==",
            "pod-a",
        ]);
        assert!(args.no_pushdown_warnings);
    }

    #[test]
    fn parses_query_tokens_from_args_form() {
        let tokens = vec![
            "where".to_string(),
            "metadata.namespace".to_string(),
            "==".to_string(),
            "demo-a".to_string(),
            "select".to_string(),
            "metadata.name".to_string(),
        ];

        let ast = parse_query_tokens(&tokens).expect("must parse query tokens");
        assert_eq!(ast.predicates.len(), 1);
        assert_eq!(
            ast.select,
            Some(SelectClause::Paths(vec!["metadata.name".to_string()]))
        );
    }

    #[test]
    fn parses_query_tokens_with_order_by_from_args_form() {
        let tokens = vec![
            "where".to_string(),
            "metadata.namespace".to_string(),
            "==".to_string(),
            "demo-a".to_string(),
            "order".to_string(),
            "by".to_string(),
            "metadata.name".to_string(),
            "desc".to_string(),
        ];

        let ast = parse_query_tokens(&tokens).expect("must parse query tokens");
        let order_keys = ast.order_by.expect("must parse order by");
        assert_eq!(order_keys.len(), 1);
        assert_eq!(order_keys[0].path, "metadata.name");
        assert!(matches!(
            order_keys[0].direction,
            crate::parser::SortDirection::Desc
        ));
    }

    #[test]
    fn converts_ast_to_engine_plan() {
        let ast = crate::parser::parse_query(
            "where metadata.namespace == demo-a and spec.enabled != true \
             order by metadata.name desc select metadata.name",
        )
        .expect("must parse query");

        let plan = ast_to_engine_plan(&ast);

        assert_eq!(plan.predicates.len(), 2);
        assert_eq!(plan.predicates[0].path, "metadata.namespace");
        assert_eq!(plan.predicates[0].op, EngineOperator::Eq);
        assert_eq!(plan.predicates[1].path, "spec.enabled");
        assert_eq!(plan.predicates[1].op, EngineOperator::Ne);
        assert_eq!(
            plan.selection,
            Some(EngineSelection::Paths(vec!["metadata.name".to_string()]))
        );
        assert_eq!(
            plan.sort_keys
                .as_ref()
                .expect("sort keys must be present")
                .first()
                .expect("first key must exist")
                .direction,
            EngineSortDirection::Desc
        );
    }

    #[test]
    fn converts_aggregation_ast_to_engine_plan() {
        let ast = crate::parser::parse_query(
            "where metadata.namespace == demo-a select count(*), sum(spec.replicas)",
        )
        .expect("must parse query");

        let plan = ast_to_engine_plan(&ast);
        let Some(EngineSelection::Aggregations(expressions)) = plan.selection else {
            panic!("expected aggregation selection");
        };
        assert_eq!(expressions.len(), 2);
        assert_eq!(expressions[0].function, EngineAggregationFunction::Count);
        assert_eq!(expressions[0].path, None);
        assert_eq!(expressions[1].function, EngineAggregationFunction::Sum);
        assert_eq!(expressions[1].path.as_deref(), Some("spec.replicas"));
    }

    #[test]
    fn output_paths_for_rows_uses_projection_paths() {
        let plan = crate::engine::QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Paths(vec![
                "metadata.name".to_string(),
                "metadata.namespace".to_string(),
            ])),
            sort_keys: None,
        };

        let paths = output_paths_for_rows(&plan, &[]).expect("paths must be present");
        assert_eq!(
            paths,
            vec![
                "metadata.name".to_string(),
                "metadata.namespace".to_string()
            ]
        );
    }

    #[test]
    fn output_paths_for_rows_uses_aggregation_row_keys() {
        let plan = crate::engine::QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(Vec::new())),
            sort_keys: None,
        };

        let row = DynamicObject {
            fields: [
                ("count(*)".to_string(), serde_json::Value::from(2)),
                ("sum(spec.replicas)".to_string(), serde_json::Value::from(5)),
            ]
            .into_iter()
            .collect(),
        };

        let paths = output_paths_for_rows(&plan, &[row]).expect("paths must be present");
        assert_eq!(
            paths,
            vec!["count(*)".to_string(), "sum(spec.replicas)".to_string()]
        );
    }

    #[test]
    fn k8s_error_contains_connectivity_tip() {
        let err = CliError::K8s(K8sError::ApiUnreachable {
            stage: "discovery",
            source: crate::error::boxed_error(std::io::Error::other("dial tcp timeout")),
        });
        let rendered = err.to_string();
        assert!(rendered.contains("Kubernetes API is unreachable"));
        assert!(rendered.contains("kubectl cluster-info"));
    }

    #[test]
    fn k8s_error_not_found_contains_api_resources_tip() {
        let err = CliError::K8s(K8sError::ResourceNotFound {
            resource: "podsx".to_string(),
        });
        let rendered = err.to_string();
        assert!(rendered.contains("resource was not found"));
        assert!(rendered.contains("kubectl api-resources"));
    }

    #[test]
    fn k8s_error_includes_source_details_in_rendered_message() {
        let err = CliError::K8s(K8sError::ConfigInfer {
            source: boxed_error(std::io::Error::other(
                "no such file or directory: /tmp/missing-kubeconfig",
            )),
        });
        let rendered = err.to_string();
        assert!(rendered.contains("failed to infer kube config"));
        assert!(rendered.contains("missing-kubeconfig"));
    }

    #[test]
    fn parse_error_contains_query_example_tip() {
        let err = CliError::Parse("invalid query syntax".to_string());
        let rendered = err.to_string();
        assert!(rendered.contains("query format"));
        assert!(rendered.contains("kubiq pods where"));
    }

    #[test]
    fn k8s_error_fallback_tip_is_rendered_for_non_connectivity_failures() {
        let err = CliError::K8s(K8sError::DiscoveryRun {
            source: boxed_error(std::io::Error::other("forbidden")),
        });
        let rendered = err.to_string();
        assert!(rendered.contains("verify cluster access with `kubectl get ns`"));
    }

    #[test]
    fn output_error_contains_format_tip() {
        let source = serde_json::from_str::<serde_json::Value>("not json").expect_err("must fail");
        let err = CliError::Output(OutputError::JsonSerialize { source });
        let rendered = err.to_string();
        assert!(rendered.contains("output error"));
        assert!(rendered.contains("supported formats are `table`, `json`, `yaml`"));
    }

    #[test]
    fn formats_planner_diagnostics() {
        let diagnostic = crate::k8s::planner::PlannerDiagnostic {
            path: "spec.nodeName".to_string(),
            op: Operator::Eq,
            reason: NotPushableReason::UnsupportedPath,
        };

        let rendered = format_planner_diagnostic(&diagnostic);
        assert!(rendered.contains("spec.nodeName"));
        assert!(rendered.contains("unsupported path"));
    }

    #[test]
    fn formats_runtime_selector_fallback_diagnostics() {
        let diagnostic = K8sDiagnostic::SelectorFallback {
            reason: SelectorFallbackReason::ApiRejectedBadRequest,
            attempted: ListQueryOptions {
                field_selector: Some("metadata.namespace=demo-a".to_string()),
                label_selector: None,
            },
        };

        let rendered = format_k8s_diagnostic(&diagnostic);
        assert!(rendered.contains("retried without selectors"));
        assert!(rendered.contains("metadata.namespace=demo-a"));
    }
}
