fn main() {
    if let Err(error) = chxchx_tech::interfaces::installer::run() {
        eprintln!("No se pudo abrir el instalador de ChxChx: {error}");
        std::process::exit(1);
    }
}
