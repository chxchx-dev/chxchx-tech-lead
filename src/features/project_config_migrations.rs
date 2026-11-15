use crate::features::project_config::validate_project_config;
use crate::features::project_config_template::render_default_config;
use crate::features::project_detection::ProjectDetection;
use crate::features::project_overview::WorkspaceSummary;
use std::fs;
use std::path::Path;

pub fn plan_config_update(
    root: &Path,
    name: &str,
    detection: &ProjectDetection,
    workspace: &WorkspaceSummary,
) -> Result<Option<String>, String> {
    let path = root.join(".ai").join("chxchx-tech.toml");
    if !workspace.config_exists {
        return render_default_config(root, name, &detection.profile, detection).map(Some);
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("No se pudo leer la configuración existente: {error}"))?;
    let mut document = toml::from_str::<toml::Value>(&contents)
        .map_err(|_| "El TOML existente no se puede migrar; se preservó".to_owned())?;
    let version = document
        .get("version")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| "La configuración no contiene una versión entera".to_owned())?;
    match version {
        2 => {
            let validation = validate_project_config(&document, root);
            if !validation.supported {
                return Err(validation
                    .error
                    .unwrap_or_else(|| "La configuración v2 no es compatible".to_owned()));
            }
            Ok(None)
        }
        1 => {
            for key in ["profile", "memory_project", "memory_path"] {
                if document
                    .get(key)
                    .is_some_and(|value| value.as_str().is_none())
                {
                    return Err(format!("La configuración v1 tiene un campo {key} inválido"));
                }
            }
            if document.get("workspace").is_none() {
                let defaults = render_default_config(root, name, &detection.profile, detection)?;
                let defaults = toml::from_str::<toml::Value>(&defaults)
                    .map_err(|_| "No se pudo leer la configuración predeterminada".to_owned())?;
                let workspace = defaults.get("workspace").cloned().ok_or_else(|| {
                    "Falta workspace en la configuración predeterminada".to_owned()
                })?;
                document
                    .as_table_mut()
                    .ok_or_else(|| "La configuración debe ser una tabla TOML".to_owned())?
                    .insert("workspace".to_owned(), workspace);
            }
            document
                .as_table_mut()
                .ok_or_else(|| "La configuración debe ser una tabla TOML".to_owned())?
                .insert("version".to_owned(), toml::Value::Integer(2));
            let validation = validate_project_config(&document, root);
            if !validation.supported {
                return Err(validation
                    .error
                    .unwrap_or_else(|| "La configuración v1 no puede migrarse a v2".to_owned()));
            }
            toml::to_string_pretty(&document)
                .map(Some)
                .map_err(|error| format!("No se pudo renderizar la configuración migrada: {error}"))
        }
        other => Err(format!("Versión de configuración no soportada: {other}")),
    }
}
