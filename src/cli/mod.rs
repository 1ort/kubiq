use clap::{Parser, ValueEnum, error::ErrorKind};

use crate::{engine, error::CliError, k8s, output, parser};

#[derive(Clone, Debug, ValueEnum)]
enum OutputArg {
    Table,
    Json,
    Yaml,
}

#[derive(Parser, Debug)]
#[command(name = "kubiq")]
#[command(about = "Query Kubernetes resources with where/select")]
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
    let objects = k8s::list(&args.resource).map_err(CliError::K8s)?;
    let filtered = engine::evaluate(&plan, &objects);

    let detail = if args.describe {
        output::DetailLevel::Describe
    } else {
        output::DetailLevel::Summary
    };

    output::print(
        &filtered,
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::error::{CliError, K8sError};

    use super::{CliArgs, OutputArg, parse_query_tokens};

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
    fn parse_error_contains_query_example_tip() {
        let err = CliError::Parse("invalid query syntax".to_string());
        let rendered = err.to_string();
        assert!(rendered.contains("query format"));
        assert!(rendered.contains("kubiq pods where"));
    }
}
