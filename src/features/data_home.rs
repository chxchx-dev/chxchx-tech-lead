//! Where this installation keeps its data (settings, identity, devices, jobs, backups…), chosen by
//! the person. The choice is a tiny pointer file that always lives in the original folder, because
//! the chosen folder cannot say where it is itself. `CHXCHX_TECH_HOME` still wins over it.
//!
//! Changing it is a copy, never a move: the new folder is filled and checked first, the pointer is
//! written last, and the original folder stays untouched as the way back. Going back is removing
//! the pointer (the old one stays as `.bak`).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

const POINTER: &str = "data-location.json";
const SCHEMA: &str = "chxchx.data-location";

#[derive(Serialize, Deserialize)]
struct Pointer {
    schema: String,
    path: String,
}

/// What a change would do, found without touching anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub from: PathBuf,
    pub to: PathBuf,
    pub files: u64,
    pub bytes: u64,
}

/// The folder used when nothing was chosen.
pub fn default_home() -> Option<PathBuf> {
    std::env::home_dir().map(|home| home.join(".chxchx-tech-lead"))
}

/// True when `CHXCHX_TECH_HOME` decides the folder (it takes precedence over the pointer).
pub fn overridden_by_environment() -> bool {
    std::env::var_os("CHXCHX_TECH_HOME").is_some()
}

fn read_pointer(default: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(default.join(POINTER)).ok()?;
    let pointer: Pointer = serde_json::from_str(&text).ok()?;
    let path = PathBuf::from(pointer.path);
    (pointer.schema == SCHEMA && path.is_absolute()).then_some(path)
}

/// The folder chosen by the person when it is there; a pointer to a folder that is not (an unplugged
/// disk) is ignored so a new, empty folder is never started by surprise.
pub fn chosen_in(default: &Path) -> Option<PathBuf> {
    read_pointer(default).filter(|path| path.is_dir())
}

/// The pointer as written, even when the folder is missing (to say so).
pub fn pointed_in(default: &Path) -> Option<PathBuf> {
    read_pointer(default)
}

/// The chosen folder, read once per run so a change is seen after restarting, never halfway.
pub fn chosen() -> Option<PathBuf> {
    #[cfg(test)]
    {
        None // tests never read the real pointer; they pass their own folders
    }
    #[cfg(not(test))]
    {
        static CHOSEN: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
        CHOSEN
            .get_or_init(|| default_home().and_then(|default| chosen_in(&default)))
            .clone()
    }
}

fn expand(text: &str, home: Option<&Path>) -> PathBuf {
    let text = text.trim();
    if text == "~" {
        return home.map(Path::to_path_buf).unwrap_or_default();
    }
    if let Some(rest) = text.strip_prefix("~/")
        && let Some(home) = home
    {
        return home.join(rest);
    }
    PathBuf::from(text)
}

fn scan(root: &Path) -> Result<(u64, u64), String> {
    let (mut files, mut bytes) = (0, 0);
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if dir == root && entry.file_name() == POINTER {
                continue;
            }
            if kind.is_dir() {
                stack.push(entry.path());
            } else {
                files += 1;
                if kind.is_file() {
                    bytes += entry.metadata().map_err(|e| e.to_string())?.len();
                }
            }
        }
    }
    Ok((files, bytes))
}

/// Checks the destination and counts what would be copied. Nothing is written.
pub fn preview_in(from: &Path, home: Option<&Path>, to_text: &str) -> Result<Plan, String> {
    if to_text.trim().is_empty() {
        return Err("Escribe la carpeta nueva (ruta completa).".into());
    }
    let to = expand(to_text, home);
    if !to.is_absolute() {
        return Err("La carpeta debe ser una ruta completa (empieza por / o por ~/).".into());
    }
    if to.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("La ruta no puede contener «..».".into());
    }
    if to.parent().is_none() || Some(to.as_path()) == home {
        return Err(
            "Elige una carpeta propia para ChxChx, no la raíz ni tu carpeta personal.".into(),
        );
    }
    let (from_real, to_real) = (
        fs::canonicalize(from).unwrap_or_else(|_| from.to_path_buf()),
        // The nearest folder that exists, so a path not created yet is still compared.
        to.ancestors()
            .find_map(|a| {
                fs::canonicalize(a)
                    .ok()
                    .map(|r| r.join(to.strip_prefix(a).unwrap_or(Path::new(""))))
            })
            .unwrap_or_else(|| to.clone()),
    );
    if to_real == from_real {
        return Err("Esa ya es la carpeta que usas.".into());
    }
    if to_real.starts_with(&from_real) {
        return Err("La carpeta nueva no puede estar dentro de la actual.".into());
    }
    if from_real.starts_with(&to_real) {
        return Err("La carpeta nueva no puede contener a la actual.".into());
    }
    if to.exists() {
        if !to.is_dir() {
            return Err(format!("{} existe y no es una carpeta.", to.display()));
        }
        let occupied = fs::read_dir(&to)
            .map_err(|e| format!("{}: {e}", to.display()))?
            .next()
            .is_some();
        if occupied {
            return Err(format!(
                "{} no está vacía: elige una carpeta vacía o que no exista (no se mezcla con lo que haya).",
                to.display()
            ));
        }
    } else if let Some(ancestor) = to.ancestors().find(|a| a.exists())
        && !ancestor.is_dir()
    {
        return Err(format!("{} no es una carpeta.", ancestor.display()));
    }
    let (files, bytes) = scan(from)?;
    Ok(Plan {
        from: from.to_path_buf(),
        to,
        files,
        bytes,
    })
}

fn copy_tree(from: &Path, to: &Path, top: bool) -> Result<(u64, u64), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let (mut files, mut bytes) = (0, 0);
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if top && entry.file_name() == POINTER {
            continue;
        }
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            let (f, b) = copy_tree(&source, &target, false)?;
            files += f;
            bytes += b;
        } else if kind.is_symlink() {
            #[cfg(unix)]
            {
                let link = fs::read_link(&source).map_err(|e| e.to_string())?;
                std::os::unix::fs::symlink(link, &target)
                    .map_err(|e| format!("{}: {e}", target.display()))?;
                files += 1;
            }
        } else {
            bytes +=
                fs::copy(&source, &target).map_err(|e| format!("{}: {e}", source.display()))?;
            files += 1;
        }
    }
    if let Ok(meta) = fs::metadata(from) {
        let _ = fs::set_permissions(to, meta.permissions());
    }
    Ok((files, bytes))
}

fn write_pointer(default: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(default).map_err(|e| format!("{}: {e}", default.display()))?;
    let file = default.join(POINTER);
    if file.exists() {
        let _ = fs::copy(&file, default.join(format!("{POINTER}.bak")));
    }
    let text = serde_json::to_string_pretty(&Pointer {
        schema: SCHEMA.into(),
        path: to.display().to_string(),
    })
    .map_err(|e| e.to_string())?;
    let temp = default.join(format!("{POINTER}.tmp"));
    fs::write(&temp, format!("{text}\n")).map_err(|e| format!("{}: {e}", temp.display()))?;
    fs::rename(&temp, &file).map_err(|e| format!("{}: {e}", file.display()))
}

/// Copies everything to the new folder, checks it and only then points to it. If anything fails
/// the new folder is emptied again and the original is as it was.
pub fn apply_in(default: &Path, plan: &Plan) -> Result<Vec<String>, String> {
    let created = !plan.to.exists();
    let outcome = copy_tree(&plan.from, &plan.to, true).and_then(|copied| {
        let found = scan(&plan.to)?;
        if found != copied {
            return Err(format!(
                "La copia no coincide (se copiaron {} archivos y hay {}); no se cambió nada.",
                copied.0, found.0
            ));
        }
        write_pointer(default, &plan.to)?;
        Ok(copied)
    });
    match outcome {
        Ok((files, bytes)) => Ok(vec![
            format!(
                "Copiados {files} archivos ({}) a {}.",
                human(bytes),
                plan.to.display()
            ),
            format!(
                "La carpeta original ({}) no se tocó: es tu copia de seguridad. Bórrala tú cuando \
                 compruebes que todo funciona.",
                plan.from.display()
            ),
            "Reinicia Studio y el nodo para que usen la carpeta nueva.".into(),
        ]),
        Err(error) => {
            if created {
                let _ = fs::remove_dir_all(&plan.to);
            } else if let Ok(entries) = fs::read_dir(&plan.to) {
                for entry in entries.flatten() {
                    let _ =
                        fs::remove_dir_all(entry.path()).or_else(|_| fs::remove_file(entry.path()));
                }
            }
            Err(error)
        }
    }
}

/// Goes back to the original folder: the pointer is removed (kept as `.bak`). What was saved in the
/// other folder meanwhile is not copied back.
pub fn back_in(default: &Path) -> Result<Vec<String>, String> {
    let file = default.join(POINTER);
    let Some(was) = pointed_in(default) else {
        return Err("Ya usas la carpeta original.".into());
    };
    fs::rename(&file, default.join(format!("{POINTER}.bak")))
        .map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(vec![
        format!("Vuelves a la carpeta original ({}).", default.display()),
        format!(
            "Lo guardado desde el cambio sigue en {}; no se copia de vuelta.",
            was.display()
        ),
        "Reinicia Studio y el nodo para que lo usen.".into(),
    ])
}

pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let (mut value, mut unit) = (bytes as f64, 0);
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Why nothing can change now, or `None` when it can.
pub fn blocked() -> Option<String> {
    if overridden_by_environment() {
        return Some(
            "La variable CHXCHX_TECH_HOME define la carpeta y manda sobre este ajuste: quítala para elegirla aquí."
                .into(),
        );
    }
    #[cfg(feature = "node")]
    if crate::integrations::node_link::endpoint().is_some() {
        return Some(
            "El nodo está en marcha y escribe en la carpeta: detenlo antes (`chxchx-tech node stop`) y repite."
                .into(),
        );
    }
    None
}

/// The folder in use and where it comes from, in words.
pub fn describe() -> Vec<String> {
    let mut lines = Vec::new();
    let default = default_home();
    let current = crate::features::project_profiles::application_home();
    let shown = current
        .as_ref()
        .map_or("?".into(), |p| p.display().to_string());
    if overridden_by_environment() {
        lines.push(format!(
            "Carpeta de datos: {shown} (la define CHXCHX_TECH_HOME)."
        ));
    } else if let Some(default) = &default {
        match (pointed_in(default), chosen_in(default)) {
            (Some(_), Some(path)) => lines.push(format!("Carpeta de datos: {} (la elegiste tú).", path.display())),
            (Some(path), None) => lines.push(format!(
                "⚠ La carpeta elegida ({}) no está disponible: se usa la original ({}) hasta que vuelva.",
                path.display(),
                default.display()
            )),
            _ => lines.push(format!("Carpeta de datos: {shown} (la de siempre).")),
        }
    }
    lines
}

pub fn preview(to: &str) -> Result<Plan, String> {
    if let Some(reason) = blocked() {
        return Err(reason);
    }
    let from = crate::features::project_profiles::application_home()
        .ok_or("No se pudo resolver la carpeta de ChxChx.")?;
    preview_in(&from, std::env::home_dir().as_deref(), to)
}

pub fn apply(plan: &Plan) -> Result<Vec<String>, String> {
    if let Some(reason) = blocked() {
        return Err(reason);
    }
    let default = default_home().ok_or("No se pudo resolver tu carpeta personal.")?;
    apply_in(&default, plan)
}

pub fn back() -> Result<Vec<String>, String> {
    let default = default_home().ok_or("No se pudo resolver tu carpeta personal.")?;
    back_in(&default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-data-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fill(home: &Path) {
        fs::create_dir_all(home.join("node")).unwrap();
        fs::write(home.join("node/token"), "secreto").unwrap();
        fs::write(home.join("projects.json"), "{}").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(home.join("node"), fs::Permissions::from_mode(0o700)).unwrap();
            fs::set_permissions(home.join("node/token"), fs::Permissions::from_mode(0o600))
                .unwrap();
        }
    }

    #[test]
    fn a_change_copies_checks_and_only_then_points_leaving_the_original_alone() {
        let root = sandbox("ok");
        let (default, to) = (root.join("default"), root.join("elsewhere/data"));
        fill(&default);
        let plan = preview_in(&default, None, to.to_str().unwrap()).unwrap();
        assert_eq!((plan.files, plan.bytes), (2, 9));
        assert!(!to.exists(), "la vista previa no escribe nada");
        assert!(chosen_in(&default).is_none());
        let said = apply_in(&default, &plan).unwrap().join("\n");
        assert!(said.contains("no se tocó"), "{said}");
        assert_eq!(chosen_in(&default), Some(to.clone()));
        assert_eq!(
            fs::read_to_string(to.join("node/token")).unwrap(),
            "secreto"
        );
        assert!(default.join("node/token").exists(), "el original sigue ahí");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |p: PathBuf| fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(to.join("node")), 0o700);
            assert_eq!(
                mode(to.join("node/token")),
                0o600,
                "los secretos siguen privados"
            );
        }
        // The pointer is never copied into the new folder, and the second change is refused as occupied.
        assert!(!to.join(POINTER).exists());
        let again = preview_in(&default, None, to.to_str().unwrap()).unwrap_err();
        assert!(again.contains("no está vacía"), "{again}");
    }

    #[test]
    fn going_back_removes_the_pointer_and_says_what_stays() {
        let root = sandbox("back");
        let (default, to) = (root.join("default"), root.join("data"));
        fill(&default);
        let plan = preview_in(&default, None, to.to_str().unwrap()).unwrap();
        apply_in(&default, &plan).unwrap();
        let said = back_in(&default).unwrap().join("\n");
        assert!(said.contains("no se copia de vuelta"), "{said}");
        assert!(chosen_in(&default).is_none());
        assert!(default.join("data-location.json.bak").exists());
        assert!(back_in(&default).unwrap_err().contains("Ya usas"));
    }

    #[test]
    fn bad_destinations_are_refused_before_anything_is_written() {
        let root = sandbox("bad");
        let default = root.join("default");
        fill(&default);
        let full = root.join("full");
        fs::create_dir_all(&full).unwrap();
        fs::write(full.join("algo"), "x").unwrap();
        let check = |text: &str| preview_in(&default, Some(&root), text).unwrap_err();
        assert!(check("").contains("Escribe"));
        assert!(check("relativa/carpeta").contains("ruta completa"));
        assert!(check(&format!("{}/../x", root.display())).contains(".."));
        #[cfg(unix)]
        assert!(check("/").contains("propia"));
        assert!(check(root.to_str().unwrap()).contains("propia"));
        assert!(check(default.to_str().unwrap()).contains("ya es la carpeta"));
        assert!(check(default.join("dentro").to_str().unwrap()).contains("dentro de la actual"));
        assert!(check(full.to_str().unwrap()).contains("no está vacía"));
        assert!(!root.join("dentro").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_copy_leaves_the_new_folder_empty_and_no_pointer() {
        use std::os::unix::fs::PermissionsExt;
        let root = sandbox("fail");
        let (default, to) = (root.join("default"), root.join("data"));
        fill(&default);
        let plan = preview_in(&default, None, to.to_str().unwrap()).unwrap();
        // A file that cannot be read stops the copy halfway.
        fs::set_permissions(
            default.join("projects.json"),
            fs::Permissions::from_mode(0o000),
        )
        .unwrap();
        if fs::read(default.join("projects.json")).is_ok() {
            return; // running as root: nothing is unreadable
        }
        assert!(apply_in(&default, &plan).is_err());
        assert!(!to.exists(), "lo copiado a medias se quita");
        assert!(chosen_in(&default).is_none());
    }

    #[test]
    fn a_pointer_to_a_missing_folder_is_ignored_but_reported() {
        let root = sandbox("gone");
        let default = root.join("default");
        fill(&default);
        let gone = root.join("unplugged");
        write_pointer(&default, &gone).unwrap();
        assert_eq!(pointed_in(&default), Some(gone));
        assert!(chosen_in(&default).is_none());
    }

    #[test]
    fn a_broken_or_foreign_pointer_is_ignored() {
        let root = sandbox("junk");
        fs::write(root.join(POINTER), "no es json").unwrap();
        assert!(pointed_in(&root).is_none());
        fs::write(root.join(POINTER), r#"{"schema":"otro","path":"/x"}"#).unwrap();
        assert!(pointed_in(&root).is_none());
        fs::write(
            root.join(POINTER),
            r#"{"schema":"chxchx.data-location","path":"relativa"}"#,
        )
        .unwrap();
        assert!(pointed_in(&root).is_none());
    }
}
