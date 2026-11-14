fn main() {
    if let Err(error) = chxchx_tech::interfaces::cli::run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
