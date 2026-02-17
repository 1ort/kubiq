use crate::{engine, k8s, output, parser};

#[derive(Debug)]
pub enum CliError {
    MissingArgs,
    Parse(String),
    K8s(String),
}

impl std::fmt::Display for CliError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::MissingArgs => write!(f, "usage: mini-kql <resource> <where-clause>"),
            Self::Parse(error) => write!(f, "parse error: {error}"),
            Self::K8s(error) => write!(f, "k8s error: {error}"),
        }
    }
}

impl std::error::Error for CliError {}

pub fn run() -> Result<(), CliError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
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

    output::print_table(&filtered);
    Ok(())
}
