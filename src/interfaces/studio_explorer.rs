//! The explorer as a place to organize the project, the way editors do: select one or several
//! items, right-click for a menu, rename in place, create, copy, cut, paste, duplicate and
//! delete (to a trash that can be undone). The file work itself is `features::project_fs`;
//! this module is the state of the interaction and what each action asks of it.
use super::*;
use crate::features::project_fs::{self, Trashed};
use iced::Point;
use iced::keyboard::Modifiers;
use std::collections::BTreeSet;

#[path = "studio_explorer_ops.rs"]
mod ops;
#[path = "studio_explorer_view.rs"]
pub(super) mod view;

/// What a right-click landed on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// The empty area: the project folder itself.
    Root,
    Item {
        path: PathBuf,
        directory: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Open,
    NewFile,
    NewFolder,
    Rename,
    Duplicate,
    Copy,
    Cut,
    Paste,
    CopyName,
    CopyPath,
    CopyRelative,
    Reveal,
    Delete,
    Refresh,
    CollapseAll,
    UndoDelete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum EditKind {
    Rename(PathBuf),
    /// The folder the new file will be created in.
    NewFile(PathBuf),
    NewFolder(PathBuf),
}

/// The inline name box of a rename or of a new entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Edit {
    pub kind: EditKind,
    pub text: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Clipboard {
    pub paths: Vec<PathBuf>,
    pub cut: bool,
}

/// A row held down: a click when it is released in place, a drag once the pointer has moved.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Pressed {
    pub path: PathBuf,
    pub directory: bool,
    pub origin: Point,
    /// No Ctrl/Shift was held: releasing in place opens it.
    pub plain: bool,
}

/// How far the pointer must move with a row held down before it counts as a drag.
const DRAG_DISTANCE: f32 = 6.0;

pub(super) struct State {
    pub pressed: Option<Pressed>,
    pub dragging: bool,
    /// The row under the pointer while something is held down: where a drop would land.
    pub hover: Option<(PathBuf, bool)>,
    pub selection: BTreeSet<PathBuf>,
    pub anchor: Option<PathBuf>,
    /// The explorer owns the keyboard: its shortcuts apply, not those of an editor or chat.
    pub focused: bool,
    pub clipboard: Option<Clipboard>,
    pub menu: Option<(Target, Point)>,
    /// Where the pointer last was, relative to the explorer panel.
    pub pointer: Point,
    pub modifiers: Modifiers,
    pub edit: Option<Edit>,
    /// Items waiting for the person to confirm their deletion.
    pub confirm: Option<Vec<PathBuf>>,
    /// Deletions that can be undone, newest last.
    pub trashed: Vec<Vec<Trashed>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pressed: None,
            dragging: false,
            hover: None,
            selection: BTreeSet::new(),
            anchor: None,
            focused: false,
            clipboard: None,
            menu: None,
            pointer: Point::ORIGIN,
            modifiers: Modifiers::empty(),
            edit: None,
            confirm: None,
            trashed: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Pointer(Point),
    Modifiers(Modifiers),
    /// The left button went down on a row: it becomes selected, and a drag may start.
    Press(PathBuf, bool),
    /// The left button went up anywhere: ends a drag (dropping), or completes a click.
    Release,
    /// The pointer entered or left a row while something is held down.
    Hover(Option<(PathBuf, bool)>),
    /// A left click on a row, as a press and release in place amount to.
    #[cfg(test)]
    Click(PathBuf, bool),
    /// A file or folder dropped on the window from outside.
    Dropped(PathBuf),
    /// Selects one item (a menu acting on the open file).
    Select(PathBuf),
    /// A click on the empty area.
    ClickRoot,
    Context(Target),
    Do(Action),
    /// A keyboard shortcut: only acts while the explorer has the keyboard.
    Shortcut(Action),
    EditText(String),
    EditSubmit,
    ConfirmDelete,
    /// Escape: closes whatever is open, in the order it was opened.
    Escape,
}

pub(super) const INPUT: &str = "explorer-name";
const MAX_UNDO: usize = 10;

/// Where a new entry or a paste goes for this target: the folder itself, or a file's folder.
fn folder_of(target: &Target) -> PathBuf {
    match target {
        Target::Root => PathBuf::new(),
        Target::Item { path, directory } => {
            if *directory {
                path.clone()
            } else {
                path.parent().map(Path::to_path_buf).unwrap_or_default()
            }
        }
    }
}

/// The folders and files in the order the tree shows them, for Shift-click ranges.
fn visible(state: &Dashboard) -> Vec<PathBuf> {
    fn walk(state: &Dashboard, folder: &Path, out: &mut Vec<PathBuf>, depth: u16) {
        let Some(entries) = state.editor.folders.get(folder).filter(|_| depth <= 20) else {
            return;
        };
        for entry in entries {
            out.push(entry.relative.clone());
            if entry.directory && state.editor.expanded.contains(&entry.relative) {
                walk(state, &entry.relative, out, depth + 1);
            }
        }
    }
    let mut out = Vec::new();
    walk(state, Path::new(""), &mut out, 0);
    out
}

/// The items an action applies to: the selection when the target is part of it, else the target.
fn subjects(state: &Dashboard, target: &Target) -> Vec<PathBuf> {
    match target {
        Target::Root => Vec::new(),
        Target::Item { path, .. } => {
            if state.explorer.selection.contains(path) {
                state.explorer.selection.iter().cloned().collect()
            } else {
                vec![path.clone()]
            }
        }
    }
}

/// The target of a keyboard shortcut or a menu pick.
fn target_of(state: &Dashboard) -> Target {
    if let Some((target, _)) = &state.explorer.menu {
        return target.clone();
    }
    match state.explorer.selection.iter().next_back() {
        Some(path) => Target::Item {
            path: path.clone(),
            directory: is_directory(state, path),
        },
        None => Target::Root,
    }
}

fn is_directory(state: &Dashboard, path: &Path) -> bool {
    state
        .editor
        .folders
        .get(path.parent().unwrap_or(Path::new("")))
        .and_then(|entries| entries.iter().find(|entry| entry.relative == path))
        .is_some_and(|entry| entry.directory)
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Pointer(point) => {
            state.explorer.pointer = point;
            if let Some(pressed) = &state.explorer.pressed
                && !state.explorer.dragging
                && pressed.origin.distance(point) > DRAG_DISTANCE
            {
                state.explorer.dragging = true;
                state.explorer.menu = None;
            }
        }
        Event::Press(path, directory) => press(state, path, directory),
        Event::Release => return release(state),
        Event::Hover(target) => {
            if state.explorer.pressed.is_some() {
                state.explorer.hover = target;
            }
        }
        Event::Dropped(path) => return dropped(state, path),
        Event::Select(path) => {
            state.explorer.selection = BTreeSet::from([path.clone()]);
            state.explorer.anchor = Some(path);
        }
        Event::Modifiers(modifiers) => state.explorer.modifiers = modifiers,
        #[cfg(test)]
        #[cfg(test)]
        Event::Click(path, directory) => return click(state, path, directory),
        Event::ClickRoot => {
            state.explorer.menu = None;
            state.explorer.focused = true;
            state.explorer.selection.clear();
            state.explorer.anchor = None;
        }
        Event::Context(target) => {
            if let Target::Item { path, .. } = &target
                && !state.explorer.selection.contains(path)
            {
                state.explorer.selection = BTreeSet::from([path.clone()]);
                state.explorer.anchor = Some(path.clone());
            }
            state.explorer.focused = true;
            state.explorer.menu = Some((target, state.explorer.pointer));
        }
        Event::Do(action) => return run(state, action),
        Event::Shortcut(action) => {
            let active = state.explorer.focused
                && state.editor.explorer
                && state.shell.side == shell::Side::Explorer
                && state.explorer.edit.is_none();
            if active {
                return run(state, action);
            }
        }
        Event::EditText(text) => {
            if let Some(edit) = state.explorer.edit.as_mut() {
                edit.error = (!text.is_empty())
                    .then(|| project_fs::validate_name(&text).err())
                    .flatten();
                edit.text = text;
            }
        }
        Event::EditSubmit => return submit(state),
        Event::ConfirmDelete => return confirm_delete(state),
        Event::Escape => escape(&mut state.explorer),
    }
    Task::none()
}

/// Whatever the person does elsewhere closes the explorer's menu and, when it is typing or
/// clicking in another part of the window, hands the keyboard back so its keys are not taken.
pub(super) fn interacted(state: &mut Dashboard, message: &Message) {
    let passive = matches!(
        message,
        Message::Explorer(_)
            | Message::Sequence(_)
            | Message::Done(_)
            | Message::Session(_)
            | Message::Git(git::Event::Poll)
    );
    if passive {
        return;
    }
    state.explorer.menu = None;
    let elsewhere = matches!(
        message,
        Message::Panes(_)
            | Message::Search(_)
            | Message::Terminal(terminal::Event::Key(..) | terminal::Event::Focus(_, true))
            | Message::Chat(chat::Event::Edit(_))
            | Message::Editor(editor::Event::Edit(_))
    );
    if elsewhere {
        state.explorer.focused = false;
    }
}

fn escape(explorer: &mut State) {
    if explorer.menu.take().is_some() {
        return;
    }
    if explorer.confirm.take().is_some() {
        return;
    }
    explorer.edit = None;
}

fn press(state: &mut Dashboard, path: PathBuf, directory: bool) {
    state.explorer.menu = None;
    state.explorer.edit = None;
    state.explorer.focused = true;
    let modifiers = state.explorer.modifiers;
    let plain = !(modifiers.command() || modifiers.shift());
    if plain {
        // Pressing inside a multiple selection keeps it, so all of it can be dragged.
        if !state.explorer.selection.contains(&path) {
            state.explorer.selection = BTreeSet::from([path.clone()]);
            state.explorer.anchor = Some(path.clone());
        }
    } else {
        // Extending a selection is done on press; there is nothing to open afterwards.
        let _ = click(state, path.clone(), directory);
    }
    state.explorer.dragging = false;
    state.explorer.hover = None;
    state.explorer.pressed = Some(Pressed {
        path,
        directory,
        origin: state.explorer.pointer,
        plain,
    });
}

fn release(state: &mut Dashboard) -> Task<Message> {
    let Some(pressed) = state.explorer.pressed.take() else {
        return Task::none();
    };
    let dragging = std::mem::take(&mut state.explorer.dragging);
    let hover = state.explorer.hover.take();
    if !dragging {
        return if pressed.plain {
            click(state, pressed.path, pressed.directory)
        } else {
            Task::none()
        };
    }
    let items = if state.explorer.selection.contains(&pressed.path) {
        state.explorer.selection.iter().cloned().collect()
    } else {
        vec![pressed.path]
    };
    let destination = hover.map_or_else(PathBuf::new, |(path, directory)| {
        folder_of(&Target::Item { path, directory })
    });
    let copy = state.explorer.modifiers.command();
    state.editor.expanded.insert(destination.clone());
    ops::spawn(
        state,
        ops::Job::Paste(
            Clipboard {
                paths: items,
                cut: !copy,
            },
            destination,
        ),
    )
}

/// Something from outside was dropped on the window: it is copied into the folder in use.
fn dropped(state: &mut Dashboard, path: PathBuf) -> Task<Message> {
    if !state.editor.explorer || state.shell.side != shell::Side::Explorer {
        return Task::none();
    }
    let destination = folder_of(&target_of(state));
    state.editor.expanded.insert(destination.clone());
    ops::spawn(state, ops::Job::Import(vec![path], destination))
}

fn click(state: &mut Dashboard, path: PathBuf, directory: bool) -> Task<Message> {
    state.explorer.menu = None;
    state.explorer.edit = None;
    state.explorer.focused = true;
    let modifiers = state.explorer.modifiers;
    if modifiers.command() {
        if !state.explorer.selection.remove(&path) {
            state.explorer.selection.insert(path.clone());
        }
        state.explorer.anchor = Some(path);
        return Task::none();
    }
    if modifiers.shift() {
        let rows = visible(state);
        let anchor = state
            .explorer
            .anchor
            .clone()
            .unwrap_or_else(|| path.clone());
        if let (Some(from), Some(to)) = (
            rows.iter().position(|row| *row == anchor),
            rows.iter().position(|row| *row == path),
        ) {
            let (low, high) = (from.min(to), from.max(to));
            state.explorer.selection = rows[low..=high].iter().cloned().collect();
            return Task::none();
        }
    }
    state.explorer.selection = BTreeSet::from([path.clone()]);
    state.explorer.anchor = Some(path.clone());
    Task::done(Message::Editor(if directory {
        editor::Event::Folder(path)
    } else {
        editor::Event::Open(path)
    }))
}

fn copy_text(state: &mut Dashboard, text: String, done: &str) -> Task<Message> {
    state.message = done.to_owned();
    iced::clipboard::write(text)
}

fn run(state: &mut Dashboard, action: Action) -> Task<Message> {
    let target = target_of(state);
    let items = subjects(state, &target);
    state.explorer.menu = None;
    state.explorer.focused = true;
    let root = state.root.clone();
    match action {
        Action::Open => {
            let Target::Item { path, directory } = target else {
                return Task::none();
            };
            return Task::done(Message::Editor(if directory {
                editor::Event::Folder(path)
            } else {
                editor::Event::Open(path)
            }));
        }
        Action::NewFile | Action::NewFolder => {
            let parent = folder_of(&target);
            let mut tasks = vec![];
            if state.editor.expanded.insert(parent.clone())
                || !state.editor.folders.contains_key(&parent)
            {
                tasks.push(editor::list(root, parent.clone()));
            }
            state.explorer.confirm = None;
            state.explorer.edit = Some(Edit {
                kind: if action == Action::NewFile {
                    EditKind::NewFile(parent)
                } else {
                    EditKind::NewFolder(parent)
                },
                text: String::new(),
                error: None,
            });
            tasks.push(iced::widget::operation::focus(INPUT));
            return Task::batch(tasks);
        }
        Action::Rename => {
            let Target::Item { path, .. } = &target else {
                return Task::none();
            };
            if items.len() > 1 {
                state.message = "Renombra los elementos de uno en uno.".into();
                return Task::none();
            }
            state.explorer.confirm = None;
            state.explorer.edit = Some(Edit {
                kind: EditKind::Rename(path.clone()),
                text: name_of(path),
                error: None,
            });
            return Task::batch([
                iced::widget::operation::focus(INPUT),
                iced::widget::operation::select_all(INPUT),
            ]);
        }
        Action::Duplicate => {
            if items.is_empty() {
                return Task::none();
            }
            return ops::spawn(state, ops::Job::Duplicate(items));
        }
        Action::Copy | Action::Cut => {
            if items.is_empty() {
                return Task::none();
            }
            state.message = format!(
                "{} {}.",
                items.len(),
                if action == Action::Cut {
                    if items.len() == 1 {
                        "elemento cortado"
                    } else {
                        "elementos cortados"
                    }
                } else if items.len() == 1 {
                    "elemento copiado"
                } else {
                    "elementos copiados"
                }
            );
            state.explorer.clipboard = Some(Clipboard {
                paths: items,
                cut: action == Action::Cut,
            });
        }
        Action::Paste => {
            let destination = folder_of(&target);
            let Some(clipboard) = state.explorer.clipboard.clone() else {
                // Nothing copied inside Studio: files copied in a file manager are pasted instead.
                state.editor.expanded.insert(destination.clone());
                return ops::spawn(state, ops::Job::PasteSystem(destination));
            };
            if state.editor.expanded.insert(destination.clone()) {
                // The folder opens so the pasted items are visible once they land.
            }
            return ops::spawn(state, ops::Job::Paste(clipboard, destination));
        }
        Action::CopyName => {
            let names: Vec<String> = items.iter().map(|path| name_of(path)).collect();
            if names.is_empty() {
                return Task::none();
            }
            return copy_text(state, names.join("\n"), "Nombre copiado al portapapeles.");
        }
        Action::CopyRelative => {
            let paths: Vec<String> = items.iter().map(|path| slashes(path)).collect();
            if paths.is_empty() {
                return Task::none();
            }
            return copy_text(
                state,
                paths.join("\n"),
                "Ruta relativa copiada al portapapeles.",
            );
        }
        Action::CopyPath => {
            let paths: Vec<String> = if items.is_empty() {
                vec![root.display().to_string()]
            } else {
                items
                    .iter()
                    .map(|path| root.join(path).display().to_string())
                    .collect()
            };
            return copy_text(state, paths.join("\n"), "Ruta copiada al portapapeles.");
        }
        Action::Reveal => {
            let path = items
                .first()
                .map_or_else(|| root.clone(), |path| root.join(path));
            return ops::spawn(state, ops::Job::Reveal(path));
        }
        Action::Delete => {
            if items.is_empty() {
                return Task::none();
            }
            state.explorer.edit = None;
            state.explorer.confirm = Some(items);
        }
        Action::Refresh => return ops::refresh(state),
        Action::CollapseAll => {
            state
                .editor
                .expanded
                .retain(|path| path.as_os_str().is_empty());
        }
        Action::UndoDelete => {
            let Some(batch) = state.explorer.trashed.pop() else {
                state.message = "No hay eliminaciones que deshacer.".into();
                return Task::none();
            };
            return ops::spawn(state, ops::Job::Restore(batch));
        }
    }
    Task::none()
}

fn submit(state: &mut Dashboard) -> Task<Message> {
    let Some(edit) = state.explorer.edit.clone() else {
        return Task::none();
    };
    if let Err(error) = project_fs::validate_name(&edit.text) {
        if let Some(open) = state.explorer.edit.as_mut() {
            open.error = Some(error);
        }
        return Task::none();
    }
    let job = match edit.kind {
        EditKind::NewFile(parent) => ops::Job::NewFile(parent, edit.text),
        EditKind::NewFolder(parent) => ops::Job::NewFolder(parent, edit.text),
        EditKind::Rename(path) => ops::Job::Rename(path, edit.text),
    };
    ops::spawn(state, job)
}

fn confirm_delete(state: &mut Dashboard) -> Task<Message> {
    let Some(items) = state.explorer.confirm.take() else {
        return Task::none();
    };
    ops::spawn(state, ops::Job::Delete(items))
}

/// How many deletions can still be undone (the explorer's header shows the button while > 0).
pub(super) fn undo_depth(state: &Dashboard) -> usize {
    state.explorer.trashed.len()
}

pub(super) fn remember_deletion(explorer: &mut State, batch: Vec<Trashed>) {
    explorer.trashed.push(batch);
    let excess = explorer.trashed.len().saturating_sub(MAX_UNDO);
    explorer.trashed.drain(..excess);
}

#[cfg(test)]
#[path = "studio_explorer_tests.rs"]
mod tests;
