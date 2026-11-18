//! Local model diagnosis and setup through Ollama and OpenCode.
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hardware {
    pub total_ram: u64,
    pub available_ram: u64,
    pub cpu_cores: u64,
    pub disk_free: u64,
    pub models_dir: String,
    pub accelerator: String,
    pub accelerated_bytes: u64,
    pub unified_memory: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ollama {
    pub installed: bool,
    pub running: bool,
    pub version: String,
    pub base_url: String,
    pub install_command: Option<String>,
    pub install_url: String,
    /// `(name, size in bytes)` of each downloaded model.
    pub models: Vec<(String, u64)>,
    pub loaded: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Candidate {
    pub tag: String,
    pub label: String,
    pub size: u64,
    pub context_k: u64,
    pub note: String,
    /// `gpu`, `partial`, `cpu` or `none`.
    pub fit: String,
    pub required: u64,
    pub disk_ok: bool,
    pub summary: String,
    pub recommended: bool,
    pub downloaded: bool,
    pub loaded: bool,
    pub configured: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub hardware: Hardware,
    pub ollama: Ollama,
    pub opencode_installed: bool,
    pub opencode_hint: String,
    pub configured: Vec<String>,
    pub context: u64,
    pub models: Vec<Candidate>,
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn parse_snapshot(output: &str) -> Result<Snapshot, String> {
    let value: Value = serde_json::from_str(output.trim())
        .map_err(|error| format!("Respuesta de modelos locales inválida: {error}"))?;
    if value.get("schema").and_then(Value::as_str) != Some("chxchx.local-models") {
        let error = string(&value, "error");
        return Err(if error.is_empty() {
            "El CLI no devolvió los modelos locales".to_owned()
        } else {
            error
        });
    }
    let (hw, ol, oc) = (&value["hardware"], &value["ollama"], &value["opencode"]);
    let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
    Ok(Snapshot {
        hardware: Hardware {
            total_ram: number(hw, "total_ram_bytes"),
            available_ram: number(hw, "available_ram_bytes"),
            cpu_cores: number(hw, "cpu_cores"),
            disk_free: number(hw, "disk_free_bytes"),
            models_dir: string(hw, "models_dir"),
            accelerator: string(hw, "accelerator"),
            accelerated_bytes: number(hw, "accelerated_bytes"),
            unified_memory: flag(hw, "unified_memory"),
        },
        ollama: Ollama {
            installed: flag(ol, "installed"),
            running: flag(ol, "running"),
            version: string(ol, "version"),
            base_url: string(ol, "base_url"),
            install_command: ol["install"]["command"].as_str().map(str::to_owned),
            install_url: string(&ol["install"], "url"),
            models: list(&ol["models"])
                .iter()
                .map(|m| (string(m, "name"), number(m, "size_bytes")))
                .collect(),
            loaded: list(&ol["loaded"])
                .iter()
                .filter_map(|m| m.get("name").and_then(Value::as_str).map(str::to_owned))
                .collect(),
        },
        opencode_installed: flag(oc, "installed"),
        opencode_hint: string(oc, "install_hint"),
        configured: list(&oc["configured_models"])
            .iter()
            .filter_map(|m| m.as_str().map(str::to_owned))
            .collect(),
        context: number(&value, "context"),
        models: list(&value["models"])
            .iter()
            .map(|m| Candidate {
                tag: string(m, "tag"),
                label: string(m, "label"),
                size: number(m, "size_bytes"),
                context_k: number(m, "context_k"),
                note: string(m, "note"),
                fit: string(m, "fit"),
                required: number(m, "required_bytes"),
                disk_ok: flag(m, "disk_ok"),
                summary: string(m, "summary"),
                recommended: flag(m, "recommended"),
                downloaded: flag(m, "downloaded"),
                loaded: flag(m, "loaded"),
                configured: flag(m, "configured"),
            })
            .filter(|m| !m.tag.is_empty())
            .collect(),
    })
}

/// Hardware, Ollama and the catalog. Blocking: run it off the UI thread.
pub fn snapshot(project: &Path) -> Result<Snapshot, String> {
    let ollama_path = super::executable_search::find_executable("ollama");
    snapshot_with(project, ollama_path.as_deref())
}

/// Injectable executable path keeps local diagnosis testable without a running Ollama server.
pub fn snapshot_with(project: &Path, ollama_executable: Option<&Path>) -> Result<Snapshot, String> {
    let (total_ram, available_ram) = memory_bytes();
    let models_dir = models_directory();
    let (accelerated_bytes, accelerator, unified_memory) = accelerator_info(total_ram);
    let hardware = Hardware {
        total_ram,
        available_ram,
        cpu_cores: std::thread::available_parallelism().map_or(1, |n| n.get() as u64),
        disk_free: free_disk_bytes(&models_dir),
        models_dir: models_dir.display().to_string(),
        accelerator,
        accelerated_bytes,
        unified_memory,
    };
    let installed = ollama_executable.is_some();
    let mut ollama = Ollama {
        installed,
        base_url: ollama_endpoint(),
        install_command: if cfg!(target_os = "linux") {
            Some("curl -fsSL https://ollama.com/install.sh | sh".into())
        } else {
            None
        },
        install_url: "https://ollama.com/download".into(),
        ..Ollama::default()
    };
    if let Some(executable) = ollama_executable {
        if let Ok(output) = run_limited(executable, &["--version"], Duration::from_secs(5)) {
            ollama.version = output.trim().to_owned();
        }
        if let Ok(output) = run_limited(executable, &["list"], Duration::from_secs(8)) {
            ollama.running = true;
            ollama.models = parse_model_table(&output);
        }
        if let Ok(output) = run_limited(executable, &["ps"], Duration::from_secs(5)) {
            ollama.loaded = parse_model_table(&output)
                .into_iter()
                .map(|(name, _)| name)
                .collect();
        }
    }
    let configured = configured_models(project);
    let opencode_installed = super::executable_search::find_executable("opencode").is_some();
    let context = 32_768;
    let downloaded = ollama
        .models
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    let loaded = ollama.loaded.iter().cloned().collect::<BTreeSet<_>>();
    let configured_set = configured.iter().cloned().collect::<BTreeSet<_>>();
    let mut models = model_catalog()
        .into_iter()
        .map(|(tag, label, size, context_k, note)| {
            let required =
                ((size as f64 * 1.05) + (context as f64 / 1024.0 * 64.0 * 1024.0 * 1024.0)) as u64;
            let fit = assess_fit(size, required, &hardware);
            let disk_ok = hardware.disk_free as f64 >= size as f64 * 1.05;
            let mut summary = match fit {
                "gpu" => "Cabe en la GPU: velocidad completa.".to_owned(),
                "partial" => "Se reparte entre GPU y RAM: funciona, pero más lento.".to_owned(),
                "cpu" => "Solo CPU: lento, aceptable para tareas pequeñas.".to_owned(),
                _ => format!(
                    "Necesita ~{:.0} GB de memoria y no cabe en este equipo.",
                    required as f64 / 1024_f64.powi(3)
                ),
            };
            if fit != "none" && !disk_ok {
                summary.push_str(&format!(
                    " Falta espacio en disco (libres {:.0} GB).",
                    hardware.disk_free as f64 / 1024_f64.powi(3)
                ));
            }
            Candidate {
                tag: tag.to_owned(),
                label: label.to_owned(),
                size,
                context_k,
                note: note.to_owned(),
                fit: fit.into(),
                required,
                disk_ok,
                summary,
                recommended: false,
                downloaded: downloaded.contains(tag),
                loaded: loaded
                    .iter()
                    .any(|name| name == tag || name.starts_with(&format!("{tag}-ctx"))),
                configured: configured_set
                    .iter()
                    .any(|name| name == tag || name.starts_with(&format!("{tag}-ctx"))),
            }
        })
        .collect::<Vec<_>>();
    let preferred = ["gpu", "partial", "cpu"];
    if let Some(best) = preferred
        .iter()
        .find_map(|fit| models.iter().position(|m| m.fit == *fit && m.disk_ok))
    {
        models[best].recommended = true;
    }
    let snapshot = Snapshot {
        hardware,
        ollama,
        opencode_installed,
        opencode_hint: "npm install -g opencode-ai".into(),
        configured,
        context,
        models,
    };
    Ok(snapshot)
}

fn model_catalog() -> Vec<(&'static str, &'static str, u64, u64, &'static str)> {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    [
        (
            "gpt-oss:120b",
            "gpt-oss 120B",
            65.0,
            128,
            "Muy grande: requiere workstation o más de 64 GB de memoria.",
        ),
        (
            "qwen3-coder:30b",
            "Qwen3 Coder 30B",
            19.0,
            256,
            "Modelo de código de uso general, el mejor equilibrio si cabe.",
        ),
        (
            "devstral:24b",
            "Devstral 24B",
            14.0,
            128,
            "Pensado para agentes de código.",
        ),
        (
            "gpt-oss:20b",
            "gpt-oss 20B",
            14.0,
            128,
            "Razonamiento y herramientas.",
        ),
        (
            "qwen2.5-coder:32b",
            "Qwen2.5 Coder 32B",
            20.0,
            32,
            "Código, ventana de 32K.",
        ),
        (
            "qwen2.5-coder:14b",
            "Qwen2.5 Coder 14B",
            9.0,
            32,
            "Opción media para 16 GB de memoria.",
        ),
        (
            "qwen2.5-coder:7b",
            "Qwen2.5 Coder 7B",
            4.7,
            32,
            "Ligero; razona menos y falla más con herramientas.",
        ),
        (
            "qwen2.5-coder:3b",
            "Qwen2.5 Coder 3B",
            1.9,
            32,
            "Mínimo viable; solo tareas simples.",
        ),
    ]
    .into_iter()
    .map(|(tag, label, size, ctx, note)| (tag, label, (size * GIB) as u64, ctx, note))
    .collect()
}

fn run_limited(executable: &Path, args: &[&str], timeout: Duration) -> Result<String, String> {
    let mut process = std::process::Command::new(executable);
    process
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut process);
    let mut child = process.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("sin salida")?;
    let stderr = child.stderr.take().ok_or("sin error")?;
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut reader = stdout;
        let _ = reader.read_to_end(&mut bytes);
        bytes
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut reader = stderr;
        let _ = reader.read_to_end(&mut bytes);
        bytes
    });
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let stdout = stdout_reader.join().unwrap_or_default();
            let stderr = stderr_reader.join().unwrap_or_default();
            if status.success() {
                return Ok(String::from_utf8_lossy(&stdout).into_owned());
            }
            return Err(String::from_utf8_lossy(&stderr).into_owned());
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err("Ollama tardó demasiado en responder".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn hide_console(command: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = command;
}

fn parse_model_table(output: &str) -> Vec<(String, u64)> {
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let columns = line.split_whitespace().collect::<Vec<_>>();
            if columns.len() < 3 {
                return None;
            }
            let amount = columns[2].parse::<f64>().ok()?;
            let multiplier = match columns
                .get(3)
                .copied()
                .unwrap_or("")
                .to_ascii_uppercase()
                .as_str()
            {
                "GB" => 1024_f64.powi(3),
                "MB" => 1024_f64.powi(2),
                "KB" => 1024_f64,
                _ => 1.0,
            };
            Some((columns[0].to_owned(), (amount * multiplier) as u64))
        })
        .collect()
}

fn ollama_endpoint() -> String {
    let value = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "127.0.0.1:11434".into());
    let endpoint = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .unwrap_or(&value)
        .trim_end_matches('/');
    if endpoint.ends_with(":11434") {
        format!("http://{endpoint}")
    } else {
        format!("http://{endpoint}:11434")
    }
}

fn models_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("OLLAMA_MODELS") {
        let path = PathBuf::from(path);
        if path.starts_with("~")
            && let Some(home) = std::env::home_dir()
        {
            return home.join(path.strip_prefix("~").unwrap_or(&path));
        }
        return path;
    }
    let home = std::env::home_dir().unwrap_or_else(std::env::temp_dir);
    let user = home.join(".ollama/models");
    if cfg!(target_os = "linux") && !user.exists() {
        PathBuf::from("/usr/share/ollama/.ollama/models")
    } else {
        user
    }
}

fn free_disk_bytes(path: &Path) -> u64 {
    #[cfg(unix)]
    {
        let mut existing = path;
        while !existing.exists() {
            if let Some(parent) = existing.parent() {
                existing = parent;
            } else {
                break;
            }
        }
        if let Ok(output) = std::process::Command::new("df")
            .args(["-Pk", existing.to_string_lossy().as_ref()])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(kb) = text
                .lines()
                .nth(1)
                .and_then(|l| l.split_whitespace().nth(3))
                .and_then(|s| s.parse::<u64>().ok())
            {
                return kb.saturating_mul(1024);
            }
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetDiskFreeSpaceExW(
                directory: *const u16,
                available_to_caller: *mut u64,
                total_bytes: *mut u64,
                total_free_bytes: *mut u64,
            ) -> i32;
        }
        let mut existing = path;
        while !existing.exists() {
            if let Some(parent) = existing.parent() {
                existing = parent;
            } else {
                break;
            }
        }
        let directory = existing
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let mut available = 0_u64;
        // SAFETY: the path is a valid NUL-terminated UTF-16 buffer and all output pointers
        // point to initialized u64 values that remain alive for the call.
        if unsafe {
            GetDiskFreeSpaceExW(
                directory.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } != 0
        {
            return available;
        }
    }
    0
}

fn memory_bytes() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = fs::read_to_string("/proc/meminfo") {
            let value = |key: &str| {
                text.lines()
                    .find(|l| l.starts_with(key))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|n| n.parse::<u64>().ok())
                    .unwrap_or(0)
                    .saturating_mul(1024)
            };
            return (value("MemTotal:"), value("MemAvailable:"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        let total = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
            .unwrap_or(0);
        let page = 4096_u64;
        let free = std::process::Command::new("vm_stat")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map(|s| {
                s.lines()
                    .filter(|l| {
                        l.contains("Pages free")
                            || l.contains("Pages inactive")
                            || l.contains("Pages speculative")
                    })
                    .filter_map(|l| {
                        l.split_whitespace()
                            .last()?
                            .trim_end_matches('.')
                            .parse::<u64>()
                            .ok()
                    })
                    .sum::<u64>()
                    .saturating_mul(page)
            })
            .unwrap_or(0);
        return (total, free);
    }
    #[cfg(windows)]
    {
        #[repr(C)]
        struct MemoryStatusEx {
            length: u32,
            memory_load: u32,
            total_physical: u64,
            available_physical: u64,
            total_page_file: u64,
            available_page_file: u64,
            total_virtual: u64,
            available_virtual: u64,
            available_extended_virtual: u64,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
        }
        let mut status = MemoryStatusEx {
            length: std::mem::size_of::<MemoryStatusEx>() as u32,
            memory_load: 0,
            total_physical: 0,
            available_physical: 0,
            total_page_file: 0,
            available_page_file: 0,
            total_virtual: 0,
            available_virtual: 0,
            available_extended_virtual: 0,
        };
        // SAFETY: `status` has the documented MEMORYSTATUSEX layout and its length is set.
        if unsafe { GlobalMemoryStatusEx(&mut status) } != 0 {
            return (status.total_physical, status.available_physical);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        (0, 0)
    }
}

fn accelerator_info(total_ram: u64) -> (u64, String, bool) {
    if cfg!(target_os = "macos") && std::env::consts::ARCH == "aarch64" {
        return (
            (total_ram as f64 * 0.66) as u64,
            "Apple Silicon (memoria unificada)".into(),
            true,
        );
    }
    let mut gpus: Vec<(String, u64)> = Vec::new();
    if let Some(exe) = super::executable_search::find_executable("nvidia-smi")
        && let Ok(output) = run_limited(
            &exe,
            &[
                "--query-gpu=name,memory.total",
                "--format=csv,noheader,nounits",
            ],
            Duration::from_secs(5),
        )
    {
        for line in output.lines() {
            let Some((name, memory)) = line.rsplit_once(',') else {
                continue;
            };
            if let Ok(mib) = memory.trim().parse::<u64>()
                && !name.trim().is_empty()
            {
                gpus.push((name.trim().to_owned(), mib.saturating_mul(1024 * 1024)));
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(cards) = fs::read_dir("/sys/class/drm") {
            let mut cards = cards.flatten().collect::<Vec<_>>();
            cards.sort_by_key(|card| card.file_name());
            for card in cards {
                let device = card.path().join("device");
                let vendor = fs::read_to_string(device.join("vendor")).unwrap_or_default();
                if vendor.trim() == "0x1002" {
                    let bytes = fs::read_to_string(device.join("mem_info_vram_total"))
                        .ok()
                        .and_then(|s| s.trim().parse::<u64>().ok())
                        .unwrap_or(0);
                    if bytes > 0 {
                        gpus.push(("AMD Radeon".into(), bytes));
                    }
                }
            }
        }
    }
    let accelerated = gpus.iter().map(|(_, bytes)| *bytes).max().unwrap_or(0);
    if accelerated == 0 {
        (0, "sin GPU compatible".into(), false)
    } else {
        let names = gpus
            .iter()
            .map(|(name, bytes)| format!("{name} ({:.0} GB)", *bytes as f64 / 1024_f64.powi(3)))
            .collect::<Vec<_>>()
            .join(", ");
        (accelerated, names, false)
    }
}

fn assess_fit(size: u64, required: u64, hardware: &Hardware) -> &'static str {
    let accelerated = hardware.accelerated_bytes as f64;
    let available = hardware.available_ram as f64;
    if accelerated > 0.0
        && required as f64 <= accelerated * 0.95
        && (!hardware.unified_memory || required <= hardware.available_ram)
    {
        "gpu"
    } else if accelerated > 0.0
        && !hardware.unified_memory
        && required as f64 <= accelerated + available * 0.6
    {
        "partial"
    } else if required as f64 <= available * 0.7 && size <= 10 * 1024 * 1024 * 1024 {
        "cpu"
    } else {
        "none"
    }
}

fn configured_models(project: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(project.join("opencode.json")) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    value
        .get("provider")
        .and_then(|p| p.get("ollama"))
        .and_then(|o| o.get("models"))
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Downloads the model, widens its context and connects it to OpenCode.
    Setup { tag: String, set_default: bool },
    /// Deletes it from Ollama and from the project's `opencode.json`.
    Remove { tag: String },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OperationOptions {
    pub confirmed: bool,
    pub dry_run: bool,
    pub force: bool,
}

/// A native Ollama setup/removal job. Its worker reports JSON events to Studio.
pub struct Job {
    pub op: Op,
    events: Receiver<Value>,
    cancel: Arc<AtomicBool>,
    pub exit: Option<i32>,
}

impl Job {
    pub fn start(
        project: &Path,
        op: Op,
        context: u64,
        trusted: bool,
        options: OperationOptions,
        snapshot: Snapshot,
    ) -> Result<Self, String> {
        if !trusted {
            return Err(
                "Concede confianza al proyecto antes de configurar modelos locales.".into(),
            );
        }
        let executable = super::executable_search::find_executable("ollama").ok_or_else(|| {
            "Ollama no está instalado. Instálalo con el método oficial y vuelve a intentarlo."
                .to_owned()
        })?;
        Self::start_with(project, op, context, executable, trusted, options, snapshot)
    }

    pub(crate) fn start_with(
        project: &Path,
        op: Op,
        context: u64,
        executable: PathBuf,
        trusted: bool,
        options: OperationOptions,
        snapshot: Snapshot,
    ) -> Result<Self, String> {
        if !trusted {
            return Err(
                "Concede confianza al proyecto antes de configurar modelos locales.".into(),
            );
        }
        let project = project
            .canonicalize()
            .map_err(|error| format!("No se pudo resolver el proyecto: {error}"))?;
        let (sender, events) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let worker_op = op.clone();
        std::thread::spawn(move || {
            let result = run_operation(OperationContext {
                project: &project,
                op: &worker_op,
                context,
                ollama: &executable,
                snapshot,
                options,
                events: &sender,
                cancel: &worker_cancel,
            });
            let code = match result {
                Ok(()) => 0,
                Err(error) => {
                    let _ = sender.send(serde_json::json!({"type":"error","message":error}));
                    if worker_cancel.load(Ordering::Relaxed) {
                        130
                    } else {
                        1
                    }
                }
            };
            let _ = sender.send(serde_json::json!({"type":"_finished","code":code}));
            super::wake::notify_after_end();
        });
        Ok(Self {
            op,
            events,
            cancel,
            exit: None,
        })
    }

    pub fn poll(&mut self) -> Vec<Value> {
        let mut found = Vec::new();
        for mut event in self.events.try_iter() {
            if event.get("type").and_then(Value::as_str) == Some("_finished") {
                self.exit = Some(event.get("code").and_then(Value::as_i64).unwrap_or(-1) as i32);
            } else {
                super::wake::notify();
                found.push(std::mem::take(&mut event));
            }
        }
        found
    }

    pub fn is_running(&self) -> bool {
        self.exit.is_none()
    }

    /// Stops the work (a download stops when its connection closes).
    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        if self.exit.is_none() {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }
}

struct OperationContext<'a> {
    project: &'a Path,
    op: &'a Op,
    context: u64,
    ollama: &'a Path,
    snapshot: Snapshot,
    options: OperationOptions,
    events: &'a std::sync::mpsc::Sender<Value>,
    cancel: &'a AtomicBool,
}

fn run_operation(operation: OperationContext<'_>) -> Result<(), String> {
    let OperationContext {
        project,
        op,
        context,
        ollama,
        snapshot,
        options,
        events,
        cancel,
    } = operation;
    if cancel.load(Ordering::Relaxed) {
        return Err("Operación cancelada".into());
    }
    if !snapshot.ollama.installed {
        return Err(
            "Ollama no está instalado. Instálalo con el método oficial y vuelve a intentarlo."
                .into(),
        );
    }
    if !snapshot.ollama.running && !options.dry_run {
        return Err(format!(
            "Ollama no responde en {}. Inicia la aplicación o `ollama serve`.",
            snapshot.ollama.base_url
        ));
    }
    if matches!(op, Op::Setup { .. }) && context < 8192 {
        return Err("El contexto mínimo de un modelo local es 8192 tokens.".into());
    }
    match op {
        Op::Setup { tag, set_default } => {
            let (label, size, fit, disk_ok) = snapshot
                .models
                .iter()
                .find(|model| model.tag == *tag)
                .map(|model| {
                    (
                        model.label.clone(),
                        model.size,
                        model.fit.clone(),
                        model.disk_ok,
                    )
                })
                .ok_or_else(|| format!("`{tag}` no está en el catálogo local."))?;
            if fit == "none" && !options.force {
                return Err(format!(
                    "{label} no cabe en este equipo; la estimación es: {}",
                    snapshot
                        .models
                        .iter()
                        .find(|m| m.tag == *tag)
                        .map_or("sin memoria suficiente", |m| m.summary.as_str())
                ));
            }
            if !disk_ok {
                return Err(format!("No hay espacio suficiente para descargar {tag}."));
            }
            if !snapshot.ollama.models.iter().any(|(name, _)| name == tag) {
                if !options.confirmed && !options.dry_run {
                    return Err(format!(
                        "Descargar {tag} usa disco y ancho de banda. Confirma con --yes para continuar."
                    ));
                }
                let _ = events.send(serde_json::json!({"type":"step","step":"download","model":tag,"size_bytes":size}));
                if options.dry_run {
                    let _ = events.send(serde_json::json!({
                        "type":"notice",
                        "message":format!("DRY RUN: se descargarían ~{} de {tag}.", size_label(size))
                    }));
                } else {
                    run_ollama(ollama, &["pull", tag], tag, events, cancel)?;
                }
            }
            let variant = format!("{tag}-ctx{}k", context / 1024);
            if !snapshot
                .ollama
                .models
                .iter()
                .any(|(name, _)| name == &variant)
            {
                let _ = events.send(serde_json::json!({"type":"step","step":"context","model":variant,"context":context}));
                if options.dry_run {
                    let _ = events.send(serde_json::json!({
                        "type":"notice",
                        "message":format!("DRY RUN: se crearía {variant} con contexto de {context} tokens.")
                    }));
                } else {
                    create_context_variant(ollama, tag, &variant, context, events, cancel)?;
                }
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("Operación cancelada".into());
            }
            let _ = events.send(serde_json::json!({"type":"step","step":"config","model":variant}));
            let (before, after) = plan_model_config(
                project,
                &variant,
                &format!("{} ({}K)", label, context / 1024),
                context,
                &snapshot.ollama.base_url,
                *set_default,
            )?;
            let changed = before != after;
            let backup = if changed && options.dry_run {
                let _ = events.send(serde_json::json!({
                    "type":"preview",
                    "path":project.join("opencode.json"),
                    "content":after
                }));
                None
            } else if changed {
                write_opencode(project, &before, &after)?
            } else {
                None
            };
            let _ = events.send(serde_json::json!({"type":"done","model":format!("ollama/{variant}"),"changed":changed,"backup":backup,"dry_run":options.dry_run}));
        }
        Op::Remove { tag } => {
            if !snapshot.models.iter().any(|model| model.tag == *tag) {
                return Err(format!("`{tag}` no está en el catálogo local."));
            }
            let mut names = snapshot
                .ollama
                .models
                .iter()
                .map(|(name, _)| name)
                .filter(|name| name.starts_with(&format!("{tag}-ctx")))
                .cloned()
                .collect::<Vec<_>>();
            names.sort_by(|a, b| b.cmp(a));
            if snapshot.ollama.models.iter().any(|(name, _)| name == tag) {
                names.push(tag.clone());
            }
            let before = read_opencode(project)?;
            let after = remove_from_config(&before, tag)?;
            if names.is_empty() && after == before {
                return Err(format!(
                    "{tag} no está descargado ni configurado en este proyecto."
                ));
            }
            if !options.confirmed && !options.dry_run {
                return Err(format!(
                    "Quitar {tag} borra el modelo de Ollama y lo desconecta de OpenCode. Confirma con --yes para continuar."
                ));
            }
            for name in &names {
                let _ =
                    events.send(serde_json::json!({"type":"step","step":"delete","model":name}));
                if options.dry_run {
                    let _ = events.send(serde_json::json!({
                        "type":"notice",
                        "message":format!("DRY RUN: se borraría {name} de Ollama.")
                    }));
                } else {
                    run_ollama(ollama, &["rm", name], name, events, cancel)?;
                }
            }
            let _ = events.send(serde_json::json!({"type":"step","step":"config","model":tag}));
            let backup = if before != after && options.dry_run {
                let _ = events.send(serde_json::json!({
                    "type":"preview",
                    "path":project.join("opencode.json"),
                    "content":after
                }));
                None
            } else if before != after {
                write_opencode(project, &before, &after)?
            } else {
                None
            };
            let _ = events.send(
                serde_json::json!({"type":"done","model":tag,"removed":names,"backup":backup,"dry_run":options.dry_run}),
            );
        }
    }
    Ok(())
}

fn run_ollama(
    executable: &Path,
    args: &[&str],
    model: &str,
    events: &std::sync::mpsc::Sender<Value>,
    cancel: &AtomicBool,
) -> Result<String, String> {
    run_ollama_with_timeout(executable, args, model, events, cancel, None)
}

fn run_ollama_with_timeout(
    executable: &Path,
    args: &[&str],
    model: &str,
    events: &std::sync::mpsc::Sender<Value>,
    cancel: &AtomicBool,
    timeout: Option<Duration>,
) -> Result<String, String> {
    let mut process = std::process::Command::new(executable);
    process
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut process);
    let mut child = process
        .spawn()
        .map_err(|error| format!("No pude iniciar Ollama: {error}"))?;
    let (sender, receiver) = channel::<Vec<u8>>();
    let streams: Vec<Box<dyn Read + Send>> = [
        child
            .stdout
            .take()
            .map(|stream| Box::new(stream) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|stream| Box::new(stream) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .collect();
    for stream in streams {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let mut reader = stream;
            let mut buffer = [0_u8; 4096];
            loop {
                match std::io::Read::read(&mut reader, &mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(size) => {
                        if sender.send(buffer[..size].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }
    drop(sender);
    let mut output = String::new();
    let mut status_line = String::new();
    let deadline = timeout.map(|limit| std::time::Instant::now() + limit);
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Operación cancelada".into());
        }
        while let Ok(chunk) = receiver.try_recv() {
            append_ollama_output(&chunk, model, events, &mut output, &mut status_line);
        }
        if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("`ollama create` tardó más de 10 minutos.".into());
        }
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(status) => {
                while let Ok(chunk) = receiver.recv_timeout(Duration::from_secs(1)) {
                    append_ollama_output(&chunk, model, events, &mut output, &mut status_line);
                }
                return if status.success() {
                    Ok(output)
                } else {
                    let detail = output
                        .lines()
                        .rev()
                        .take(8)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n");
                    Err(if detail.trim().is_empty() {
                        "Ollama terminó el comando con error.".into()
                    } else {
                        detail
                    })
                };
            }
            None => std::thread::sleep(Duration::from_millis(40)),
        }
    }
}

fn append_ollama_output(
    chunk: &[u8],
    model: &str,
    events: &std::sync::mpsc::Sender<Value>,
    output: &mut String,
    status_line: &mut String,
) {
    let text = String::from_utf8_lossy(chunk);
    output.push_str(&text);
    status_line.push_str(&text);
    let segments = status_line
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if let Some(last) = segments.last() {
        let percent = last
            .split_whitespace()
            .find_map(|part| part.strip_suffix('%')?.parse::<u64>().ok());
        let _ = events.send(serde_json::json!({"type":"progress","model":model,"status":last,"completed":percent.unwrap_or(0),"total":if percent.is_some(){100}else{0}}));
    }
    if let Some(index) = status_line.rfind(['\r', '\n']) {
        status_line.drain(..=index);
    }
}

fn create_context_variant(
    executable: &Path,
    base: &str,
    name: &str,
    context: u64,
    events: &std::sync::mpsc::Sender<Value>,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let folder = std::env::temp_dir().join(format!(
        "chxchx-modelfile-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let file = folder.join("Modelfile");
    fs::write(&file, format!("FROM {base}\nPARAMETER num_ctx {context}\n"))
        .map_err(|error| error.to_string())?;
    let file = file.to_string_lossy().into_owned();
    let result = run_ollama_with_timeout(
        executable,
        &["create", name, "-f", &file],
        name,
        events,
        cancel,
        Some(Duration::from_secs(600)),
    );
    let _ = fs::remove_dir_all(folder);
    result.map(|_| ())
}

fn read_opencode(project: &Path) -> Result<String, String> {
    let path = project.join("opencode.json");
    if !path.exists() && project.join("opencode.jsonc").exists() {
        return Err(
            "El proyecto usa opencode.jsonc (con comentarios); edítalo a mano para no perderlos."
                .into(),
        );
    }
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.to_string()),
    }
}

fn plan_model_config(
    project: &Path,
    tag: &str,
    label: &str,
    context: u64,
    endpoint: &str,
    set_default: bool,
) -> Result<(String, String), String> {
    let before = read_opencode(project)?;
    let mut config = if before.trim().is_empty() {
        serde_json::Map::new()
    } else {
        let value: Value = serde_json::from_str(&before).map_err(|error| {
            format!(
                "opencode.json no es JSON válido ({}); no lo modifico.",
                error
            )
        })?;
        value
            .as_object()
            .cloned()
            .ok_or_else(|| "opencode.json debe contener un objeto JSON.".to_owned())?
    };
    config
        .entry("$schema")
        .or_insert_with(|| json!("https://opencode.ai/config.json"));
    let provider = config
        .entry("provider")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("`provider` en opencode.json debe ser un objeto")?;
    let ollama = provider
        .entry("ollama")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("`ollama` en opencode.json debe ser un objeto")?;
    ollama
        .entry("npm")
        .or_insert_with(|| json!("@ai-sdk/openai-compatible"));
    ollama
        .entry("name")
        .or_insert_with(|| json!("Ollama (local)"));
    let options = ollama
        .entry("options")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("`options` de Ollama debe ser un objeto")?;
    options
        .entry("baseURL")
        .or_insert_with(|| json!(format!("{}/v1", endpoint.trim_end_matches('/'))));
    let models = ollama
        .entry("models")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("`models` de Ollama debe ser un objeto")?;
    models.insert(
        tag.to_owned(),
        json!({"name":label,"limit":{"context":context,"output":8192.min(context/4)}}),
    );
    if set_default {
        config.insert("model".into(), json!(format!("ollama/{tag}")));
    }
    let after = format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(config)).map_err(|error| error.to_string())?
    );
    Ok((before, after))
}

fn remove_from_config(before: &str, tag: &str) -> Result<String, String> {
    if before.trim().is_empty() {
        return Ok(before.to_owned());
    }
    let mut data: Value = serde_json::from_str(before).map_err(|error| {
        format!(
            "opencode.json no es JSON válido ({}); no lo modifico.",
            error
        )
    })?;
    let Some(root) = data.as_object_mut() else {
        return Err("opencode.json debe contener un objeto JSON.".into());
    };
    let removed = {
        let Some(provider) = root.get_mut("provider").and_then(Value::as_object_mut) else {
            return Ok(before.to_owned());
        };
        let Some(ollama) = provider.get_mut("ollama").and_then(Value::as_object_mut) else {
            return Ok(before.to_owned());
        };
        let Some(models) = ollama.get_mut("models").and_then(Value::as_object_mut) else {
            return Ok(before.to_owned());
        };
        let removed = models
            .keys()
            .filter(|name| *name == tag || name.starts_with(&format!("{tag}-ctx")))
            .cloned()
            .collect::<Vec<_>>();
        for name in &removed {
            models.remove(name);
        }
        removed
    };
    if removed.is_empty() {
        return Ok(before.to_owned());
    }
    if root
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(|name| {
            removed
                .iter()
                .any(|model| name == format!("ollama/{model}"))
        })
    {
        root.remove("model");
    }
    let remove_provider = root
        .get("provider")
        .and_then(|p| p.get("ollama"))
        .and_then(Value::as_object)
        .is_some_and(|ollama| {
            ollama
                .get("models")
                .and_then(Value::as_object)
                .is_some_and(serde_json::Map::is_empty)
                && ollama
                    .keys()
                    .all(|key| ["npm", "name", "options", "models"].contains(&key.as_str()))
        });
    if remove_provider
        && let Some(provider) = root.get_mut("provider").and_then(Value::as_object_mut)
    {
        provider.remove("ollama");
        if provider.is_empty() {
            root.remove("provider");
        }
    }
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&data).map_err(|error| error.to_string())?
    ))
}

fn write_opencode(project: &Path, before: &str, after: &str) -> Result<Option<String>, String> {
    let path = project.join("opencode.json");
    if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("No se reemplaza un opencode.json enlazado".into());
    }
    if read_opencode(project)? != before {
        return Err(
            "opencode.json cambió mientras se preparaba el modelo; no se sobrescribió".into(),
        );
    }
    let backup = if path.is_file() {
        Some(backup_opencode(project, &path)?)
    } else {
        None
    };
    let temp = project.join(format!(
        ".opencode-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&temp, after).map_err(|error| error.to_string())?;
    replace_file(&temp, &path).inspect_err(|_| {
        let _ = fs::remove_file(&temp);
    })?;
    Ok(backup)
}

fn backup_opencode(project: &Path, source: &Path) -> Result<String, String> {
    let home = crate::features::project_profiles::application_home()
        .ok_or("No se pudo resolver el directorio de ChxChx para el backup")?;
    let name = project.file_name().unwrap_or_default().to_string_lossy();
    use sha1::Digest as _;
    let digest = sha1::Sha1::digest(project.display().to_string().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let key = format!("{}-{}", name, &digest[..8]);
    let target = home
        .join("backups-opencode")
        .join(key)
        .join(
            time::OffsetDateTime::now_utc()
                .unix_timestamp_nanos()
                .to_string(),
        )
        .join("opencode.json");
    let parent = target
        .parent()
        .ok_or("No se pudo resolver destino de backup")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::copy(source, &target)
        .map_err(|error| format!("No se pudo respaldar opencode.json: {error}"))?;
    Ok(target.display().to_string())
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
        const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
        unsafe extern "system" {
            fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
        }
        let from = source
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let to = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: both paths are NUL-terminated UTF-16 buffers valid for the duration of the call.
        let ok = unsafe {
            MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, destination).map_err(|error| error.to_string())
    }
}

/// `7.4 GB` / `820 MB`.
pub fn size_label(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.1} GB", value / GIB)
    } else {
        format!("{:.0} MB", value / (1024.0 * 1024.0))
    }
}

/// Versioned JSON contract shared with the previous local-models bridge.
pub fn snapshot_json(snapshot: &Snapshot) -> Value {
    let loaded = snapshot
        .ollama
        .loaded
        .iter()
        .map(|name| {
            let size = snapshot
                .ollama
                .models
                .iter()
                .find(|(model, _)| model == name)
                .map_or(0, |(_, size)| *size);
            json!({"name":name,"size_bytes":size,"vram_bytes":0})
        })
        .collect::<Vec<_>>();
    json!({
        "schema":"chxchx.local-models",
        "schema_version":1,
        "hardware":{
            "total_ram_bytes":snapshot.hardware.total_ram,
            "available_ram_bytes":snapshot.hardware.available_ram,
            "cpu_cores":snapshot.hardware.cpu_cores,
            "disk_free_bytes":snapshot.hardware.disk_free,
            "models_dir":snapshot.hardware.models_dir,
            "accelerator":snapshot.hardware.accelerator,
            "accelerated_bytes":snapshot.hardware.accelerated_bytes,
            "unified_memory":snapshot.hardware.unified_memory
        },
        "ollama":{
            "installed":snapshot.ollama.installed,
            "running":snapshot.ollama.running,
            "version":snapshot.ollama.version,
            "base_url":snapshot.ollama.base_url,
            "install":{"command":snapshot.ollama.install_command,"url":snapshot.ollama.install_url},
            "models":snapshot.ollama.models.iter().map(|(name,size)| json!({"name":name,"size_bytes":size})).collect::<Vec<_>>(),
            "loaded":loaded
        },
        "opencode":{
            "installed":snapshot.opencode_installed,
            "install_hint":snapshot.opencode_hint,
            "configured_models":snapshot.configured
        },
        "context":snapshot.context,
        "models":snapshot.models.iter().map(|model| json!({
            "tag":model.tag,
            "label":model.label,
            "size_bytes":model.size,
            "context_k":model.context_k,
            "note":model.note,
            "fit":model.fit,
            "required_bytes":model.required,
            "disk_ok":model.disk_ok,
            "summary":model.summary,
            "recommended":model.recommended,
            "downloaded":model.downloaded,
            "loaded":model.loaded,
            "configured":model.configured
        })).collect::<Vec<_>>()
    })
}

/// Runs a confirmed or previewed operation and forwards JSON events as they arrive.
pub fn run_cli(
    project: &Path,
    op: Op,
    context: u64,
    options: OperationOptions,
    mut emit: impl FnMut(Value),
) -> Result<(), String> {
    let project = project
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver el proyecto: {error}"))?;
    let executable = super::executable_search::find_executable("ollama").ok_or_else(|| {
        "Ollama no está instalado. Instálalo con el método oficial y vuelve a intentarlo."
            .to_owned()
    })?;
    let snapshot = snapshot_with(&project, Some(&executable))?;
    let mut job = Job::start_with(&project, op, context, executable, true, options, snapshot)?;
    let mut error = None;
    loop {
        for event in job.poll() {
            if event.get("type").and_then(Value::as_str) == Some("error") {
                error = event
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            emit(event);
        }
        if let Some(code) = job.exit {
            return if code == 0 {
                Ok(())
            } else {
                Err(error
                    .unwrap_or_else(|| format!("La operación terminó con error (código {code}).")))
            };
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SAMPLE: &str = r#"{"schema":"chxchx.local-models","schema_version":1,
      "hardware":{"total_ram_bytes":22975639552,"available_ram_bytes":13642752000,"cpu_cores":8,
        "disk_free_bytes":904763838464,"models_dir":"/m","accelerator":"AMD Radeon (2 GB)",
        "accelerated_bytes":2147483648,"unified_memory":false},
      "ollama":{"installed":false,"running":false,"version":null,"base_url":"http://127.0.0.1:11434",
        "install":{"command":"curl -fsSL https://ollama.com/install.sh | sh","url":"https://ollama.com/download"},
        "models":[{"name":"qwen2.5-coder:7b","size_bytes":5046586572}],"loaded":[]},
      "opencode":{"installed":true,"install_hint":"npm i","configured_models":["x"]},"context":32768,
      "models":[{"tag":"qwen2.5-coder:7b","label":"Qwen2.5 Coder 7B","size_bytes":5046586572,
        "context_k":32,"note":"n","fit":"partial","required_bytes":7446399548,"disk_ok":true,
        "summary":"s","recommended":true,"downloaded":false,"loaded":false,"configured":false}]}"#;

    #[test]
    fn reads_the_snapshot_of_the_cli() {
        let snapshot = parse_snapshot(SAMPLE).unwrap();
        assert_eq!(snapshot.hardware.cpu_cores, 8);
        assert_eq!(snapshot.hardware.accelerated_bytes, 2147483648);
        assert!(!snapshot.ollama.installed && snapshot.ollama.install_command.is_some());
        assert_eq!(snapshot.ollama.models.len(), 1);
        assert!(snapshot.opencode_installed && snapshot.configured == ["x"]);
        let model = &snapshot.models[0];
        assert!(model.recommended && model.fit == "partial" && model.disk_ok);
        let restored = parse_snapshot(&snapshot_json(&snapshot).to_string()).unwrap();
        assert_eq!(restored, snapshot);
    }

    #[test]
    fn a_foreign_answer_is_an_error() {
        assert!(parse_snapshot(r#"{"schema":"otra"}"#).is_err());
        assert!(parse_snapshot("no json").is_err());
    }

    #[test]
    fn parses_ollama_model_table_sizes_and_loaded_models() {
        let output = "NAME ID SIZE MODIFIED\nqwen2.5-coder:7b abc 4.7 GB 2 days ago\nsmall xyz 800 MB 3 hours ago\n";
        assert_eq!(
            parse_model_table(output),
            vec![
                (
                    "qwen2.5-coder:7b".into(),
                    (4.7_f64 * 1024_f64.powi(3)) as u64
                ),
                ("small".into(), 800 * 1024 * 1024),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_uses_the_ollama_cli_and_project_opencode_config() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("chxchx-local-models-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("opencode.json"),
            r#"{"provider":{"ollama":{"models":{"qwen2.5-coder:7b-ctx32k":{}}}}}"#,
        )
        .unwrap();
        let ollama = root.join("ollama");
        fs::write(&ollama, "#!/bin/sh\ncase \"$1\" in\n  --version) echo 'ollama version 0.9.0' ;;\n  list) printf 'NAME ID SIZE MODIFIED\\nqwen2.5-coder:7b abc 4.7 GB today\\n' ;;\n  ps) printf 'NAME ID SIZE PROCESSOR\\nqwen2.5-coder:7b-ctx32k abc 4.7 GB GPU\\n' ;;\nesac\n").unwrap();
        fs::set_permissions(&ollama, fs::Permissions::from_mode(0o755)).unwrap();

        let snapshot = snapshot_with(&root, Some(&ollama)).unwrap();
        assert!(snapshot.ollama.installed && snapshot.ollama.running);
        assert_eq!(snapshot.ollama.version, "ollama version 0.9.0");
        assert_eq!(snapshot.ollama.loaded, ["qwen2.5-coder:7b-ctx32k"]);
        let candidate = snapshot
            .models
            .iter()
            .find(|m| m.tag == "qwen2.5-coder:7b")
            .unwrap();
        assert!(candidate.downloaded && candidate.loaded && candidate.configured);
        assert!(snapshot.hardware.cpu_cores > 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(size_label(5_046_586_572), "4.7 GB");
        assert_eq!(size_label(800 * 1024 * 1024), "800 MB");
    }

    #[test]
    fn estimates_gpu_partial_cpu_and_unified_memory_fits() {
        let size = 4 * 1024 * 1024 * 1024;
        let mut hardware = Hardware {
            total_ram: 16 * 1024 * 1024 * 1024,
            available_ram: 8 * 1024 * 1024 * 1024,
            accelerated_bytes: 8 * 1024 * 1024 * 1024,
            ..Hardware::default()
        };
        assert_eq!(assess_fit(size, 7 * 1024 * 1024 * 1024, &hardware), "gpu");
        assert_eq!(
            assess_fit(size, 12 * 1024 * 1024 * 1024, &hardware),
            "partial"
        );
        hardware.accelerated_bytes = 0;
        assert_eq!(assess_fit(size, 5 * 1024 * 1024 * 1024, &hardware), "cpu");
        assert_eq!(
            assess_fit(11 * size / 4, 5 * 1024 * 1024 * 1024, &hardware),
            "none"
        );
        hardware.unified_memory = true;
        hardware.accelerated_bytes = 12 * 1024 * 1024 * 1024;
        assert_eq!(assess_fit(size, 9 * 1024 * 1024 * 1024, &hardware), "none");
    }

    #[cfg(unix)]
    #[test]
    fn limited_command_drains_both_pipes_before_waiting_for_exit() {
        use std::os::unix::fs::PermissionsExt;
        let root =
            std::env::temp_dir().join(format!("chxchx-ollama-output-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let command = root.join("large-output");
        fs::write(
            &command,
            "#!/bin/sh\nhead -c 200000 /dev/zero\nhead -c 200000 /dev/zero >&2\n",
        )
        .unwrap();
        fs::set_permissions(&command, fs::Permissions::from_mode(0o755)).unwrap();
        let output = run_limited(&command, &[], Duration::from_secs(3)).unwrap();
        assert_eq!(output.len(), 200000);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    fn operation_fixture(downloaded: bool) -> Snapshot {
        let tag = "qwen2.5-coder:7b";
        Snapshot {
            hardware: Hardware {
                disk_free: 50 * 1024 * 1024 * 1024,
                ..Hardware::default()
            },
            ollama: Ollama {
                installed: true,
                running: true,
                base_url: "http://127.0.0.1:11434".into(),
                models: if downloaded {
                    vec![(tag.into(), 5 * 1024 * 1024 * 1024)]
                } else {
                    Vec::new()
                },
                ..Ollama::default()
            },
            opencode_installed: true,
            context: 32768,
            models: vec![Candidate {
                tag: tag.into(),
                label: "Qwen 7B".into(),
                size: 5 * 1024 * 1024 * 1024,
                fit: "cpu".into(),
                disk_ok: true,
                ..Candidate::default()
            }],
            ..Snapshot::default()
        }
    }

    #[cfg(unix)]
    #[test]
    fn setup_requires_download_confirmation_before_running_ollama() {
        use std::sync::atomic::AtomicBool;
        let root =
            std::env::temp_dir().join(format!("chxchx-model-confirm-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let ollama = root.join("ollama");
        fs::write(&ollama, "#!/bin/sh\ntouch called\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&ollama, fs::Permissions::from_mode(0o755)).unwrap();
        let (sender, receiver) = channel();

        let result = run_operation(OperationContext {
            project: &root,
            op: &Op::Setup {
                tag: "qwen2.5-coder:7b".into(),
                set_default: false,
            },
            context: 32768,
            ollama: &ollama,
            snapshot: operation_fixture(false),
            options: OperationOptions::default(),
            events: &sender,
            cancel: &AtomicBool::new(false),
        });

        assert!(result.unwrap_err().contains("Confirma con --yes"));
        assert!(!root.join("called").exists());
        assert!(!root.join("opencode.json").exists());
        assert!(receiver.try_iter().next().is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn dry_run_previews_setup_without_ollama_or_project_writes() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::AtomicBool;
        let root =
            std::env::temp_dir().join(format!("chxchx-model-dry-run-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let ollama = root.join("ollama");
        fs::write(&ollama, "#!/bin/sh\ntouch called\n").unwrap();
        fs::set_permissions(&ollama, fs::Permissions::from_mode(0o755)).unwrap();
        let (sender, receiver) = channel();
        let mut snapshot = operation_fixture(false);
        snapshot.ollama.running = false;

        run_operation(OperationContext {
            project: &root,
            op: &Op::Setup {
                tag: "qwen2.5-coder:7b".into(),
                set_default: true,
            },
            context: 32768,
            ollama: &ollama,
            snapshot,
            options: OperationOptions {
                dry_run: true,
                ..OperationOptions::default()
            },
            events: &sender,
            cancel: &AtomicBool::new(false),
        })
        .unwrap();

        let events = receiver.try_iter().collect::<Vec<_>>();
        assert!(events.iter().any(|event| event["type"] == "preview"));
        assert!(events.iter().any(|event| event["type"] == "done"));
        assert!(!root.join("called").exists());
        assert!(!root.join("opencode.json").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn removal_requires_confirmation_before_deleting_or_rewriting_config() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::AtomicBool;
        let root = std::env::temp_dir().join(format!(
            "chxchx-model-remove-confirm-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let ollama = root.join("ollama");
        fs::write(&ollama, "#!/bin/sh\ntouch called\n").unwrap();
        fs::set_permissions(&ollama, fs::Permissions::from_mode(0o755)).unwrap();
        let before = r#"{"provider":{"ollama":{"models":{"qwen2.5-coder:7b-ctx32k":{}}}}}"#;
        fs::write(root.join("opencode.json"), before).unwrap();
        let (sender, _) = channel();

        let result = run_operation(OperationContext {
            project: &root,
            op: &Op::Remove {
                tag: "qwen2.5-coder:7b".into(),
            },
            context: 32768,
            ollama: &ollama,
            snapshot: operation_fixture(true),
            options: OperationOptions::default(),
            events: &sender,
            cancel: &AtomicBool::new(false),
        });

        assert!(result.unwrap_err().contains("Confirma con --yes"));
        assert!(!root.join("called").exists());
        assert_eq!(
            fs::read_to_string(root.join("opencode.json")).unwrap(),
            before
        );
        let _ = fs::remove_dir_all(root);
    }
}
