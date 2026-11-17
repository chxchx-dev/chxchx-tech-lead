//! Launch the packaged Python CLI from the MSI's private, relocatable runtime.
use std::env;
use std::process::Command;

fn main() {
    let code = run().unwrap_or_else(|error| {
        eprintln!("No se pudo iniciar el CLI Python incluido: {error}");
        1
    });
    std::process::exit(code);
}

fn run() -> Result<i32, String> {
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let scripts = executable
        .parent()
        .ok_or("el launcher no tiene carpeta padre")?;
    let runtime = scripts
        .parent()
        .ok_or("el launcher no tiene raíz de runtime")?
        .join("python")
        .join("python.exe");
    if !runtime.is_file() {
        return Err(format!("no existe {}", runtime.display()));
    }

    Command::new(runtime)
        .arg("-c")
        .arg("from chxchx_tech_lead.cli import app; app()")
        .args(env::args_os().skip(1))
        .status()
        .map(|status| status.code().unwrap_or(1))
        .map_err(|error| error.to_string())
}
