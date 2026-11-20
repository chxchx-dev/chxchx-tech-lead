//! Persistence glue: restores the last layout and session, and saves changes lazily.
use super::shell::DockTab;
use super::*;
use crate::features::studio_preferences::{
    self as storage, Layout, LoadedPreferences, Preferences, ProjectSession,
};
use std::time::Duration;

const SAVE_DELAY: Duration = Duration::from_millis(1500);

pub(super) struct State {
    preferences: Preferences,
    key: String,
    writable: bool,
    dirty: bool,
    saving: bool,
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Resized(iced::Size),
    Save,
    OpenProject(String),
    /// Asks for a folder with the system's picker and opens it.
    PickProject {
        new_window: bool,
    },
    /// Opens a folder as the project: in a window of its own, or in place of this one.
    OpenFolder {
        path: PathBuf,
        new_window: bool,
    },
}

pub(super) fn boot(root: &Path, loaded: LoadedPreferences) -> (State, Option<String>) {
    let LoadedPreferences {
        mut preferences,
        warning,
        writable,
    } = loaded;
    preferences.touch_recent_project(root);
    let state = State {
        preferences,
        key: storage::project_key(root),
        writable,
        dirty: writable,
        saving: false,
    };
    (state, warning)
}

/// Preferences that are never written, for unit tests.
#[cfg(test)]
pub(super) fn fixture() -> State {
    State {
        preferences: Preferences::default(),
        key: String::new(),
        writable: false,
        dirty: false,
        saving: false,
    }
}

impl State {
    pub fn layout(&self) -> &Layout {
        &self.preferences.layout
    }

    pub fn session(&self) -> ProjectSession {
        self.preferences
            .projects
            .get(&self.key)
            .cloned()
            .unwrap_or_default()
    }

    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    /// Changes the preferences and schedules them to be saved.
    pub fn edit(&mut self, change: impl FnOnce(&mut Preferences)) {
        change(&mut self.preferences);
        self.preferences.sanitize();
        self.dirty = true;
    }

    pub fn scale(&self) -> f32 {
        self.preferences.appearance.scale
    }

    pub fn recent_projects(&self) -> &[String] {
        &self.preferences.recent_projects
    }

    pub fn recent_files(&self) -> &[String] {
        self.preferences
            .projects
            .get(&self.key)
            .map_or(&[], |session| session.recent_files.as_slice())
    }

    pub fn needs_save(&self) -> bool {
        self.dirty && self.writable
    }
}

/// Applies the saved layout to a freshly created dashboard.
pub(super) fn apply(state: &mut Dashboard) {
    let layout = state.session.layout().clone();
    let session = state.session.session();
    state.shell.explorer_width = layout.explorer_width;
    state.shell.dock_height = layout.dock_height;
    state.editor.explorer = layout.explorer;
    state.shell.side = match layout.side.as_str() {
        "search" => shell::Side::Search,
        "workspace" => shell::Side::Workspace,
        "git" => shell::Side::Git,
        "agents" => shell::Side::Agents,
        _ => shell::Side::Explorer,
    };
    match layout.dock.as_deref() {
        Some("terminal") => shell::restore_dock(state, Some(DockTab::Terminal)),
        Some("activity") => shell::restore_dock(state, Some(DockTab::Activity)),
        _ => {}
    }
    state.page = Page::from_key(&session.page);
    for folder in &session.expanded {
        state.editor.expanded.insert(PathBuf::from(folder));
    }
}

/// Reads the saved tabs and reopens them without blocking the window.
pub(super) fn restore(state: &Dashboard) -> Task<Message> {
    let session = state.session.session();
    let root = state.root.clone();
    let folders: Vec<PathBuf> = session.expanded.iter().map(PathBuf::from).collect();
    let mut tasks: Vec<Task<Message>> = folders
        .into_iter()
        .map(|folder| editor::list(root.clone(), folder))
        .collect();
    if !session.tabs.is_empty() {
        tasks.push(editor::restore(root, session.tabs, session.active));
    }
    Task::batch(tasks)
}

/// Records a file the user opened at the front of the project's recent files.
pub(super) fn note_file(state: &mut Dashboard, relative: &Path) {
    let path = relative.to_string_lossy().replace('\\', "/");
    let key = state.session.key.clone();
    let session = state.session.preferences.projects.entry(key).or_default();
    storage::push_recent(&mut session.recent_files, &path, storage::MAX_RECENT_FILES);
    state.session.dirty = true;
}

/// Mirrors the visible layout and session into the preferences; marks them dirty on change.
pub(super) fn capture(state: &mut Dashboard) {
    let editor = &state.editor;
    let mut session = state.session.session();
    // Generated tabs (Git diffs) are not part of the saved session.
    let key = |tab: &editor::Tab| tab.document.relative.to_string_lossy().replace('\\', "/");
    session.tabs = editor
        .tabs
        .iter()
        .filter(|tab| !tab.readonly)
        .map(key)
        .collect();
    session.active = if editor.home {
        None
    } else {
        editor
            .selected
            .and_then(|index| editor.tabs.get(index))
            .filter(|tab| !tab.readonly)
            .map(key)
    };
    let mut expanded: Vec<String> = editor
        .expanded
        .iter()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect();
    expanded.truncate(storage::MAX_EXPANDED);
    session.expanded = expanded;
    session.page = state.page.key().to_owned();

    let layout = Layout {
        explorer: editor.explorer,
        side: match state.shell.side {
            shell::Side::Git => "git",
            shell::Side::Agents => "agents",
            shell::Side::Explorer => "explorer",
            shell::Side::Search => "search",
            shell::Side::Workspace => "workspace",
        }
        .to_owned(),
        dock: shell::dock_tab(state).map(|tab| match tab {
            DockTab::Terminal => "terminal".to_owned(),
            DockTab::Activity => "activity".to_owned(),
        }),
        explorer_width: state.shell.explorer_width,
        dock_height: state.shell.dock_height,
        window: state.session.preferences.layout.window,
    };
    let preferences = &mut state.session.preferences;
    let mut changed = false;
    if preferences.layout != layout {
        preferences.layout = layout;
        changed = true;
    }
    if preferences.projects.get(&state.session.key) != Some(&session) {
        preferences
            .projects
            .insert(state.session.key.clone(), session);
        changed = true;
    }
    state.session.dirty |= changed;
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Resized(size) => {
            let window = Some([size.width, size.height]);
            if state.session.preferences.layout.window != window {
                state.session.preferences.layout.window = window;
                state.session.dirty = true;
            }
        }
        Event::Save => {
            if !state.session.needs_save() || state.session.saving {
                return Task::none();
            }
            state.session.dirty = false;
            state.session.saving = true;
            let snapshot = state.session.preferences.clone();
            return jobs::quiet(
                move || storage::save(&snapshot),
                |state, result| {
                    state.session.saving = false;
                    if let Err(error) = result {
                        // Stop retrying every tick; the user keeps working with defaults.
                        state.session.writable = false;
                        state.message = error;
                    }
                    Task::none()
                },
            );
        }
        Event::PickProject { new_window } => {
            return Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Abrir proyecto")
                        .pick_folder()
                        .await
                        .map(|folder| folder.path().to_path_buf())
                },
                move |picked| match picked {
                    Some(path) => Message::Session(Event::OpenFolder { path, new_window }),
                    None => Message::Sequence(Vec::new()),
                },
            );
        }
        Event::OpenFolder { path, new_window } => return open_folder(state, path, new_window),
        Event::OpenProject(path) => {
            let target = PathBuf::from(&path);
            if !target.is_dir() {
                state.message = format!("La carpeta ya no existe: {path}");
            } else if let Err(error) = std::env::current_exe()
                .and_then(|exe| std::process::Command::new(exe).arg(&target).spawn())
            {
                state.message = format!("No se pudo abrir el proyecto: {error}");
            }
        }
    }
    Task::none()
}

/// Starts Studio on `path`. In place of this window, this one then closes (asking about unsaved
/// work first); if the person cancels that, both stay open.
fn open_folder(state: &mut Dashboard, path: PathBuf, new_window: bool) -> Task<Message> {
    if !path.is_dir() {
        state.message = format!("La carpeta ya no existe: {}", path.display());
        return Task::none();
    }
    let same = path.canonicalize().ok() == state.root.canonicalize().ok();
    if same && !new_window {
        state.message = "Ese proyecto ya está abierto en esta ventana.".into();
        return Task::none();
    }
    if let Err(error) =
        std::env::current_exe().and_then(|exe| std::process::Command::new(exe).arg(&path).spawn())
    {
        state.message = format!("No se pudo abrir el proyecto: {error}");
        return Task::none();
    }
    if new_window {
        Task::none()
    } else {
        Task::done(Message::Editor(editor::Event::Exit))
    }
}

/// Writes pending preferences right away; used while the window is closing.
pub(super) fn flush(state: &mut Dashboard) {
    if state.session.needs_save() {
        capture(state);
        let _ = storage::save(&state.session.preferences);
        state.session.dirty = false;
    }
}

pub(super) fn subscription(state: &Dashboard) -> iced::Subscription<Message> {
    let resized =
        iced::window::resize_events().map(|(_, size)| Message::Session(Event::Resized(size)));
    if state.session.needs_save() {
        iced::Subscription::batch([
            resized,
            iced::time::every(SAVE_DELAY).map(|_| Message::Session(Event::Save)),
        ])
    } else {
        resized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::project_files::Document;

    fn document(path: &str) -> Document {
        Document {
            relative: PathBuf::from(path),
            text: "contenido".into(),
            fingerprint: vec![],
        }
    }

    #[test]
    fn capture_records_layout_tabs_and_recent_files_once() {
        let mut state = editor::fixture();
        state.session.key = "proyecto".into();
        let _ = editor::update(&mut state, editor::Event::Loaded(Ok(document("src/a.rs"))));
        state.shell.explorer_width = 300.0;
        state.page = Page::Catalog;
        capture(&mut state);

        let session = state.session.session();
        assert_eq!(session.tabs, ["src/a.rs"]);
        assert_eq!(session.active.as_deref(), Some("src/a.rs"));
        assert_eq!(session.recent_files, ["src/a.rs"]);
        assert_eq!(session.page, "catalog");
        assert_eq!(state.session.layout().explorer_width, 300.0);
        assert!(state.session.dirty);

        state.session.dirty = false;
        capture(&mut state);
        assert!(
            !state.session.dirty,
            "an unchanged layout must not trigger a save"
        );

        let _ = editor::update(&mut state, editor::Event::Home);
        capture(&mut state);
        assert_eq!(
            state.session.session().active,
            None,
            "Home active means no editor tab"
        );
    }

    #[test]
    fn restored_tabs_skip_duplicates_and_select_the_saved_one() {
        let mut state = editor::fixture();
        editor::apply_restored(
            &mut state.editor,
            vec![document("a.rs"), document("b.rs"), document("a.rs")],
            Some("b.rs".into()),
        );
        assert_eq!(state.editor.tabs.len(), 2);
        assert_eq!(state.editor.selected, Some(1));
        assert!(!state.editor.home);

        let mut state = editor::fixture();
        editor::apply_restored(&mut state.editor, vec![document("a.rs")], None);
        assert!(
            state.editor.home,
            "without an active tab the session starts on Home"
        );
    }

    #[test]
    fn saved_layout_is_applied_to_a_new_dashboard() {
        let mut state = editor::fixture();
        state.session.preferences.layout.dock = Some("activity".into());
        state.session.preferences.layout.explorer = false;
        state.session.preferences.layout.side = "workspace".into();
        state.session.preferences.layout.dock_height = 300.0;
        apply(&mut state);
        assert_eq!(shell::dock_tab(&state), Some(DockTab::Activity));
        assert!(!state.editor.explorer);
        assert_eq!(state.shell.side, shell::Side::Workspace);
        assert_eq!(state.shell.dock_height, 300.0);
    }

    #[test]
    fn a_folder_that_vanished_or_is_already_open_is_not_launched() {
        let base = std::env::temp_dir().join(format!("chxchx-open-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let mut state = super::super::editor::fixture();
        state.root = base.clone();

        let _ = open_folder(&mut state, base.join("no-existe"), false);
        assert!(state.message.contains("ya no existe"));
        let task = open_folder(&mut state, base.clone(), false);
        drop(task);
        assert!(
            state.message.contains("ya está abierto"),
            "{}",
            state.message
        );
    }
}
