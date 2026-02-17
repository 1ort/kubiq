fn main() {
    if let Err(error) = mini_kql::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
