use super::*;
use crate::features::project_catalog::{ProjectCatalog, RegisteredProject};

fn registered(alias: &str, path: Option<&str>) -> RegisteredProject {
    RegisteredProject {
        alias: alias.into(),
        name: format!("Nombre de {alias}"),
        profile: "generic".into(),
        path: path.map(str::to_owned),
        stacks: Vec::new(),
        available: path.is_some(),
        is_current: false,
    }
}

#[test]
fn an_alias_resolves_to_the_canonical_folder_of_an_existing_project() {
    let dir = std::env::temp_dir();
    let catalog = ProjectCatalog {
        projects: vec![registered("app", Some(&dir.display().to_string()))],
        current: None,
    };

    let (alias, name, path) = resolve(&catalog, "app").unwrap();

    assert_eq!((alias.as_str(), name.as_str()), ("app", "Nombre de app"));
    assert_eq!(path, std::fs::canonicalize(&dir).unwrap());
}

#[test]
fn an_unknown_alias_or_a_project_gone_from_disk_is_refused_with_a_reason() {
    let catalog = ProjectCatalog {
        projects: vec![
            registered("viejo", Some("/no/existe/nunca")),
            registered("sin-ruta", None),
        ],
        current: None,
    };

    assert!(resolve(&catalog, "otro").unwrap_err().contains("alias"));
    assert!(resolve(&catalog, "viejo").unwrap_err().contains("disco"));
    assert!(resolve(&catalog, "sin-ruta").unwrap_err().contains("disco"));
}
