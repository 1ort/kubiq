use clap::{Parser, ValueEnum, error::ErrorKind};
use serde_json::Value;

use crate::{engine, error::CliError, k8s, output, parser};

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

    let plan = engine::build_plan(ast);
    let query_options = build_list_query_options(&plan);
    let objects = k8s::list(&args.resource, &query_options).map_err(CliError::K8s)?;
    let filtered = engine::evaluate(&plan, &objects);
    let sorted = engine::sort_objects(&plan, &filtered);

    let detail = if args.describe {
        output::DetailLevel::Describe
    } else {
        output::DetailLevel::Summary
    };

    output::print(
        &sorted,
        map_output_format(args.output),
        detail,
        plan.select_paths.as_deref(),
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

fn map_output_format(format: OutputArg) -> output::OutputFormat {
    match format {
        OutputArg::Table => output::OutputFormat::Table,
        OutputArg::Json => output::OutputFormat::Json,
        OutputArg::Yaml => output::OutputFormat::Yaml,
    }
}

fn build_list_query_options(plan: &engine::QueryPlan) -> k8s::ListQueryOptions {
    let mut field_selectors = Vec::new();
    let mut label_selectors = Vec::new();

    for predicate in &plan.predicates {
        if !matches!(predicate.op, parser::Operator::Eq) {
            continue;
        }

        let Some(value) = selector_value(&predicate.value) else {
            continue;
        };
        if !is_selector_value_safe(&value) {
            continue;
        }

        if predicate.path.eq_ignore_ascii_case("metadata.name")
            || predicate.path.eq_ignore_ascii_case("metadata.namespace")
        {
            field_selectors.push(format!("{}={value}", predicate.path));
            continue;
        }

        if let Some(label_key) = predicate.path.strip_prefix("metadata.labels.")
            && is_label_key_safe(label_key)
        {
            label_selectors.push(format!("{label_key}={value}"));
        }
    }

    k8s::ListQueryOptions {
        field_selector: join_selector_parts(field_selectors),
        label_selector: join_selector_parts(label_selectors),
    }
}

fn selector_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn is_selector_value_safe(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(',')
        && !value.contains('=')
        && !value.contains('!')
        && !value.chars().any(char::is_whitespace)
}

fn is_label_key_safe(key: &str) -> bool {
    !key.is_empty() && !key.contains(',') && !key.chars().any(char::is_whitespace)
}

fn join_selector_parts(parts: Vec<String>) -> Option<String> {
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use serde_json::Value;

    use crate::error::{CliError, K8sError, OutputError, boxed_error};

    use super::{CliArgs, OutputArg, build_list_query_options, parse_query_tokens};
    use crate::{
        engine::QueryPlan,
        parser::{Operator, Predicate},
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
        assert_eq!(ast.select_paths, Some(vec!["metadata.name".to_string()]));
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
    fn builds_server_side_field_selector_for_metadata_fields() {
        let plan = QueryPlan {
            predicates: vec![
                Predicate {
                    path: "metadata.name".to_string(),
                    op: Operator::Eq,
                    value: Value::String("worker-a".to_string()),
                },
                Predicate {
                    path: "metadata.namespace".to_string(),
                    op: Operator::Eq,
                    value: Value::String("demo-a".to_string()),
                },
            ],
            select_paths: None,
            sort_keys: None,
        };

        let options = build_list_query_options(&plan);
        assert_eq!(
            options.field_selector.as_deref(),
            Some("metadata.name=worker-a,metadata.namespace=demo-a")
        );
        assert_eq!(options.label_selector, None);
    }

    #[test]
    fn builds_server_side_label_selector_for_labels() {
        let plan = QueryPlan {
            predicates: vec![Predicate {
                path: "metadata.labels.app".to_string(),
                op: Operator::Eq,
                value: Value::String("api".to_string()),
            }],
            select_paths: None,
            sort_keys: None,
        };

        let options = build_list_query_options(&plan);
        assert_eq!(options.field_selector, None);
        assert_eq!(options.label_selector.as_deref(), Some("app=api"));
    }

    #[test]
    fn does_not_push_down_ne_or_unsafe_values() {
        let plan = QueryPlan {
            predicates: vec![
                Predicate {
                    path: "metadata.name".to_string(),
                    op: Operator::Ne,
                    value: Value::String("worker-a".to_string()),
                },
                Predicate {
                    path: "metadata.labels.app".to_string(),
                    op: Operator::Eq,
                    value: Value::String("api,core".to_string()),
                },
            ],
            select_paths: None,
            sort_keys: None,
        };

        let options = build_list_query_options(&plan);
        assert_eq!(options.field_selector, None);
        assert_eq!(options.label_selector, None);
    }

    #[test]
    fn does_not_push_down_non_string_values() {
        let plan = QueryPlan {
            predicates: vec![Predicate {
                path: "metadata.namespace".to_string(),
                op: Operator::Eq,
                value: Value::Bool(true),
            }],
            select_paths: None,
            sort_keys: None,
        };

        let options = build_list_query_options(&plan);
        assert_eq!(options.field_selector, None);
        assert_eq!(options.label_selector, None);
    }
}
