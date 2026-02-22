fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("failed to initialize tokio runtime");
    if let Err(error) = runtime.block_on(kubiq::run_async()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
