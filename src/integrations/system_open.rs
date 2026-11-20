//! Shows a file or folder in the system's file manager (Finder, Explorer, the desktop's).
use super::executable_search::agent_path;
use std::path::Path;
use std::process::Command;

/// The program and arguments that show `path`: a file is selected in its folder where the system
/// can do that, a folder is opened.
pub fn command(path: &Path) -> (&'static str, Vec<String>) {
    let shown = path.display().to_string();
    if cfg!(target_os = "macos") {
        let flag = if path.is_dir() {
            vec![]
        } else {
            vec!["-R".to_owned()]
        };
        return ("open", [flag, vec![shown]].concat());
    }
    if cfg!(windows) {
        if path.is_dir() {
            return ("explorer", vec![shown]);
        }
        return ("explorer", vec![format!("/select,{shown}")]);
    }
    // Linux file managers have no common "select this file": open the folder that holds it.
    let folder = if path.is_dir() {
        shown
    } else {
        path.parent()
            .map_or(shown, |parent| parent.display().to_string())
    };
    ("xdg-open", vec![folder])
}

pub fn reveal(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("Ese elemento ya no existe".to_owned());
    }
    let (program, arguments) = command(path);
    Command::new(program)
        .args(&arguments)
        .env("PATH", agent_path())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("No pude abrir el explorador de archivos ({program}): {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_item_is_reported_before_anything_is_started() {
        assert!(
            reveal(Path::new("/no/existe/nunca"))
                .unwrap_err()
                .contains("ya no existe")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_opens_its_folder_and_a_folder_opens_itself() {
        let dir = std::env::temp_dir().join(format!("chxchx-reveal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.txt");
        std::fs::write(&file, "").unwrap();

        assert_eq!(
            command(&file),
            ("xdg-open", vec![dir.display().to_string()])
        );
        assert_eq!(command(&dir), ("xdg-open", vec![dir.display().to_string()]));
    }
}
