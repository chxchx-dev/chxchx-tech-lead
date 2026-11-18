//! Read-only workspace and process resource measurements.
use crate::features::workspace_processes::{ProcessRecord, Status};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
#[cfg(any(target_os = "macos", windows))]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Ok,
    Warning,
    Critical,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub warn_memory_percent: u8,
    pub critical_memory_percent: u8,
    pub warn_swap_percent: u8,
    pub max_agents: u8,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            warn_memory_percent: 70,
            critical_memory_percent: 85,
            warn_swap_percent: 40,
            max_agents: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemResources {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub cpu_percent: Option<f64>,
}

impl SystemResources {
    pub fn memory_percent(self) -> f64 {
        percent(self.used_bytes, self.total_bytes)
    }

    pub fn swap_percent(self) -> f64 {
        percent(self.swap_used_bytes, self.swap_total_bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessResources {
    pub process_id: String,
    pub label: String,
    pub pid: Option<u32>,
    pub rss_bytes: u64,
    pub cpu_percent: Option<f64>,
    pub status: Status,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessSummary {
    pub running_count: usize,
    pub rss_bytes: u64,
    pub cpu_percent: Option<f64>,
    pub measured_cpu_count: usize,
}

impl ProcessSummary {
    pub fn memory_percent(self, total_memory_bytes: u64) -> Option<f64> {
        (total_memory_bytes > 0).then(|| self.rss_bytes as f64 / total_memory_bytes as f64 * 100.0)
    }
}

pub fn config_from_project(root: &Path) -> Result<ResourceConfig, String> {
    let path = root.join(".ai/chxchx-tech.toml");
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ResourceConfig::default());
        }
        Err(error) => return Err(format!("No pude leer la configuración: {error}")),
    };
    let document: toml::Value =
        toml::from_str(&text).map_err(|error| format!("TOML inválido: {error}"))?;
    let Some(resources) = document
        .get("workspace")
        .and_then(|workspace| workspace.get("resources"))
    else {
        return Ok(ResourceConfig::default());
    };
    let table = resources
        .as_table()
        .ok_or_else(|| "workspace.resources debe ser una tabla".to_owned())?;
    let mut config = ResourceConfig::default();
    for (key, target) in [
        ("warn_memory_percent", &mut config.warn_memory_percent),
        (
            "critical_memory_percent",
            &mut config.critical_memory_percent,
        ),
        ("warn_swap_percent", &mut config.warn_swap_percent),
    ] {
        if let Some(value) = table.get(key) {
            let value = value
                .as_integer()
                .and_then(|value| u8::try_from(value).ok())
                .filter(|value| *value <= 100)
                .ok_or_else(|| format!("workspace.resources.{key} debe estar entre 0 y 100"))?;
            *target = value;
        }
    }
    if let Some(value) = table.get("max_agents") {
        config.max_agents = value
            .as_integer()
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| (1..=16).contains(value))
            .ok_or_else(|| "workspace.resources.max_agents debe estar entre 1 y 16".to_owned())?;
    }
    if config.critical_memory_percent < config.warn_memory_percent {
        return Err(
            "critical_memory_percent no puede ser menor que warn_memory_percent".to_owned(),
        );
    }
    Ok(config)
}

pub fn severity(config: ResourceConfig, resources: SystemResources) -> Severity {
    if resources.total_bytes == 0 {
        return Severity::Unknown;
    }
    if resources.memory_percent() >= f64::from(config.critical_memory_percent) {
        Severity::Critical
    } else if resources.memory_percent() >= f64::from(config.warn_memory_percent)
        || resources.swap_percent() >= f64::from(config.warn_swap_percent)
    {
        Severity::Warning
    } else {
        Severity::Ok
    }
}

pub fn agent_start_warnings(
    config: ResourceConfig,
    active_agents: usize,
    planned_agents: usize,
    resources: SystemResources,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let current = severity(config, resources);
    if planned_agents > 0 && current == Severity::Critical {
        warnings.push(format!(
            "RAM al {:.0}% (crítico: {}%).",
            resources.memory_percent(),
            config.critical_memory_percent
        ));
    } else if planned_agents > 0 && current == Severity::Warning {
        let mut causes = Vec::new();
        if resources.memory_percent() >= f64::from(config.warn_memory_percent) {
            causes.push(format!("RAM al {:.0}%", resources.memory_percent()));
        }
        if resources.swap_percent() >= f64::from(config.warn_swap_percent) {
            causes.push(format!("swap al {:.0}%", resources.swap_percent()));
        }
        warnings.push(format!(
            "{}; umbral de aviso {}% RAM / {}% swap.",
            causes.join(", "),
            config.warn_memory_percent,
            config.warn_swap_percent
        ));
    }
    let projected = active_agents.saturating_add(planned_agents);
    if planned_agents > 0 && projected > usize::from(config.max_agents) {
        warnings.push(format!(
            "Se proyectan {projected} agentes activos; el límite configurado es {}.",
            config.max_agents
        ));
    }
    warnings
}

pub fn process_resources(processes: &[ProcessRecord]) -> Vec<ProcessResources> {
    process_resources_with(processes, read_process_resources)
}

fn process_resources_with(
    processes: &[ProcessRecord],
    mut reader: impl FnMut(u32) -> Option<(u64, Option<f64>)>,
) -> Vec<ProcessResources> {
    processes
        .iter()
        .map(|process| {
            let measurement = if process.status == Status::Running {
                process.pid.and_then(&mut reader)
            } else {
                None
            };
            ProcessResources {
                process_id: process.id.clone(),
                label: process.label.clone(),
                pid: process.pid,
                rss_bytes: measurement.map_or(0, |(rss, _)| rss),
                cpu_percent: measurement.and_then(|(_, cpu)| cpu),
                status: process.status.clone(),
            }
        })
        .collect()
}

pub fn summarize_process_resources(processes: &[ProcessResources]) -> ProcessSummary {
    let running = processes
        .iter()
        .filter(|process| process.status == Status::Running && process.pid.is_some())
        .collect::<Vec<_>>();
    let measured_cpu = running
        .iter()
        .filter_map(|process| process.cpu_percent)
        .collect::<Vec<_>>();
    ProcessSummary {
        running_count: running.len(),
        rss_bytes: running.iter().map(|process| process.rss_bytes).sum(),
        cpu_percent: (!measured_cpu.is_empty()).then(|| measured_cpu.iter().sum()),
        measured_cpu_count: measured_cpu.len(),
    }
}

pub fn read_system_resources() -> SystemResources {
    #[cfg(target_os = "linux")]
    {
        return read_linux_system_resources();
    }
    #[cfg(target_os = "macos")]
    {
        return read_macos_system_resources();
    }
    #[cfg(windows)]
    {
        return read_windows_system_resources();
    }
    #[allow(unreachable_code)]
    SystemResources::default()
}

pub fn resources_overview_payload(current_path: &Path) -> Result<Value, String> {
    let catalog = crate::features::project_catalog::inspect_registered_projects()?;
    let current = fs::canonicalize(current_path).map_err(|error| {
        format!(
            "No se pudo resolver el proyecto {}: {error}",
            current_path.display()
        )
    })?;
    if !current.is_dir() {
        return Err(format!(
            "La ruta del proyecto no es un directorio: {}",
            current.display()
        ));
    }
    let mut paths = BTreeMap::<String, (String, std::path::PathBuf)>::new();
    for project in catalog
        .projects
        .into_iter()
        .filter(|project| project.available)
    {
        let Some(raw_path) = project.path else {
            continue;
        };
        let Ok(path) = fs::canonicalize(&raw_path) else {
            continue;
        };
        paths.insert(path.display().to_string(), (project.alias, path));
    }
    paths
        .entry(current.display().to_string())
        .or_insert_with(|| {
            let name = current
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| current.display().to_string());
            (name, current.clone())
        });

    let system = read_system_resources();
    let mut projects = Vec::with_capacity(paths.len());
    for (key, (alias, root)) in paths {
        let config = config_from_project(&root).unwrap_or_default();
        let mut process_map = crate::features::workspace_processes::list_readonly(&root)
            .unwrap_or_default()
            .into_iter()
            .map(|process| (process.id.clone(), process))
            .collect::<BTreeMap<_, _>>();
        for spec in crate::features::workspace_processes::configured(&root).unwrap_or_default() {
            let cwd = root
                .join(&spec.cwd)
                .canonicalize()
                .unwrap_or_else(|_| root.join(&spec.cwd));
            process_map.entry(spec.id.clone()).or_insert(ProcessRecord {
                id: spec.id,
                label: spec.label,
                command: spec.command,
                cwd: cwd.display().to_string(),
                status: Status::Stopped,
                pid: None,
                port: spec.port,
                started_at: None,
                exit_code: None,
                log_path: String::new(),
                shell: spec.shell,
            });
        }
        let managed = process_map.into_values().collect::<Vec<_>>();
        let measured = process_resources(&managed);
        let summary = summarize_process_resources(&measured);
        let trusted = crate::features::project_overview::inspect_project(&root)
            .is_ok_and(|overview| overview.workspace.trusted);
        projects.push(json!({
            "alias": alias,
            "name": root.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(),
            "path": key,
            "status": read_workspace_status(&root),
            "trusted": trusted,
            "running_count": summary.running_count,
            "rss_bytes": summary.rss_bytes,
            "cpu_percent": summary.cpu_percent,
            "warning_memory_percent": config.warn_memory_percent,
            "critical_memory_percent": config.critical_memory_percent,
            "max_agents": config.max_agents,
        }));
    }
    Ok(json!({
        "schema": "chxchx.resources-overview",
        "schema_version": 1,
        "system": {
            "memory_total_bytes": system.total_bytes,
            "memory_used_bytes": system.used_bytes,
            "memory_available_bytes": system.available_bytes,
            "memory_percent": system.memory_percent(),
            "swap_total_bytes": system.swap_total_bytes,
            "swap_used_bytes": system.swap_used_bytes,
            "swap_percent": system.swap_percent(),
            "cpu_percent": system.cpu_percent,
        },
        "projects": projects,
    }))
}

fn read_workspace_status(root: &Path) -> String {
    let Some(home) = crate::features::project_profiles::application_home() else {
        return "STOPPED".to_owned();
    };
    let Ok(contents) = fs::read(home.join("workspace-state.json")) else {
        return "STOPPED".to_owned();
    };
    let Ok(document) = serde_json::from_slice::<Value>(&contents) else {
        return "STOPPED".to_owned();
    };
    document
        .get("workspaces")
        .and_then(|workspaces| workspaces.get(root.display().to_string()))
        .and_then(|workspace| workspace.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("STOPPED")
        .to_owned()
}

pub fn format_bytes(value: i64) -> String {
    if value < 0 {
        return "N/D".to_owned();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut amount = value as f64;
    for unit in units {
        if amount < 1024.0 || unit == "TB" {
            return format!("{amount:.1} {unit}");
        }
        amount /= 1024.0;
    }
    "N/D".to_owned()
}

fn percent(value: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64 * 100.0
    }
}

#[cfg(target_os = "linux")]
fn read_linux_system_resources() -> SystemResources {
    let Ok(contents) = std::fs::read_to_string("/proc/meminfo") else {
        return SystemResources::default();
    };
    let mut values = std::collections::BTreeMap::new();
    for line in contents.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let Some(number) = value
            .split_whitespace()
            .next()
            .and_then(|n| n.parse::<u64>().ok())
        else {
            continue;
        };
        values.insert(key, number.saturating_mul(1024));
    }
    let total = values.get("MemTotal").copied().unwrap_or_default();
    let available = values
        .get("MemAvailable")
        .copied()
        .or_else(|| values.get("MemFree").copied())
        .unwrap_or_default();
    let swap_total = values.get("SwapTotal").copied().unwrap_or_default();
    let swap_free = values.get("SwapFree").copied().unwrap_or_default();
    SystemResources {
        total_bytes: total,
        used_bytes: total.saturating_sub(available),
        available_bytes: available,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_total.saturating_sub(swap_free),
        cpu_percent: read_linux_cpu_percent(),
    }
}

#[cfg(target_os = "linux")]
fn read_linux_cpu_percent() -> Option<f64> {
    static PREVIOUS: OnceLock<Mutex<Option<(u64, u64)>>> = OnceLock::new();
    let contents = std::fs::read_to_string("/proc/stat").ok()?;
    let line = contents.lines().next()?.strip_prefix("cpu ")?;
    let counters = line
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if counters.len() < 4 {
        return None;
    }
    let total = counters.iter().copied().fold(0_u64, u64::saturating_add);
    let idle = counters[3].saturating_add(counters.get(4).copied().unwrap_or_default());
    let lock = PREVIOUS.get_or_init(|| Mutex::new(None));
    let mut previous = lock.lock().ok()?;
    let before = *previous;
    *previous = Some((total, idle));
    let (old_total, old_idle) = before?;
    let total_delta = total.saturating_sub(old_total);
    let idle_delta = idle.saturating_sub(old_idle);
    (total_delta > 0).then(|| {
        (total_delta.saturating_sub(idle_delta) as f64 / total_delta as f64 * 100.0)
            .clamp(0.0, 100.0)
    })
}

#[cfg(target_os = "linux")]
fn read_process_resources(pid: u32) -> Option<(u64, Option<f64>)> {
    let statm = std::fs::read_to_string(format!("/proc/{pid}/statm")).ok()?;
    let resident_pages = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    let page_size = libc_page_size()?;
    Some((resident_pages.saturating_mul(page_size), None))
}

#[cfg(target_os = "linux")]
fn libc_page_size() -> Option<u64> {
    unsafe extern "C" {
        fn sysconf(name: i32) -> isize;
    }
    const SC_PAGESIZE: i32 = 30;
    // SAFETY: sysconf reads one process-wide constant and has no pointer arguments.
    let value = unsafe { sysconf(SC_PAGESIZE) };
    (value > 0).then_some(value as u64)
}

#[cfg(target_os = "macos")]
fn read_macos_system_resources() -> SystemResources {
    let total = command_text("sysctl", &["-n", "hw.memsize"])
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or_default();
    let page_data = Command::new("vm_stat").output().ok();
    let (free, speculative, used) = page_data
        .filter(|output| output.status.success())
        .map(|output| parse_vm_stat(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default();
    let page_size = page_data_page_size().unwrap_or(4096);
    let swap = command_text("sysctl", &["vm.swapusage"])
        .map(|text| parse_swap_usage(&text))
        .unwrap_or_default();
    let available = free.saturating_add(speculative).saturating_mul(page_size);
    SystemResources {
        total_bytes: total,
        used_bytes: used.saturating_mul(page_size).min(total),
        available_bytes: available.min(total),
        swap_total_bytes: swap.0,
        swap_used_bytes: swap.1,
        cpu_percent: None,
    }
}

#[cfg(target_os = "macos")]
fn page_data_page_size() -> Option<u64> {
    let output = Command::new("vm_stat").output().ok()?;
    let first = String::from_utf8_lossy(&output.stdout);
    let marker = "page size of ";
    let rest = first.split_once(marker)?.1;
    rest.split_whitespace().next()?.parse().ok()
}

#[cfg(target_os = "macos")]
fn parse_vm_stat(text: &str) -> (u64, u64, u64) {
    let mut values = std::collections::BTreeMap::new();
    for line in text.lines().skip(1) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let Some(value) = value
            .trim()
            .trim_end_matches('.')
            .replace('.', "")
            .parse::<u64>()
            .ok()
        else {
            continue;
        };
        values.insert(key.trim(), value);
    }
    let free = values.get("Pages free").copied().unwrap_or_default();
    let speculative = values.get("Pages speculative").copied().unwrap_or_default();
    let used = [
        "Pages active",
        "Pages inactive",
        "Pages wired down",
        "Pages occupied by compressor",
    ]
    .into_iter()
    .filter_map(|key| values.get(key))
    .copied()
    .sum();
    (free, speculative, used)
}

#[cfg(target_os = "macos")]
fn parse_swap_usage(text: &str) -> (u64, u64) {
    let total = text
        .split_once("total =")
        .and_then(|(_, tail)| tail.split_whitespace().next())
        .and_then(parse_macos_size)
        .unwrap_or_default();
    let used = text
        .split_once("used =")
        .and_then(|(_, tail)| tail.split_whitespace().next())
        .and_then(parse_macos_size)
        .unwrap_or_default();
    (total, used)
}

#[cfg(target_os = "macos")]
fn parse_macos_size(raw: &str) -> Option<u64> {
    let unit = raw.chars().last()?.to_ascii_uppercase();
    let amount = raw[..raw.len() - 1].parse::<f64>().ok()?;
    let multiplier = match unit {
        'K' => 1024.0,
        'M' => 1024.0_f64.powi(2),
        'G' => 1024.0_f64.powi(3),
        _ => return None,
    };
    Some((amount * multiplier) as u64)
}

#[cfg(target_os = "macos")]
fn read_process_resources(pid: u32) -> Option<(u64, Option<f64>)> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-o", "%cpu=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let metrics = String::from_utf8_lossy(&output.stdout);
    let mut parts = metrics.split_whitespace();
    let rss = parts.next()?.parse::<u64>().ok()?.saturating_mul(1024);
    let cpu = parts.next().and_then(|value| value.parse::<f64>().ok());
    Some((rss, cpu))
}

#[cfg(windows)]
fn read_windows_system_resources() -> SystemResources {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    #[repr(C)]
    struct PerformanceInformation {
        size: u32,
        commit_total: usize,
        commit_limit: usize,
        commit_peak: usize,
        physical_total: usize,
        physical_available: usize,
        system_cache: usize,
        kernel_total: usize,
        kernel_paged: usize,
        kernel_nonpaged: usize,
        page_size: usize,
        handle_count: u32,
        process_count: u32,
        thread_count: u32,
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetPerformanceInfo(info: *mut PerformanceInformation, size: u32) -> i32;
    }
    let mut status = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };
    // SAFETY: status has the documented size and lives through the API call.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return SystemResources::default();
    }
    let mut performance = PerformanceInformation {
        size: std::mem::size_of::<PerformanceInformation>() as u32,
        commit_total: 0,
        commit_limit: 0,
        commit_peak: 0,
        physical_total: 0,
        physical_available: 0,
        system_cache: 0,
        kernel_total: 0,
        kernel_paged: 0,
        kernel_nonpaged: 0,
        page_size: 0,
        handle_count: 0,
        process_count: 0,
        thread_count: 0,
    };
    // SAFETY: performance has the documented size and lives through the API call.
    let swap_bytes = if unsafe {
        GetPerformanceInfo(
            &mut performance,
            std::mem::size_of::<PerformanceInformation>() as u32,
        )
    } != 0
    {
        let page_size = performance.page_size as u64;
        let physical = performance.physical_total as u64;
        let total = (performance.commit_limit as u64)
            .saturating_sub(physical)
            .saturating_mul(page_size);
        let used = (performance.commit_total as u64)
            .saturating_sub(physical)
            .saturating_mul(page_size)
            .min(total);
        (total, used)
    } else {
        (0, 0)
    };
    SystemResources {
        total_bytes: status.total_phys,
        used_bytes: status.total_phys.saturating_sub(status.avail_phys),
        available_bytes: status.avail_phys,
        swap_total_bytes: swap_bytes.0,
        swap_used_bytes: swap_bytes.1,
        cpu_percent: None,
    }
}

#[cfg(windows)]
fn read_process_resources(pid: u32) -> Option<(u64, Option<f64>)> {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, process_id: u32) -> *mut std::ffi::c_void;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }
    const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    const PROCESS_VM_READ: u32 = 0x0010;
    // SAFETY: the API receives only scalar identifiers and a valid out pointer.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut counters = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    // SAFETY: counters is correctly sized, and handle was returned by OpenProcess.
    let success = unsafe {
        GetProcessMemoryInfo(
            handle,
            &mut counters,
            std::mem::size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    // SAFETY: close the valid handle exactly once after querying it.
    unsafe { CloseHandle(handle) };
    (success != 0).then_some((counters.working_set_size as u64, None))
}

#[cfg(target_os = "macos")]
fn command_text(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(used: u64, swap_used: u64) -> SystemResources {
        SystemResources {
            total_bytes: 100,
            used_bytes: used,
            available_bytes: 100 - used,
            swap_total_bytes: 100,
            swap_used_bytes: swap_used,
            cpu_percent: Some(25.0),
        }
    }

    #[test]
    fn classifies_memory_and_swap_thresholds_in_order() {
        let config = ResourceConfig {
            warn_memory_percent: 75,
            critical_memory_percent: 90,
            warn_swap_percent: 40,
            ..ResourceConfig::default()
        };
        assert_eq!(severity(config, snapshot(80, 10)), Severity::Warning);
        assert_eq!(severity(config, snapshot(70, 95)), Severity::Warning);
        assert_eq!(severity(config, snapshot(95, 0)), Severity::Critical);
        assert_eq!(
            severity(config, SystemResources::default()),
            Severity::Unknown
        );
    }

    #[test]
    fn agent_start_warnings_report_pressure_and_agent_budget() {
        let config = ResourceConfig::default();
        let warnings = agent_start_warnings(config, 1, 2, snapshot(88, 50));
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("RAM al 88%"));
        assert!(warnings[1].contains("3 agentes activos"));
        assert!(agent_start_warnings(config, 2, 0, snapshot(99, 99)).is_empty());
    }

    #[test]
    fn process_measurements_are_limited_to_running_records() {
        let running = ProcessRecord {
            id: "api".into(),
            label: "API".into(),
            command: serde_json::json!(["api"]),
            cwd: "/tmp".into(),
            status: Status::Running,
            pid: Some(123),
            port: None,
            started_at: None,
            exit_code: None,
            log_path: "/tmp/api.log".into(),
            shell: false,
        };
        let mut stopped = running.clone();
        stopped.id = "worker".into();
        stopped.label = "Worker".into();
        stopped.status = Status::Stopped;
        stopped.pid = Some(456);
        let seen = std::cell::RefCell::new(Vec::new());
        let measured = process_resources_with(&[running, stopped], |pid| {
            seen.borrow_mut().push(pid);
            Some((12_345, Some(4.5)))
        });
        assert_eq!(*seen.borrow(), [123]);
        assert_eq!(measured[0].rss_bytes, 12_345);
        assert_eq!(measured[1].rss_bytes, 0);
        assert_eq!(measured[1].cpu_percent, None);
    }

    #[test]
    fn summaries_and_byte_format_match_the_public_contract() {
        let records = [
            ProcessResources {
                process_id: "api".into(),
                label: "API".into(),
                pid: Some(10),
                rss_bytes: 20_000,
                cpu_percent: Some(12.5),
                status: Status::Running,
            },
            ProcessResources {
                process_id: "web".into(),
                label: "Web".into(),
                pid: Some(11),
                rss_bytes: 30_000,
                cpu_percent: Some(5.0),
                status: Status::Running,
            },
            ProcessResources {
                process_id: "stopped".into(),
                label: "Stopped".into(),
                pid: None,
                rss_bytes: 50_000,
                cpu_percent: None,
                status: Status::Stopped,
            },
        ];
        let summary = summarize_process_resources(&records);
        assert_eq!(summary.running_count, 2);
        assert_eq!(summary.rss_bytes, 50_000);
        assert_eq!(summary.cpu_percent, Some(17.5));
        assert_eq!(summary.measured_cpu_count, 2);
        assert_eq!(summary.memory_percent(1_000_000), Some(5.0));
        assert_eq!(format_bytes(0), "0.0 B");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(-1), "N/D");
    }

    #[test]
    fn resource_config_is_parsed_and_validated_from_project_toml() {
        let root = std::env::temp_dir().join(format!("chxchx-resources-{}", std::process::id()));
        std::fs::create_dir_all(root.join(".ai")).unwrap();
        std::fs::write(
            root.join(".ai/chxchx-tech.toml"),
            "version=2\n[workspace]\nname='demo'\n[workspace.resources]\nwarn_memory_percent=72\ncritical_memory_percent=88\nmax_agents=4\n",
        )
        .unwrap();
        let config = config_from_project(&root).unwrap();
        assert_eq!(config.warn_memory_percent, 72);
        assert_eq!(config.critical_memory_percent, 88);
        assert_eq!(config.max_agents, 4);
        std::fs::write(
            root.join(".ai/chxchx-tech.toml"),
            "version=2\n[workspace]\nname='demo'\n[workspace.resources]\nwarn_memory_percent=90\ncritical_memory_percent=70\n",
        )
        .unwrap();
        assert!(config_from_project(&root).is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
