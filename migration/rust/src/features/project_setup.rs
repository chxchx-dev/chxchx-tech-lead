use crate::features::project_backup::backup_project;
use crate::features::project_config_migrations::plan_config_update;
use crate::features::project_config_template::memory_project_name;
use crate::features::project_overview::{inspect_project, normalize_canonical_path};
use crate::features::project_registry::{
    RegistryUpdate, apply_registration, check_registration_unchanged, plan_registration,
    restore_registry,
};
use crate::features::project_templates::{
    PROJECT_DIRECTORIES, claude_rules_body, initial_ai_files, project_rules_body,
    upsert_managed_block,
};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub(crate) struct ProjectFileChange {
    pub(crate) relative: String,
    pub(crate) before: Option<Vec<u8>>,
    pub(crate) after: Vec<u8>,
}

#[derive(Clone)]
pub struct ProjectInitPlan {
    pub project_root: String,
    pub minimal: bool,
    pub actions: Vec<String>,
    pub warnings: Vec<String>,
    pub blocked_reason: Option<String>,
    root: PathBuf,
    directories: Vec<PathBuf>,
    files: Vec<ProjectFileChange>,
    registry: Option<RegistryUpdate>,
    basic_memory_executable: Option<String>,
    memory_project: String,
    backup_required: bool,
}

pub struct ProjectInitResult {
    pub plan: ProjectInitPlan,
    pub applied: bool,
    pub completed_actions: Vec<String>,
    pub backups: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectInitApplication {
    pub completed_actions: Vec<String>,
    pub backups: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn initialize_project(
    path: &Path,
    apply: bool,
    minimal: bool,
) -> Result<ProjectInitResult, String> {
    let plan = plan_project_init(path, minimal)?;
    let (completed_actions, backups, warnings) = if apply {
        let application = apply_project_init(&plan)?;
        (
            application.completed_actions,
            application.backups,
            application.warnings,
        )
    } else {
        (Vec::new(), Vec::new(), plan.warnings.clone())
    };
    Ok(ProjectInitResult {
        plan,
        applied: apply,
        completed_actions,
        backups,
        warnings,
    })
}

pub fn plan_project_init(path: &Path, minimal: bool) -> Result<ProjectInitPlan, String> {
    let overview = inspect_project(path)?;
    let root = PathBuf::from(&overview.project.root);
    let mut actions = Vec::new();
    let mut warnings = Vec::new();
    let mut files = Vec::new();
    let mut directories = BTreeSet::new();
    let mut blocked_reason = invalid_project_paths(&root);

    if blocked_reason.is_none() {
        match plan_config_update(
            &root,
            &overview.project.name,
            &overview.detection,
            &overview.workspace,
        ) {
            Ok(Some(contents)) => {
                let relative = ".ai/chxchx-tech.toml";
                match plan_file_change(&root, relative, contents.into_bytes()) {
                    Ok(Some(change)) => {
                        let action = if overview.workspace.config_version == Some(1) {
                            "migrar .ai/chxchx-tech.toml de v1 a v2"
                        } else {
                            "crear .ai/chxchx-tech.toml"
                        };
                        actions.push(action.to_owned());
                        files.push(change);
                    }
                    Ok(None) => {}
                    Err(error) => blocked_reason = Some(error),
                }
            }
            Ok(None) => {}
            Err(error) => blocked_reason = Some(error),
        }
    }

    let directory_paths = if minimal {
        vec![".ai/memory"]
    } else {
        PROJECT_DIRECTORIES.to_vec()
    };
    if blocked_reason.is_none() {
        for relative in directory_paths {
            match missing_directory_chain(&root, relative) {
                Ok(paths) => {
                    for path in paths {
                        let relative = path.strip_prefix(&root).unwrap_or(&path);
                        let label = relative.to_string_lossy().replace('\\', "/");
                        if directories.insert(path) {
                            actions.push(format!("crear directorio {label}"));
                        }
                    }
                }
                Err(error) => {
                    blocked_reason = Some(error);
                    break;
                }
            }
        }
    }

    if !minimal && blocked_reason.is_none() {
        for (relative, contents) in initial_ai_files(&overview.project.name, &overview.detection) {
            match plan_missing_file(&root, &relative, contents.into_bytes()) {
                Ok(Some(change)) => {
                    actions.push(format!("crear {relative}"));
                    files.push(change);
                }
                Ok(None) => {}
                Err(error) => {
                    blocked_reason = Some(error);
                    break;
                }
            }
        }
    }

    if !minimal && blocked_reason.is_none() {
        match crate::features::mcp_setup::opencode_example_contents(&root, &overview.project.name) {
            Ok(contents) => match plan_file_change(
                &root,
                ".ai/integrations/opencode-mcp.example.json",
                contents.into_bytes(),
            ) {
                Ok(Some(change)) => {
                    actions.push("actualizar ejemplo MCP de OpenCode".to_owned());
                    files.push(change);
                }
                Ok(None) => {}
                Err(error) => blocked_reason = Some(error),
            },
            Err(error) => blocked_reason = Some(error),
        }
    }

    if !minimal && blocked_reason.is_none() {
        let managed = [
            (
                "AGENTS.md",
                "project-rules",
                project_rules_body(&root, &overview.project.name, &overview.detection),
            ),
            ("CLAUDE.md", "claude-rules", claude_rules_body().to_owned()),
        ];
        for (relative, block, body) in managed {
            match plan_managed_file(&root, relative, block, &body) {
                Ok(Some(change)) => {
                    actions.push(format!("actualizar bloque administrado en {relative}"));
                    files.push(change);
                }
                Ok(None) => {}
                Err(error) => {
                    blocked_reason = Some(error);
                    break;
                }
            }
        }
    }

    let basic_memory_executable = crate::integrations::basic_memory::command_name();
    let memory_project = memory_project_name(&root, &overview.project.name);
    if basic_memory_executable.is_some() && blocked_reason.is_none() {
        actions.push(format!(
            "asegurar el proyecto `{memory_project}` en Basic Memory"
        ));
    } else if blocked_reason.is_none() {
        warnings.push(
            "Basic Memory no está instalado; el proyecto quedó preparado localmente. Instálalo y vuelve a ejecutar init."
                .to_owned(),
        );
    }

    let registry = if blocked_reason.is_none() {
        match plan_registration(
            &overview.project.root,
            &overview.project.name,
            &overview.detection,
        ) {
            Ok(update) => {
                if update.changed {
                    actions.push("actualizar registro local de proyectos".to_owned());
                }
                Some(update)
            }
            Err(error) => {
                blocked_reason = Some(error);
                None
            }
        }
    } else {
        None
    };

    if !minimal {
        warnings.push(
            "Init no configura clientes MCP; usa `integrate --apply` después de inicializar."
                .to_owned(),
        );
    }
    let backup_required = (!files.is_empty() || !directories.is_empty())
        && ["AGENTS.md", "CLAUDE.md", ".ai"]
            .iter()
            .any(|relative| fs::symlink_metadata(root.join(relative)).is_ok());
    if backup_required && blocked_reason.is_none() {
        actions.push("crear backup del estado administrado antes de aplicar".to_owned());
    }
    if blocked_reason.is_some() {
        actions.clear();
        warnings.clear();
        files.clear();
        directories.clear();
    } else if actions.is_empty() {
        actions.push("todo está actualizado".to_owned());
    }
    Ok(ProjectInitPlan {
        project_root: root.display().to_string(),
        minimal,
        actions,
        warnings,
        blocked_reason,
        root,
        directories: directories.into_iter().collect(),
        files,
        registry,
        basic_memory_executable,
        memory_project,
        backup_required,
    })
}

pub fn apply_project_init(plan: &ProjectInitPlan) -> Result<ProjectInitApplication, String> {
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!(
            "No se aplicó init porque requiere atención: {reason}"
        ));
    }
    if let Some(reason) = invalid_project_paths(&plan.root) {
        return Err(format!(
            "No se aplicó init porque cambió la ruta de destino: {reason}"
        ));
    }
    validate_plan_snapshot(plan)?;
    if let Some(registry) = &plan.registry {
        check_registration_unchanged(registry)?;
    }

    let mut backups = Vec::new();
    if plan.backup_required
        && let Some(path) = backup_project(&plan.root)?
    {
        backups.push(path.display().to_string());
    }

    let mut created_directories = Vec::new();
    for directory in &plan.directories {
        match fs::create_dir(directory) {
            Ok(()) => created_directories.push(directory.clone()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if !directory.is_dir() || !directory_resolves_inside(&plan.root, directory) {
                    let rollback = rollback_project(&plan.root, &[], &created_directories);
                    return Err(append_rollback(
                        format!(
                            "El destino dejó de ser un directorio válido: {}",
                            directory.display()
                        ),
                        rollback,
                    ));
                }
            }
            Err(error) => {
                let rollback = rollback_project(&plan.root, &[], &created_directories);
                return Err(append_rollback(
                    format!("No se pudo crear un directorio del proyecto: {error}"),
                    rollback,
                ));
            }
        }
    }

    let mut applied_files = Vec::new();
    for change in &plan.files {
        if let Some(reason) = invalid_project_paths(&plan.root) {
            let rollback = rollback_project(&plan.root, &applied_files, &created_directories);
            return Err(append_rollback(
                format!("La ruta cambió durante init: {reason}"),
                rollback,
            ));
        }
        if let Err(error) = validate_file_snapshot(&plan.root, change) {
            let rollback = rollback_project(&plan.root, &applied_files, &created_directories);
            return Err(append_rollback(error, rollback));
        }
        if let Err(error) = write_project_file(&plan.root, change) {
            let rollback = rollback_project(&plan.root, &applied_files, &created_directories);
            return Err(append_rollback(error, rollback));
        }
        applied_files.push(change.clone());
    }

    if let Some(registry) = &plan.registry {
        match apply_registration(registry) {
            Ok(result) => {
                if let Some(path) = result.backup {
                    backups.push(path.display().to_string());
                }
            }
            Err(error) => {
                let mut rollback =
                    rollback_project(&plan.root, &applied_files, &created_directories);
                if let Err(registry_error) = restore_registry(registry) {
                    rollback.push(registry_error);
                }
                return Err(append_rollback(error, rollback));
            }
        }
    }

    let memory_action = format!(
        "asegurar el proyecto `{}` en Basic Memory",
        plan.memory_project
    );
    let mut completed = plan
        .actions
        .iter()
        .filter(|action| {
            action.as_str() != "todo está actualizado" && action.as_str() != memory_action
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut warnings = plan.warnings.clone();
    if let Some(executable) = &plan.basic_memory_executable {
        match crate::integrations::basic_memory::ensure_project(
            executable,
            &plan.memory_project,
            &plan.root.join(".ai").join("memory"),
            &plan.root,
        ) {
            Ok(true) => completed.push("proyecto Basic Memory registrado".to_owned()),
            Ok(false) => completed.push("proyecto Basic Memory ya estaba registrado".to_owned()),
            Err(error) => warnings.push(format!("Basic Memory: {error}")),
        }
    }
    if completed.is_empty() {
        completed.push("sin cambios locales".to_owned());
    }
    Ok(ProjectInitApplication {
        completed_actions: completed,
        backups,
        warnings,
    })
}

pub(crate) fn missing_directory_chain(root: &Path, relative: &str) -> Result<Vec<PathBuf>, String> {
    let target = root.join(relative);
    if !target.starts_with(root) {
        return Err("Ruta de directorio inválida para init".to_owned());
    }
    let mut missing = Vec::new();
    let mut current = target;
    while current != root {
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if !metadata.is_dir() && !metadata.file_type().is_symlink() {
                    return Err(format!(
                        "Se esperaba un directorio en {}",
                        current.strip_prefix(root).unwrap_or(&current).display()
                    ));
                }
                if metadata.file_type().is_symlink() && !directory_resolves_inside(root, &current) {
                    return Err("Un directorio de destino apunta fuera del proyecto".to_owned());
                }
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(current.clone());
                current = current
                    .parent()
                    .ok_or_else(|| "No se pudo resolver un directorio de init".to_owned())?
                    .to_owned();
            }
            Err(error) => return Err(format!("No se pudo inspeccionar un directorio: {error}")),
        }
    }
    missing.reverse();
    Ok(missing)
}

fn plan_missing_file(
    root: &Path,
    relative: &str,
    after: Vec<u8>,
) -> Result<Option<ProjectFileChange>, String> {
    let path = root.join(relative);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(format!("No se modifica un archivo enlazado: {relative}"))
        }
        Ok(metadata) if metadata.is_dir() => Err(format!("Se esperaba un archivo en {relative}")),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Some(ProjectFileChange {
            relative: relative.to_owned(),
            before: None,
            after,
        })),
        Err(error) => Err(format!("No se pudo inspeccionar {relative}: {error}")),
    }
}

pub(crate) fn plan_file_change(
    root: &Path,
    relative: &str,
    after: Vec<u8>,
) -> Result<Option<ProjectFileChange>, String> {
    let path = root.join(relative);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(format!("No se modifica un archivo enlazado: {relative}"))
        }
        Ok(metadata) if !metadata.is_file() => Err(format!("Se esperaba un archivo en {relative}")),
        Ok(_) => {
            let before =
                fs::read(&path).map_err(|error| format!("No se pudo leer {relative}: {error}"))?;
            if before == after {
                Ok(None)
            } else {
                Ok(Some(ProjectFileChange {
                    relative: relative.to_owned(),
                    before: Some(before),
                    after,
                }))
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Some(ProjectFileChange {
            relative: relative.to_owned(),
            before: None,
            after,
        })),
        Err(error) => Err(format!("No se pudo inspeccionar {relative}: {error}")),
    }
}

pub(crate) fn plan_managed_file(
    root: &Path,
    relative: &str,
    marker: &str,
    body: &str,
) -> Result<Option<ProjectFileChange>, String> {
    let path = root.join(relative);
    let before = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!("No se modifica un archivo enlazado: {relative}"));
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(format!("Se esperaba un archivo en {relative}"));
        }
        Ok(_) => {
            Some(fs::read(&path).map_err(|error| format!("No se pudo leer {relative}: {error}"))?)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("No se pudo inspeccionar {relative}: {error}")),
    };
    let existing = before
        .as_deref()
        .map(|bytes| {
            String::from_utf8(bytes.to_vec())
                .map_err(|_| format!("{relative} no contiene UTF-8 válido; se preservó"))
        })
        .transpose()?
        .unwrap_or_default();
    let after = upsert_managed_block(&existing, marker, body).into_bytes();
    if before.as_deref() == Some(after.as_slice()) {
        Ok(None)
    } else {
        Ok(Some(ProjectFileChange {
            relative: relative.to_owned(),
            before,
            after,
        }))
    }
}

fn validate_plan_snapshot(plan: &ProjectInitPlan) -> Result<(), String> {
    if !plan.root.is_dir() {
        return Err("La raíz del proyecto dejó de existir".to_owned());
    }
    for change in &plan.files {
        validate_file_snapshot(&plan.root, change)?;
    }
    for directory in &plan.directories {
        if directory.exists()
            && (!directory.is_dir() || !directory_resolves_inside(&plan.root, directory))
        {
            return Err(format!(
                "Cambió un directorio desde la vista previa: {}",
                directory.display()
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_file_snapshot(
    root: &Path,
    change: &ProjectFileChange,
) -> Result<(), String> {
    let path = root.join(&change.relative);
    let current = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!("Cambió {} desde la vista previa", change.relative));
        }
        Ok(metadata) if metadata.is_file() => Some(
            fs::read(&path)
                .map_err(|error| format!("No se pudo releer {}: {error}", change.relative))?,
        ),
        Ok(_) => return Err(format!("Cambió {} desde la vista previa", change.relative)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("No se pudo releer {}: {error}", change.relative)),
    };
    if current != change.before {
        return Err(format!(
            "{} cambió desde la vista previa; vuelve a previsualizar",
            change.relative
        ));
    }
    Ok(())
}

pub(crate) fn write_project_file(root: &Path, change: &ProjectFileChange) -> Result<(), String> {
    let path = root.join(&change.relative);
    let parent = path
        .parent()
        .ok_or_else(|| format!("No se pudo resolver la carpeta de {}", change.relative))?;
    if !directory_resolves_inside(root, parent) {
        return Err(format!(
            "La carpeta de {} sale del proyecto",
            change.relative
        ));
    }
    if change.before.is_none() {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("No se pudo crear {}: {error}", change.relative))?;
        if let Err(error) = file.write_all(&change.after).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&path);
            return Err(format!("No se pudo escribir {}: {error}", change.relative));
        }
        return Ok(());
    }
    let before = change.before.as_deref().unwrap_or_default();
    let temp = parent.join(format!(
        ".chxchx-{}-{}.tmp",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("No se pudo preparar {}: {error}", change.relative))?;
    if let Err(error) = file.write_all(&change.after).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(format!("No se pudo preparar {}: {error}", change.relative));
    }
    drop(file);
    if let Err(error) = validate_file_snapshot(root, change) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    if let Err(error) = fs::remove_file(&path) {
        let _ = fs::remove_file(&temp);
        return Err(format!(
            "No se pudo reemplazar {}: {error}",
            change.relative
        ));
    }
    if let Err(error) = fs::rename(&temp, &path) {
        let _ = fs::remove_file(&temp);
        return match write_new_file(&path, before) {
            Ok(()) => Err(format!("No se pudo guardar {}: {error}", change.relative)),
            Err(restore) => Err(format!(
                "No se pudo guardar {}: {error}; tampoco restaurar el original: {restore}",
                change.relative
            )),
        };
    }
    Ok(())
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("No se pudo restaurar {}: {error}", path.display()))?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("No se pudo restaurar {}: {error}", path.display()))
}

pub(crate) fn rollback_project(
    root: &Path,
    applied_files: &[ProjectFileChange],
    created_directories: &[PathBuf],
) -> Vec<String> {
    let mut errors = Vec::new();
    for change in applied_files.iter().rev() {
        let full_path = root.join(&change.relative);
        if fs::symlink_metadata(&full_path).is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            errors.push(format!(
                "No se revirtió {} porque ahora es un enlace simbólico",
                change.relative
            ));
            continue;
        }
        let current = fs::read(&full_path).ok();
        if current.as_deref() != Some(change.after.as_slice()) {
            errors.push(format!(
                "No se revirtió {} porque cambió después de aplicarse",
                change.relative
            ));
            continue;
        }
        match &change.before {
            Some(before) => {
                if let Err(error) = fs::remove_file(&full_path) {
                    errors.push(format!(
                        "No se pudo preparar la restauración de {}: {error}",
                        change.relative
                    ));
                } else if let Err(error) = write_new_file(&full_path, before) {
                    errors.push(error);
                }
            }
            None => {
                if let Err(error) = fs::remove_file(&full_path) {
                    errors.push(format!("No se pudo eliminar {}: {error}", change.relative));
                }
            }
        }
    }
    for directory in created_directories.iter().rev() {
        if let Err(error) = fs::remove_dir(directory)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            errors.push(format!(
                "No se pudo quitar {}: {error}",
                directory.display()
            ));
        }
    }
    errors
}

pub(crate) fn directory_resolves_inside(root: &Path, directory: &Path) -> bool {
    if directory == root {
        return true;
    }
    if !directory.exists() {
        return directory.starts_with(root);
    }
    fs::canonicalize(directory)
        .map(normalize_canonical_path)
        .is_ok_and(|canonical| canonical.starts_with(root))
}

pub(crate) fn invalid_project_paths(root: &Path) -> Option<String> {
    let ai_path = root.join(".ai");
    if fs::symlink_metadata(&ai_path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Some(".ai es un enlace simbólico; init no lo atraviesa".to_owned());
    }
    if ai_path.exists() {
        if !ai_path.is_dir() {
            return Some(".ai existe y no es un directorio".to_owned());
        }
        if !directory_resolves_inside(root, &ai_path) {
            return Some(".ai apunta fuera del proyecto".to_owned());
        }
    }
    None
}

pub(crate) fn append_rollback(error: String, rollback: Vec<String>) -> String {
    if rollback.is_empty() {
        error
    } else {
        format!("{error}. Rollback incompleto: {}", rollback.join("; "))
    }
}
