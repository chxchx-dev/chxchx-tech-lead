//! Files that the system's clipboard holds (copied in a file manager), so they can be pasted
//! into the project.

/// The paths on the system clipboard, or none when it holds something else or is unavailable.
pub fn files() -> Vec<std::path::PathBuf> {
    arboard::Clipboard::new()
        .ok()
        .and_then(|mut clipboard| clipboard.get().file_list().ok())
        .unwrap_or_default()
}
