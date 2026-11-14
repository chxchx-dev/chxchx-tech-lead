use std::io::{Read, Result as IoResult};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const OUTPUT_LIMIT_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(command: &[String], cwd: &Path) -> Result<CommandOutput, String> {
    let (mut child, stdout, stderr) = spawn(command, cwd)?;
    let stdout_reader = read_on_thread(stdout);
    let stderr_reader = read_on_thread(stderr);
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < COMMAND_TIMEOUT => {
                thread::sleep(Duration::from_millis(40));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "La orden MCP superó el límite de {} segundos",
                    COMMAND_TIMEOUT.as_secs()
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("No se pudo esperar la orden MCP: {error}"));
            }
        }
    }
    let status = child
        .wait()
        .map_err(|error| format!("No se pudo recoger el resultado MCP: {error}"))?;
    let stdout = receive_output(stdout_reader, "salida")?;
    let stderr = receive_output(stderr_reader, "error")?;
    Ok(CommandOutput {
        status: status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

fn spawn(
    command: &[String],
    cwd: &Path,
) -> Result<
    (
        std::process::Child,
        impl Read + Send + 'static,
        impl Read + Send + 'static,
    ),
    String,
> {
    let program = command
        .first()
        .ok_or_else(|| "La orden MCP no tiene ejecutable".to_owned())?;
    let resolved = find_executable(program)
        .ok_or_else(|| format!("No se encontró el ejecutable `{program}` en PATH"))?;
    let extension = resolved
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut process = command_builder(&resolved, &extension, command, cwd)?
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("No se pudo iniciar {program}: {error}"))?;
    let stdout = process
        .stdout
        .take()
        .ok_or_else(|| "No se capturó la salida MCP".to_owned())?;
    let stderr = process
        .stderr
        .take()
        .ok_or_else(|| "No se capturó el error MCP".to_owned())?;
    Ok((process, stdout, stderr))
}

fn command_builder(
    resolved: &Path,
    extension: &str,
    command: &[String],
    cwd: &Path,
) -> Result<Command, String> {
    #[cfg(windows)]
    if matches!(extension, "ps1" | "cmd" | "bat") {
        let arguments = serde_json::to_string(&command[1..])
            .map_err(|error| format!("No se pudieron preparar argumentos MCP: {error}"))?;
        let powershell = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .map(|root| {
                root.join("System32")
                    .join("WindowsPowerShell")
                    .join("v1.0")
                    .join("powershell.exe")
            })
            .filter(|path| path.is_file())
            .unwrap_or_else(|| PathBuf::from("powershell.exe"));
        let mut process = Command::new(powershell);
        process
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$mcpArgs = @(ConvertFrom-Json -InputObject $env:CHXCHX_MCP_ARGS); & $env:CHXCHX_MCP_SCRIPT @mcpArgs; exit $LASTEXITCODE",
            ])
            .env("CHXCHX_MCP_SCRIPT", resolved)
            .env("CHXCHX_MCP_ARGS", arguments)
            .current_dir(cwd);
        return Ok(process);
    }
    #[cfg(not(windows))]
    let _ = extension;

    let mut process = Command::new(resolved);
    process.args(&command[1..]).current_dir(cwd);
    Ok(process)
}

pub fn find_executable(name: &str) -> Option<PathBuf> {
    let requested = Path::new(name);
    if requested.components().count() > 1 {
        return executable_candidate(requested);
    }
    let path_value = std::env::var_os("PATH")?;
    std::env::split_paths(&path_value)
        .map(|directory| executable_candidate(&directory.join(name)))
        .find_map(|candidate| candidate)
}

fn executable_candidate(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_owned());
    }
    #[cfg(windows)]
    if path.extension().is_none() {
        for extension in ["exe", "com", "cmd", "bat", "ps1"] {
            let candidate = path.with_extension(extension);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn read_all(mut reader: impl Read) -> IoResult<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = OUTPUT_LIMIT_BYTES.saturating_sub(output.len());
        truncated |= read > remaining;
        output.extend_from_slice(&buffer[..read.min(remaining)]);
    }
    if truncated {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "la salida superó el límite de captura",
        ));
    }
    Ok(output)
}

fn read_on_thread(reader: impl Read + Send + 'static) -> Receiver<IoResult<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(read_all(reader));
    });
    receiver
}

fn receive_output(receiver: Receiver<IoResult<Vec<u8>>>, label: &str) -> Result<Vec<u8>, String> {
    receiver
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| format!("La lectura de {label} MCP excedió el límite"))?
        .map_err(|error| format!("No se pudo leer {label} MCP: {error}"))
}
