#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::path::PathBuf;

fn main() {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if let Err(error) = chxchx_tech::interfaces::studio::run(root) {
        eprintln!("No se pudo iniciar ChxChx Studio: {error}");
        std::process::exit(1);
    }
}
