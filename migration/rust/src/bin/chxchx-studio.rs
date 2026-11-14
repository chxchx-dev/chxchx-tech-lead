#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use chxchx_tech::features::startup_log;
use std::path::PathBuf;

fn main() {
    // Windows has no console here: failures and panics must also reach a file.
    startup_log::install_panic_hook();
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let file = std::env::args_os().nth(2).map(PathBuf::from);
    if let Err(error) = chxchx_tech::interfaces::studio::run_with_file(root, file) {
        let message = format!("No se pudo iniciar ChxChx Studio: {error}");
        eprintln!("{message}");
        if let Some(path) = startup_log::record(&message) {
            eprintln!("Detalle guardado en {}", path.display());
        }
        std::process::exit(1);
    }
}
