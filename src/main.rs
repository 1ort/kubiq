fn main() {
    if let Err(error) = kubiq::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
