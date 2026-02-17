pub mod cli;
pub mod dynamic_object;
pub mod engine;
pub mod k8s;
pub mod output;
pub mod parser;

pub fn run() -> Result<(), cli::CliError> {
    cli::run()
}
