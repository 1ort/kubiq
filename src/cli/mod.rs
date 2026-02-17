use crate::{engine, k8s, output, parser};

#[derive(Debug)]
pub enum CliError {
    MissingArgs,
    InvalidArgs(String),
    Parse(String),
    K8s(String),
    Output(String),
}

impl std::fmt::Display for CliError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::MissingArgs => write!(f, "{}", usage_text()),
            Self::InvalidArgs(error) => write!(f, "invalid args: {error}\n\n{}", usage_text()),
            Self::Parse(error) => write!(f, "parse error: {error}"),
            Self::K8s(error) => write!(f, "k8s error: {error}"),
            Self::Output(error) => write!(f, "output error: {error}"),
        }
    }
}

impl std::error::Error for CliError {}

pub fn run() -> Result<(), CliError> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    if is_help_requested(&raw_args) {
        println!("{}", usage_text());
        return Ok(());
    }
    if is_version_requested(&raw_args) {
        println!("mini-kql {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let (format, detail, args) = parse_cli_flags(&raw_args)?;

    if args.len() < 2 {
        return Err(CliError::MissingArgs);
    }

    let resource = &args[0];
    let ast = if args[1].eq_ignore_ascii_case("where") {
        parser::parse_query_args(&args[1..]).map_err(CliError::Parse)?
    } else {
        let query = args[1..].join(" ");
        parser::parse_query(&query).map_err(CliError::Parse)?
    };
    let plan = engine::build_plan(ast);
    let objects = k8s::list(resource).map_err(CliError::K8s)?;
    let filtered = engine::evaluate(&plan, &objects);

    output::print(&filtered, format, detail, plan.select_paths.as_deref())
        .map_err(CliError::Output)?;
    Ok(())
}

fn parse_cli_flags(
    args: &[String]
) -> Result<(output::OutputFormat, output::DetailLevel, Vec<String>), CliError> {
    let mut format = output::OutputFormat::Table;
    let mut detail = output::DetailLevel::Summary;
    let mut positional = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let current = &args[index];

        if current == "--" {
            positional.extend_from_slice(&args[index + 1..]);
            break;
        }

        if current == "--output" || current == "-o" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| CliError::InvalidArgs("missing value for --output".to_string()))?;
            format = parse_format(value)?;
            index += 2;
            continue;
        }

        if let Some(value) = current.strip_prefix("--output=") {
            format = parse_format(value)?;
            index += 1;
            continue;
        }

        if current == "--describe" || current == "-d" {
            detail = output::DetailLevel::Describe;
            index += 1;
            continue;
        }

        if current.starts_with('-') {
            return Err(CliError::InvalidArgs(format!(
                "unknown flag '{current}'. Use --help to see available options."
            )));
        }

        positional.push(current.clone());
        index += 1;
    }

    Ok((format, detail, positional))
}

fn parse_format(value: &str) -> Result<output::OutputFormat, CliError> {
    if value.eq_ignore_ascii_case("table") {
        return Ok(output::OutputFormat::Table);
    }
    if value.eq_ignore_ascii_case("json") {
        return Ok(output::OutputFormat::Json);
    }
    if value.eq_ignore_ascii_case("yaml") {
        return Ok(output::OutputFormat::Yaml);
    }
    Err(CliError::InvalidArgs(format!(
        "unsupported output format '{value}', expected table|json|yaml"
    )))
}

fn is_help_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

fn is_version_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--version" || arg == "-V")
}

fn usage_text() -> &'static str {
    "mini-kql — query Kubernetes resources with where/select\n\
\n\
USAGE:\n\
  mini-kql [--output table|json|yaml] [--describe] <resource> where <predicates> [select <paths>]\n\
\n\
FLAGS:\n\
  -o, --output <format>   Output format: table (default), json, yaml\n\
  -d, --describe          Print full nested object fields\n\
  -h, --help              Show this help\n\
  -V, --version           Show mini-kql version\n\
\n\
EXAMPLES:\n\
  mini-kql pods where metadata.namespace == demo-a\n\
  mini-kql -o json pods where metadata.name == worker-a select metadata\n\
  mini-kql -o yaml -d pods where metadata.name == worker-a"
}

#[cfg(test)]
mod tests {
    use crate::output::{DetailLevel, OutputFormat};

    use super::{is_help_requested, is_version_requested, parse_cli_flags};

    #[test]
    fn parses_output_and_describe_flags() {
        let args = vec![
            "--output".to_string(),
            "json".to_string(),
            "--describe".to_string(),
            "pods".to_string(),
            "where".to_string(),
            "metadata.name".to_string(),
            "==".to_string(),
            "pod-a".to_string(),
        ];

        let (format, detail, positional) = parse_cli_flags(&args).expect("flags must parse");
        assert_eq!(format, OutputFormat::Json);
        assert_eq!(detail, DetailLevel::Describe);
        assert_eq!(positional[0], "pods");
    }

    #[test]
    fn parses_yaml_output_flag() {
        let args = vec![
            "--output=yaml".to_string(),
            "pods".to_string(),
            "where".to_string(),
            "metadata.name".to_string(),
            "==".to_string(),
            "pod-a".to_string(),
        ];

        let (format, _, _) = parse_cli_flags(&args).expect("flags must parse");
        assert_eq!(format, OutputFormat::Yaml);
    }

    #[test]
    fn rejects_unknown_flag() {
        let args = vec!["--unknown".to_string(), "pods".to_string()];
        assert!(parse_cli_flags(&args).is_err());
    }

    #[test]
    fn supports_double_dash_for_positional_tail() {
        let args = vec![
            "--output=json".to_string(),
            "--".to_string(),
            "pods".to_string(),
            "where".to_string(),
            "metadata.name".to_string(),
            "==".to_string(),
            "pod-a".to_string(),
        ];
        let (format, _, positional) = parse_cli_flags(&args).expect("flags must parse");
        assert_eq!(format, OutputFormat::Json);
        assert_eq!(positional[0], "pods");
    }

    #[test]
    fn detects_help_and_version_flags() {
        assert!(is_help_requested(&["-h".to_string()]));
        assert!(is_version_requested(&["--version".to_string()]));
    }
}
