//! Start, inspect and stop configured project processes with trust and PID checks.
use crate::features::project_overview::inspect_project;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const STATE_FILE: &str = "workspace-state.json";
const LOG_MAX_BYTES: u64 = 5 * 1024 * 1024;
const LOG_BACKUPS: usize = 3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Running,
    Stopped,
    Exited,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessRecord {
    pub id: String,
    pub label: String,
    pub command: Value,
    pub cwd: String,
    pub status: Status,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub started_at: Option<String>,
    pub exit_code: Option<i32>,
    pub log_path: String,
    pub shell: bool,
}

#[derive(Clone, Debug)]
pub struct ProcessSpec {
    pub id: String,
    pub label: String,
    pub command: Value,
    pub cwd: PathBuf,
    pub auto_start: bool,
    pub port: Option<u16>,
    pub shell: bool,
}

#[derive(Clone, Debug)]
pub struct Action {
    pub process: ProcessRecord,
    pub changed: bool,
    pub message: String,
}

pub fn configured(root: &Path) -> Result<Vec<ProcessSpec>, String> {
    let path = root.join(".ai/chxchx-tech.toml");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("No pude leer la configuración: {error}")),
    };
    let doc: toml::Value =
        toml::from_str(&text).map_err(|error| format!("TOML inválido: {error}"))?;
    let entries = doc
        .get("workspace")
        .and_then(|w| w.get("processes"))
        .and_then(toml::Value::as_array);
    let mut result = Vec::new();
    for entry in entries.into_iter().flatten() {
        let Some(table) = entry.as_table() else {
            continue;
        };
        let Some(id) = table.get("id").and_then(toml::Value::as_str) else {
            continue;
        };
        let Some(command) = table.get("command") else {
            continue;
        };
        let shell = table
            .get("shell")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        let valid = if shell {
            command.as_str().is_some_and(|s| !s.trim().is_empty())
        } else {
            command
                .as_array()
                .is_some_and(|a| !a.is_empty() && a.iter().all(|x| x.as_str().is_some()))
        };
        if !valid {
            return Err(format!("workspace.processes.{id}.command no es válido"));
        }
        let cwd = table
            .get("cwd")
            .and_then(toml::Value::as_str)
            .unwrap_or(".");
        if Path::new(cwd).is_absolute() || cwd.split(['/', '\\']).any(|part| part == "..") {
            return Err(format!(
                "El cwd de `{id}` debe permanecer dentro del proyecto"
            ));
        }
        result.push(ProcessSpec {
            id: id.to_owned(),
            label: table
                .get("label")
                .and_then(toml::Value::as_str)
                .unwrap_or(id)
                .to_owned(),
            command: if let Some(items) = command.as_array() {
                json!(
                    items
                        .iter()
                        .filter_map(toml::Value::as_str)
                        .collect::<Vec<_>>()
                )
            } else {
                json!(command.as_str())
            },
            cwd: PathBuf::from(cwd),
            auto_start: table
                .get("auto_start")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false),
            port: table
                .get("port")
                .and_then(toml::Value::as_integer)
                .and_then(|p| u16::try_from(p).ok()),
            shell,
        });
    }
    Ok(result)
}

pub fn list(root: &Path) -> Result<Vec<ProcessRecord>, String> {
    list_with_persistence(root, true)
}

pub fn list_readonly(root: &Path) -> Result<Vec<ProcessRecord>, String> {
    list_with_persistence(root, false)
}

fn list_with_persistence(root: &Path, persist: bool) -> Result<Vec<ProcessRecord>, String> {
    let mut state = load_state(root)?;
    let records = process_map(&mut state, root);
    for value in records.values_mut() {
        if let Ok(mut record) = serde_json::from_value::<ProcessRecord>(value.clone()) {
            refresh(&mut record);
            *value = serde_json::to_value(record).map_err(|error| error.to_string())?;
        }
    }
    let mut result = records
        .values()
        .filter_map(|value| serde_json::from_value(value.clone()).ok())
        .collect::<Vec<ProcessRecord>>();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    if persist {
        save_state(root, state)?;
    }
    Ok(result)
}

pub fn start(root: &Path, id: &str, dry_run: bool) -> Result<Action, String> {
    let overview = inspect_project(root)?;
    if !overview.workspace.trusted {
        return Err(
            "El proyecto no es confiable; concédele confianza antes de ejecutar procesos".into(),
        );
    }
    let spec = configured(root)?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("No existe el proceso configurado: {id}"))?;
    let root = fs::canonicalize(root).map_err(|error| error.to_string())?;
    let cwd = fs::canonicalize(root.join(&spec.cwd))
        .map_err(|error| format!("No existe el cwd de `{id}`: {error}"))?;
    if !cwd.starts_with(&root) || !cwd.is_dir() {
        return Err(format!(
            "El cwd de `{id}` debe permanecer dentro del proyecto"
        ));
    }
    let args = command_args(&spec)?;
    if !spec.shell && !executable_exists(&args[0], &cwd) {
        return Err(format!("No encuentro el ejecutable de `{id}`: {}", args[0]));
    }
    let mut state = load_state(&root)?;
    if let Some(value) = process_map(&mut state, &root).get(id).cloned()
        && let Ok(mut current) = serde_json::from_value::<ProcessRecord>(value)
    {
        refresh(&mut current);
        if current.status == Status::Running {
            return Ok(Action {
                message: format!(
                    "`{id}` ya está ejecutándose (PID {})",
                    current.pid.unwrap_or_default()
                ),
                process: current,
                changed: false,
            });
        }
    }
    let log = log_path(&root, id);
    let mut record = ProcessRecord {
        id: id.to_owned(),
        label: spec.label,
        command: spec.command.clone(),
        cwd: cwd.display().to_string(),
        status: Status::Stopped,
        pid: None,
        port: spec.port,
        started_at: None,
        exit_code: None,
        log_path: log.display().to_string(),
        shell: spec.shell,
    };
    if dry_run {
        record.status = Status::Running;
        record.started_at = Some(now());
        return Ok(Action {
            process: record,
            changed: true,
            message: format!("DRY RUN: iniciar `{id}`"),
        });
    }
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    rotate_log(&log)?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .map_err(|error| error.to_string())?;
    let stderr = file.try_clone().map_err(|error| error.to_string())?;
    let mut command = if spec.shell {
        #[cfg(windows)]
        {
            let mut c = Command::new("cmd");
            c.args(["/C", &args[0]]);
            c
        }
        #[cfg(not(windows))]
        {
            let mut c = Command::new("sh");
            c.args(["-c", &args[0]]);
            c
        }
    } else {
        let mut c = Command::new(&args[0]);
        c.args(&args[1..]);
        c
    };
    command
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(stderr));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            record.status = Status::Failed;
            process_map(&mut state, &root).insert(
                id.to_owned(),
                serde_json::to_value(&record).map_err(|serialize| serialize.to_string())?,
            );
            save_state(&root, state)?;
            return Err(format!("No pude iniciar `{id}`: {error}"));
        }
    };
    record.pid = Some(child.id());
    let started = std::time::Instant::now();
    let startup_grace_until = started + Duration::from_millis(100);
    let ready_until = started + Duration::from_millis(250);
    loop {
        if let Some(action) = record_early_exit(&mut child, &mut record, &mut state, &root, id)? {
            return Ok(action);
        }
        let matched = pid_matches(child.id(), &record.command, record.shell);
        let current = std::time::Instant::now();
        if (matched && current >= startup_grace_until) || current >= ready_until {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if let Some(action) = record_early_exit(&mut child, &mut record, &mut state, &root, id)? {
        return Ok(action);
    }
    record.status = Status::Running;
    record.started_at = Some(now());
    process_map(&mut state, &root).insert(
        id.to_owned(),
        serde_json::to_value(&record).map_err(|error| error.to_string())?,
    );
    save_state(&root, state)?;
    drop(child);
    Ok(Action {
        message: format!("Iniciado `{id}` (PID {})", record.pid.unwrap_or_default()),
        process: record,
        changed: true,
    })
}

fn record_early_exit(
    child: &mut std::process::Child,
    record: &mut ProcessRecord,
    state: &mut Value,
    root: &Path,
    id: &str,
) -> Result<Option<Action>, String> {
    let Some(status) = child
        .try_wait()
        .map_err(|error| format!("No pude revisar el inicio de `{id}`: {error}"))?
    else {
        return Ok(None);
    };
    record.status = if status.success() {
        Status::Exited
    } else {
        Status::Failed
    };
    record.exit_code = status.code();
    record.started_at = Some(now());
    process_map(state, root).insert(
        id.to_owned(),
        serde_json::to_value(&*record).map_err(|error| error.to_string())?,
    );
    save_state(root, state.clone())?;
    Ok(Some(Action {
        process: record.clone(),
        changed: true,
        message: format!(
            "`{id}` terminó durante el inicio (estado {:?}, código {})",
            record.status,
            record
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "desconocido".to_owned())
        ),
    }))
}

pub fn read_log(
    root: &Path,
    id: &str,
    max_bytes: usize,
    max_lines: usize,
) -> Result<String, String> {
    if !configured(root)?.iter().any(|process| process.id == id) {
        return Err(format!("No existe el proceso configurado: {id}"));
    }
    let mut state = load_state(root)?;
    let record = process_map(&mut state, root)
        .get(id)
        .and_then(|value| serde_json::from_value::<ProcessRecord>(value.clone()).ok());
    let path = record
        .map(|process| PathBuf::from(process.log_path))
        .unwrap_or_else(|| log_path(root, id));
    read_log_file(&path, max_bytes, max_lines)
}

fn read_log_file(path: &Path, max_bytes: usize, max_lines: usize) -> Result<String, String> {
    let max_lines = max_lines.min(3000);
    if max_bytes == 0 || max_lines == 0 {
        return Ok(String::new());
    }
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(error) => return Err(format!("No pude leer el log {}: {error}", path.display())),
    };
    let size = file.metadata().map_err(|error| error.to_string())?.len();
    let count = size.min(max_bytes.min(1024 * 1024) as u64);
    file.seek(std::io::SeekFrom::End(-(count as i64)))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(count as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let content = String::from_utf8_lossy(&bytes);
    let mut lines = content.lines().rev().take(max_lines).collect::<Vec<_>>();
    lines.reverse();
    Ok(lines.join("\n"))
}

pub fn stop(root: &Path, id: &str, dry_run: bool) -> Result<Action, String> {
    let mut state = load_state(root)?;
    let records = process_map(&mut state, root);
    let value = records
        .get(id)
        .cloned()
        .ok_or_else(|| format!("No hay estado para el proceso: {id}"))?;
    let mut record: ProcessRecord =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    refresh(&mut record);
    let Some(pid) = record.pid else {
        return Ok(Action {
            message: format!("`{id}` ya está detenido"),
            process: record,
            changed: false,
        });
    };
    if record.status == Status::Unknown {
        process_map(&mut state, root).insert(
            id.to_owned(),
            serde_json::to_value(&record).map_err(|error| error.to_string())?,
        );
        save_state(root, state)?;
        return Err(format!(
            "No detuve `{id}`: el PID {pid} no coincide con el comando administrado"
        ));
    }
    if record.status != Status::Running {
        return Ok(Action {
            message: format!("`{id}` ya está detenido"),
            process: record,
            changed: false,
        });
    }
    if dry_run {
        return Ok(Action {
            message: format!("DRY RUN: detener `{id}` (PID {pid})"),
            process: record,
            changed: true,
        });
    }
    terminate(pid)?;
    record.status = Status::Exited;
    record.exit_code = Some(0);
    process_map(&mut state, root).insert(
        id.to_owned(),
        serde_json::to_value(&record).map_err(|error| error.to_string())?,
    );
    save_state(root, state)?;
    Ok(Action {
        message: format!("Detenido `{id}`"),
        process: record,
        changed: true,
    })
}

fn command_args(spec: &ProcessSpec) -> Result<Vec<String>, String> {
    if let Some(s) = spec.command.as_str() {
        return Ok(vec![s.to_owned()]);
    }
    spec.command
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .filter(|a: &Vec<String>| !a.is_empty())
        .ok_or_else(|| "El comando debe ser texto o una lista de argumentos".into())
}
fn executable_exists(executable: &str, cwd: &Path) -> bool {
    let path = Path::new(executable);
    if path.components().count() > 1 {
        return if path.is_absolute() {
            path.is_file()
        } else {
            cwd.join(path).is_file()
        };
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).any(|dir| {
        let candidate = dir.join(executable);
        candidate.is_file()
            || (cfg!(windows)
                && [".exe", ".cmd", ".bat"]
                    .iter()
                    .any(|ext| dir.join(format!("{executable}{ext}")).is_file()))
    })
}
fn log_path(root: &Path, id: &str) -> PathBuf {
    let home =
        crate::features::project_profiles::application_home().unwrap_or_else(|| root.to_path_buf());
    let project = root.file_name().unwrap_or_default().to_string_lossy();
    let safe = format!("{project}-{id}")
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    home.join("logs/workspaces").join(format!("{safe}.log"))
}
fn rotate_log(path: &Path) -> Result<(), String> {
    if !path.metadata().is_ok_and(|m| m.len() >= LOG_MAX_BYTES) {
        return Ok(());
    }
    let oldest = PathBuf::from(format!("{}.{}", path.display(), LOG_BACKUPS));
    let _ = fs::remove_file(oldest);
    for i in (1..LOG_BACKUPS).rev() {
        let from = PathBuf::from(format!("{}.{i}", path.display()));
        if from.exists() {
            fs::rename(from, format!("{}.{n}", path.display(), n = i + 1))
                .map_err(|e| e.to_string())?;
        }
    }
    fs::rename(path, format!("{}.1", path.display())).map_err(|error| error.to_string())
}
fn state_path() -> PathBuf {
    crate::features::project_profiles::application_home()
        .unwrap_or_else(|| std::env::temp_dir().join("chxchx-tech"))
        .join(STATE_FILE)
}
fn load_state(root: &Path) -> Result<Value, String> {
    let path = state_path();
    let mut state: Value = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("Estado de workspace inválido: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => json!({"workspaces":{}}),
        Err(error) => return Err(error.to_string()),
    };
    if !state.get("workspaces").is_some_and(Value::is_object) {
        state["workspaces"] = json!({});
    }
    let key = fs::canonicalize(root)
        .unwrap_or_else(|_| root.to_path_buf())
        .display()
        .to_string();
    let project_path = key.clone();
    let workspace = state["workspaces"]
        .as_object_mut()
        .unwrap()
        .entry(key)
        .or_insert_with(|| json!({"project_path":project_path,"processes":{}}));
    if !workspace.is_object() {
        *workspace = json!({"project_path":project_path,"processes":{}});
    }
    if workspace.get("project_path").is_none() {
        workspace["project_path"] = json!(project_path);
    }
    Ok(state)
}
fn process_map<'a>(state: &'a mut Value, root: &Path) -> &'a mut Map<String, Value> {
    let key = fs::canonicalize(root)
        .unwrap_or_else(|_| root.to_path_buf())
        .display()
        .to_string();
    state["workspaces"][key.as_str()]
        .as_object_mut()
        .unwrap()
        .entry("processes")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .unwrap()
}
fn save_state(root: &Path, state: Value) -> Result<(), String> {
    save_state_with_workspace_status(root, state, None, None)
}

pub fn set_workspace_status(
    root: &Path,
    status: &str,
    session_name: Option<&str>,
    dry_run: bool,
) -> Result<(), String> {
    if dry_run {
        return Ok(());
    }
    let state = load_state(root)?;
    save_state_with_workspace_status(root, state, Some(status), session_name)
}

fn save_state_with_workspace_status(
    root: &Path,
    mut state: Value,
    status_override: Option<&str>,
    session_name: Option<&str>,
) -> Result<(), String> {
    let key = fs::canonicalize(root)
        .unwrap_or_else(|_| root.to_path_buf())
        .display()
        .to_string();
    if let Some(workspace) = state["workspaces"].get_mut(&key) {
        workspace["process_ids"] = Value::Array(
            workspace["processes"]
                .as_object()
                .into_iter()
                .flat_map(|m| m.iter())
                .filter(|(_, v)| v["status"] == "RUNNING")
                .map(|(k, _)| json!(k))
                .collect(),
        );
        workspace["project_path"] = json!(key);
        workspace["updated_at"] = json!(now());
        if let Some(session_name) = session_name {
            workspace["session_name"] = json!(session_name);
        }
        workspace["status"] = status_override.map_or_else(
            || {
                if workspace["process_ids"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty())
                {
                    json!("ACTIVE")
                } else {
                    json!("STOPPED")
                }
            },
            |status| json!(status),
        );
    }
    let path = state_path();
    let parent = path
        .parent()
        .ok_or("No se pudo resolver el directorio de estado")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".workspace-state-{}.tmp", std::process::id()));
    let mut file = File::create(&temp).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &state).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(&temp, path).map_err(|e| e.to_string())
}
fn refresh(record: &mut ProcessRecord) {
    let Some(pid) = record.pid else {
        record.status = Status::Stopped;
        return;
    };
    if !pid_alive(pid) {
        if record.status == Status::Running || record.status == Status::Unknown {
            record.status = Status::Exited;
        }
        return;
    }
    record.status = if pid_matches(pid, &record.command, record.shell) {
        Status::Running
    } else {
        Status::Unknown
    };
}
fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|s| s.success())
    }
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
    }
}
fn pid_matches(pid: u32, command: &Value, shell: bool) -> bool {
    let expected = if shell {
        if cfg!(windows) {
            "cmd".to_owned()
        } else {
            "sh".to_owned()
        }
    } else {
        command
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_str)
            .or_else(|| command.as_str())
            .unwrap_or("")
            .to_owned()
    };
    let expected = Path::new(expected.split_whitespace().next().unwrap_or(""))
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    #[cfg(target_os = "linux")]
    {
        if let Ok(bytes) = fs::read(format!("/proc/{pid}/cmdline")) {
            let raw = String::from_utf8_lossy(&bytes).replace('\0', " ");
            let actual = Path::new(raw.split_whitespace().next().unwrap_or(""))
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase();
            return actual == expected;
        }
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        if let Ok(output) = Command::new("/bin/ps")
            .args(["-p", &pid.to_string(), "-o", "command="])
            .output()
        {
            let actual = String::from_utf8_lossy(&output.stdout);
            return Path::new(actual.split_whitespace().next().unwrap_or(""))
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase()
                == expected;
        }
    }
    #[cfg(windows)]
    {
        if let Ok(output) = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output()
        {
            let actual = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
            return output.status.success() && actual.contains(&format!("\"{expected}.exe\""));
        }
    }
    false
}
fn terminate(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let group = format!("-{pid}");
        let _ = Command::new("/bin/kill")
            .args(["-TERM", "--", &group])
            .status();
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while pid_alive(pid) && std::time::Instant::now() < until {
            thread::sleep(Duration::from_millis(50));
        }
        if pid_alive(pid) {
            let _ = Command::new("/bin/kill")
                .args(["-KILL", "--", &group])
                .status();
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let out = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).into_owned())
        }
    }
}
fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_log_reader_limits_bytes_and_keeps_only_the_requested_tail() {
        let root = std::env::temp_dir().join(format!(
            "chxchx-process-log-{}-{}",
            std::process::id(),
            now().replace(':', "-")
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("worker.log");
        fs::write(&path, "one\ntwo\nthree\n").unwrap();

        assert_eq!(read_log_file(&path, 100, 2).unwrap(), "two\nthree");
        assert_eq!(read_log_file(&path, 5, 10).unwrap(), "hree");
        assert_eq!(
            read_log_file(&root.join("missing.log"), 100, 10).unwrap(),
            ""
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parses_configured_processes_without_shell_expansion_by_default() {
        let root =
            std::env::temp_dir().join(format!("chxchx-process-config-{}", std::process::id()));
        fs::create_dir_all(root.join(".ai")).unwrap();
        fs::write(root.join(".ai/chxchx-tech.toml"), "version=2\n[workspace]\nname='demo'\n[[workspace.processes]]\nid='web'\ncommand=['npm','run','dev']\ncwd='.'\nport=3000\n").unwrap();
        let specs = configured(&root).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].id, "web");
        assert!(!specs[0].shell);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn rejects_parent_traversal_in_process_cwd() {
        let root =
            std::env::temp_dir().join(format!("chxchx-process-escape-{}", std::process::id()));
        fs::create_dir_all(root.join(".ai")).unwrap();
        fs::write(root.join(".ai/chxchx-tech.toml"), "version=2\n[workspace]\nname='demo'\n[[workspace.processes]]\nid='bad'\ncommand=['echo']\ncwd='../outside'\n").unwrap();
        assert!(
            configured(&root)
                .unwrap_err()
                .contains("dentro del proyecto")
        );
        let _ = fs::remove_dir_all(root);
    }
}
