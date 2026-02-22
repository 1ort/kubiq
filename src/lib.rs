pub mod cli;
pub mod dynamic_object;
pub mod engine;
pub mod error;
pub mod k8s;
pub mod output;
pub mod parser;

pub fn run() -> Result<(), error::CliError> {
    cli::run()
}

pub async fn run_async() -> Result<(), error::CliError> {
    cli::run_async().await
}
