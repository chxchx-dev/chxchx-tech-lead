//! `tools install`: installs the two tools ChxChx's memory and code integrations rely on —
//! Basic Memory and Serena — as `uv` tools, the way the installers always did.
//!
//! Without a release bundle, an existing tool is left alone. With `--bundle`, the pinned offline
//! release replaces any old tool environment and shim. Nothing here edits agent configuration:
//! connecting them remains the responsibility of `integrate`.
use crate::integrations::executable_search::{agent_path, find_in};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Tool {
    pub name: &'static str,
    package: &'static str,
    version: &'static str,
    python_key: &'static str,
    /// Arguments of `uv` that install it.
    pub install: &'static [&'static str],
    /// The commands it leaves in the tool directory; any one of them means it is installed.
    pub commands: &'static [&'static str],
}

pub const TOOLS: [Tool; 2] = [
    Tool {
        name: "basic-memory",
        package: "basic-memory",
        version: "1.0.0",
        python_key: "python.basic-memory",
        install: &[
            "tool",
            "install",
            "-p",
            "3.12",
            "--prerelease",
            "allow",
            "basic-memory",
        ],
        commands: &["basic-memory", "bm"],
    },
    Tool {
        name: "serena",
        package: "serena-agent",
        version: "2.1.0",
        python_key: "python.serena-agent",
        install: &["tool", "install", "-p", "3.13", "serena-agent"],
        commands: &["serena"],
    },
];

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    AlreadyInstalled,
    Installed,
    /// The command that would run.
    Preview(Vec<String>),
    /// The release bundle says this tool cannot be installed on this platform, and why.
    Unavailable(String),
}

pub fn find_tool(name: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|tool| tool.name == name)
}

/// Where `uv` puts the commands of its tools: `UV_TOOL_BIN_DIR`, the `tools` folder of an MSI
/// install (marked by `.chxchx-msi` above the running program), or `~/.local/bin`.
pub fn tool_bin_directory() -> PathBuf {
    if let Some(configured) = std::env::var_os("UV_TOOL_BIN_DIR").filter(|value| !value.is_empty())
    {
        return PathBuf::from(configured);
    }
    if let Ok(executable) = std::env::current_exe() {
        for root in executable.ancestors().skip(1).take(6) {
            if root.join(".chxchx-msi").is_file()
                && let Some(parent) = root.parent()
            {
                return parent.join("tools");
            }
        }
    }
    std::env::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("bin")
}

/// `uv`: the one an MSI install carries in its private Python runtime, the one on the `PATH`,
/// then the folders the official installer uses.
pub fn find_uv() -> Option<PathBuf> {
    if let Ok(executable) = std::env::current_exe() {
        for root in executable.ancestors().skip(1).take(6) {
            let bundled = root.join(".venv").join("python").join("uv.exe");
            if root.join(".chxchx-msi").is_file() && bundled.is_file() {
                return Some(bundled);
            }
        }
    }
    if let Some(found) = find_in("uv", &agent_path()) {
        return Some(found);
    }
    let home = std::env::home_dir()?;
    let local = home.join(".local").join("bin");
    ["uv", "uv.exe"]
        .into_iter()
        .map(|name| local.join(name))
        .find(|candidate| candidate.is_file())
}

pub fn is_installed(tool: &Tool, tool_bin: &Path) -> bool {
    tool.commands.iter().any(|command| {
        ["", ".exe", ".cmd"]
            .iter()
            .any(|suffix| tool_bin.join(format!("{command}{suffix}")).is_file())
    })
}

/// Installs one tool. `uv` and `tool_bin` are given so tests do not touch the machine.
pub fn install_with(
    tool: &Tool,
    uv: Option<&Path>,
    tool_bin: &Path,
    dry_run: bool,
) -> Result<Outcome, String> {
    if is_installed(tool, tool_bin) {
        return Ok(Outcome::AlreadyInstalled);
    }
    let program = uv.map_or_else(|| "uv".to_owned(), |uv| uv.display().to_string());
    let mut shown = vec![program];
    shown.extend(tool.install.iter().map(|part| (*part).to_owned()));
    if dry_run {
        return Ok(Outcome::Preview(shown));
    }
    let uv = uv.ok_or("uv no está instalado o no está disponible en el PATH")?;
    let mut path = vec![tool_bin.to_path_buf()];
    path.extend(uv.parent().map(Path::to_path_buf));
    path.extend(std::env::split_paths(&agent_path()));
    let path = std::env::join_paths(path).map_err(|error| error.to_string())?;
    let status = Command::new(uv)
        .args(tool.install)
        .env("PATH", path)
        .env("UV_TOOL_BIN_DIR", tool_bin)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("No pude ejecutar uv: {error}"))?;
    if !status.success() {
        return Err(format!(
            "No se pudo instalar {} (uv terminó con {}). Revisa tu conexión e inténtalo de nuevo.",
            tool.name,
            status
                .code()
                .map_or("una señal".to_owned(), |code| code.to_string())
        ));
    }
    Ok(Outcome::Installed)
}

pub fn install(name: &str, dry_run: bool) -> Result<Outcome, String> {
    let tool = find_tool(name).ok_or_else(|| format!("Herramienta desconocida: {name}"))?;
    install_with(tool, find_uv().as_deref(), &tool_bin_directory(), dry_run)
}

/// Installs or upgrades a required tool using the release's offline bundle.
pub fn install_from_bundle(
    name: &str,
    bundle_source: &Path,
    dry_run: bool,
) -> Result<Outcome, String> {
    let tool = find_tool(name).ok_or_else(|| format!("Herramienta desconocida: {name}"))?;
    let destination_root = if dry_run {
        None
    } else {
        Some(required_tools_root()?)
    };
    install_from_bundle_at(
        tool,
        bundle_source,
        &tool_bin_directory(),
        destination_root.as_deref(),
        dry_run,
    )
}

fn install_from_bundle_at(
    tool: &Tool,
    bundle_source: &Path,
    tool_bin: &Path,
    destination_root: Option<&Path>,
    dry_run: bool,
) -> Result<Outcome, String> {
    let source = read_bundle(bundle_source)?;
    if let Some(reason) = unavailable_in(&source, tool) {
        return Ok(Outcome::Unavailable(reason.to_owned()));
    }
    if dry_run {
        return Ok(Outcome::Preview(bundle_command(tool, &source)?));
    }
    let destination_root = destination_root
        .ok_or_else(|| "Falta la carpeta persistente para guardar el bundle.".to_owned())?;
    let bundle = persist_bundle(&source, destination_root)?;
    fs::create_dir_all(tool_bin).map_err(|error| {
        format!(
            "No pude preparar la carpeta de comandos {}: {error}",
            tool_bin.display()
        )
    })?;
    let command = bundle_command(tool, &bundle)?;
    let (program, arguments) = command
        .split_first()
        .ok_or_else(|| "El comando de instalación quedó vacío.".to_owned())?;
    let uv = PathBuf::from(program);
    let mut path = vec![tool_bin.to_path_buf()];
    path.extend(uv.parent().map(Path::to_path_buf));
    path.extend(std::env::split_paths(&agent_path()));
    let path = std::env::join_paths(path).map_err(|error| error.to_string())?;
    let status = Command::new(&uv)
        .args(arguments)
        .env("PATH", path)
        .env("UV_TOOL_BIN_DIR", tool_bin)
        .env("UV_OFFLINE", "1")
        .env("UV_PYTHON_DOWNLOADS", "never")
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("No pude ejecutar uv para {}: {error}", tool.name))?;
    if !status.success() {
        return Err(format!(
            "No se pudo instalar o actualizar {} desde el bundle offline (uv terminó con {}). Revisa el error de uv e inténtalo de nuevo.",
            tool.name,
            status
                .code()
                .map_or("una señal".to_owned(), |code| code.to_string())
        ));
    }
    if !is_installed(tool, tool_bin) {
        return Err(format!(
            "uv terminó, pero no dejó los comandos requeridos de {} en {}.",
            tool.name,
            tool_bin.display()
        ));
    }
    Ok(Outcome::Installed)
}

#[derive(Clone, Debug)]
struct Bundle {
    root: PathBuf,
    id: String,
    values: BTreeMap<String, String>,
}

fn read_bundle(root: &Path) -> Result<Bundle, String> {
    let manifest_path = root.join("bundle-manifest.txt");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("No pude leer {}: {error}", manifest_path.display()))?;
    let mut values = BTreeMap::new();
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            return Err("El manifiesto del bundle tiene una línea inválida.".into());
        };
        if values.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("Clave repetida en el bundle: {key}"));
        }
    }
    if values.get("schema_version").map(String::as_str) != Some("1") {
        return Err("Versión de manifiesto de bundle no compatible.".into());
    }
    let id = values
        .get("bundle_id")
        .filter(|id| {
            !id.is_empty()
                && id.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
                })
        })
        .cloned()
        .ok_or_else(|| "El bundle no contiene un identificador válido.".to_owned())?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("No pude resolver la carpeta del bundle: {error}"))?;
    let bundle = Bundle { root, id, values };
    bundle_file(&bundle, "uv", false)?;
    for tool in &TOOLS {
        // A tool the release cannot offer on this platform says so in the manifest and has no files.
        if unavailable_in(&bundle, tool).is_some() {
            continue;
        }
        bundle_file(&bundle, tool.python_key, false)?;
        bundle_file(&bundle, &format!("lock.{}", tool.package), false)?;
        let wheelhouse = bundle_file(&bundle, &format!("wheelhouse.{}", tool.package), true)?;
        if !fs::read_dir(&wheelhouse)
            .map_err(|error| format!("No pude revisar el wheelhouse de {}: {error}", tool.name))?
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "whl")
            })
        {
            return Err(format!("El bundle no incluye wheels para {}.", tool.name));
        }
        let version_key = format!("version.{}", tool.package);
        if bundle.values.get(&version_key).map(String::as_str) != Some(tool.version) {
            return Err(format!(
                "El bundle contiene una versión inesperada de {}.",
                tool.name
            ));
        }
    }
    Ok(bundle)
}

/// Why the bundle does not carry `tool` (`unavailable.<package>` in its manifest), if it does not.
fn unavailable_in<'a>(bundle: &'a Bundle, tool: &Tool) -> Option<&'a str> {
    bundle
        .values
        .get(&format!("unavailable.{}", tool.package))
        .map(String::as_str)
        .filter(|reason| !reason.is_empty())
}

fn bundle_file(bundle: &Bundle, key: &str, directory: bool) -> Result<PathBuf, String> {
    let value = bundle
        .values
        .get(key)
        .ok_or_else(|| format!("Falta {key} en el manifiesto del bundle."))?;
    let relative = Path::new(value);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || relative
            .components()
            .all(|component| component == Component::CurDir)
    {
        return Err(format!("Ruta no válida en el bundle: {key}={value}"));
    }
    let mut path = bundle.root.clone();
    for component in relative.components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => continue,
            _ => return Err(format!("Ruta no válida en el bundle: {key}={value}")),
        }
        if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(format!(
                "El recurso {key} del bundle no puede usar enlaces."
            ));
        }
    }
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("No existe {key} en el bundle ({value}): {error}"))?;
    if metadata.file_type().is_symlink()
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err(format!(
            "El recurso {key} del bundle no tiene el tipo esperado."
        ));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("No pude resolver {key} del bundle: {error}"))?;
    if !canonical.starts_with(&bundle.root) {
        return Err(format!("La ruta {key} sale de la carpeta del bundle."));
    }
    Ok(canonical)
}

fn bundle_command(tool: &Tool, bundle: &Bundle) -> Result<Vec<String>, String> {
    let uv = bundle_file(bundle, "uv", false)?;
    let python = bundle_file(bundle, tool.python_key, false)?;
    let lock = bundle_file(bundle, &format!("lock.{}", tool.package), false)?;
    let wheelhouse = bundle_file(bundle, &format!("wheelhouse.{}", tool.package), true)?;
    let mut command = vec![uv.display().to_string(), "tool".into(), "install".into()];
    command.extend(
        [
            "--force",
            "--offline",
            "--no-index",
            "--no-python-downloads",
            "--find-links",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    command.push(wheelhouse.display().to_string());
    command.extend(["--with-requirements".into(), lock.display().to_string()]);
    command.extend(["--python".into(), python.display().to_string()]);
    command.push(format!("{}=={}", tool.package, tool.version));
    Ok(command)
}

fn persist_bundle(bundle: &Bundle, destination_root: &Path) -> Result<Bundle, String> {
    fs::create_dir_all(destination_root)
        .map_err(|error| format!("No pude preparar el almacenamiento de herramientas: {error}"))?;
    let destination = destination_root.join(&bundle.id);
    if destination.exists() {
        let installed = read_bundle(&destination).map_err(|error| {
            format!(
                "El bundle previo {} está incompleto: {error}",
                destination.display()
            )
        })?;
        if installed.id != bundle.id {
            return Err("El bundle previo no coincide con el release.".into());
        }
        return Ok(installed);
    }
    let staging = destination_root.join(format!(".{}.stage-{}", bundle.id, std::process::id()));
    if staging.exists() {
        return Err(format!(
            "Ya existe una copia incompleta del bundle: {}",
            staging.display()
        ));
    }
    if let Err(error) = copy_bundle_tree(&bundle.root, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let copied = match read_bundle(&staging) {
        Ok(copied) => copied,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    if copied.id != bundle.id {
        let _ = fs::remove_dir_all(&staging);
        return Err("La copia del bundle cambió su identificador.".into());
    }
    if let Err(error) = fs::rename(&staging, &destination) {
        if destination.exists() {
            let _ = fs::remove_dir_all(&staging);
            let installed = read_bundle(&destination).map_err(|destination_error| {
                format!(
                    "No pude guardar el bundle en {} ({error}); la copia concurrente está incompleta: {destination_error}",
                    destination.display()
                )
            })?;
            if installed.id == bundle.id {
                return Ok(installed);
            }
        }
        let _ = fs::remove_dir_all(&staging);
        return Err(format!(
            "No pude guardar el bundle en {}: {error}",
            destination.display()
        ));
    }
    read_bundle(&destination)
}

fn copy_bundle_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let root = source
        .canonicalize()
        .map_err(|error| format!("No pude resolver el bundle: {error}"))?;
    copy_bundle_tree_from(&root, &root, destination, &mut HashSet::new())
}

fn copy_bundle_tree_from(
    root: &Path,
    source: &Path,
    destination: &Path,
    active_directories: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    let resolved_source = source
        .canonicalize()
        .map_err(|error| format!("No pude resolver {}: {error}", source.display()))?;
    if !resolved_source.starts_with(root) {
        return Err(format!("La ruta {} sale del bundle.", source.display()));
    }
    if !active_directories.insert(resolved_source.clone()) {
        return Err(format!(
            "El bundle contiene un enlace circular: {}",
            source.display()
        ));
    }
    fs::create_dir(destination)
        .map_err(|error| format!("No pude crear la copia temporal del bundle: {error}"))?;
    let entries =
        fs::read_dir(source).map_err(|error| format!("No pude recorrer el bundle: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("No pude leer el bundle: {error}"))?;
        let source_path = entry.path();
        let metadata = fs::symlink_metadata(&source_path)
            .map_err(|error| format!("No pude comprobar {}: {error}", source_path.display()))?;
        let destination_path = destination.join(entry.file_name());
        let resolved_path = if metadata.file_type().is_symlink() {
            source_path.canonicalize().map_err(|error| {
                format!(
                    "No pude resolver el enlace {}: {error}",
                    source_path.display()
                )
            })?
        } else {
            source_path.clone()
        };
        if metadata.is_dir() || (metadata.file_type().is_symlink() && resolved_path.is_dir()) {
            copy_bundle_tree_from(root, &resolved_path, &destination_path, active_directories)?;
        } else if metadata.is_file()
            || (metadata.file_type().is_symlink() && resolved_path.is_file())
        {
            if !resolved_path.starts_with(root) {
                return Err(format!(
                    "El enlace {} sale del bundle.",
                    source_path.display()
                ));
            }
            fs::copy(&resolved_path, &destination_path)
                .map_err(|error| format!("No pude copiar {}: {error}", source_path.display()))?;
        } else {
            return Err(format!(
                "Tipo de archivo no admitido en el bundle: {}",
                source_path.display()
            ));
        }
    }
    active_directories.remove(&resolved_source);
    Ok(())
}

fn required_tools_root() -> Result<PathBuf, String> {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(std::env::home_dir)
        .ok_or_else(|| "No pude determinar la carpeta de datos del usuario.".to_owned())?;
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|path| Path::new(path).is_absolute())
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(".local/share")))
        .ok_or_else(|| "No pude determinar la carpeta de datos del usuario.".to_owned())?;
    Ok(base.join("ChxChxStudio").join("required-tools"))
}

#[cfg(test)]
#[path = "tools_install_tests.rs"]
mod tests;
