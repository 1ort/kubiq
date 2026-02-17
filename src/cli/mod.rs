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
            Self::MissingArgs => write!(
                f,
                "usage: mini-kql [--output table|json|yaml] [--describe] <resource> where <predicates> [select <paths>]"
            ),
            Self::InvalidArgs(error) => write!(f, "invalid args: {error}"),
            Self::Parse(error) => write!(f, "parse error: {error}"),
            Self::K8s(error) => write!(f, "k8s error: {error}"),
            Self::Output(error) => write!(f, "output error: {error}"),
        }
    }
}

impl std::error::Error for CliError {}

pub fn run() -> Result<(), CliError> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
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

#[cfg(test)]
mod tests {
    use crate::output::{DetailLevel, OutputFormat};

    use super::parse_cli_flags;

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
}
