use super::*;
use crate::features::project_files::{self, Document, Entry};
use iced::widget::text_editor;
use std::collections::{BTreeMap, BTreeSet};

#[path = "studio_editor_model.rs"]
mod model;
pub(super) use model::*;

#[derive(Clone, Debug)]
pub(super) enum Event {
    RefreshTree,
    Folder(PathBuf),
    Listed(PathBuf, Result<Vec<Entry>, String>),
    Open(PathBuf),
    /// Opens a file and selects a match: 1-based line (0 = none), column and length.
    OpenAt(PathBuf, usize, usize, usize),
    CloseCurrent,
    /// Opens read-only text (a Git diff) in its own tab, replacing an older copy.
    OpenText(PathBuf, String),
    Loaded(Result<Document, String>),
    Restored(Vec<Document>, Option<String>),
    Select(usize),
    Home,
    Edit(text_editor::Action),
    Undo,
    Redo,
    Search,
    Query(String),
    FindNext,
    Save,
    Saved(PathBuf, Result<Document, String>),
    Close(usize),
    Discard,
    Cancel,
    Filter(String),
    Exit,
    ExitDiscard,
}

pub(super) fn list(root: PathBuf, relative: PathBuf) -> Task<Message> {
    let directory = relative.clone();
    Task::perform(
        async move {
            jobs::offload(move || project_files::list(&root, &directory))
                .await
                .unwrap_or_else(|| Err("La lectura de la carpeta falló".into()))
        },
        move |result| Message::Editor(Event::Listed(relative.clone(), result)),
    )
}

pub(super) fn load(root: PathBuf, path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            jobs::offload(move || project_files::read(&root, &path))
                .await
                .unwrap_or_else(|| Err("No se pudo abrir el archivo".into()))
        },
        |result| Message::Editor(Event::Loaded(result)),
    )
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    let editor = &mut state.editor;
    match event {
        Event::RefreshTree => return list(state.root.clone(), PathBuf::new()),
        Event::Folder(path) => {
            if !editor.expanded.remove(&path) {
                editor.expanded.insert(path.clone());
                return list(state.root.clone(), path);
            }
        }
        Event::Listed(path, result) => match result {
            Ok(entries) => {
                editor.folders.insert(path, entries);
            }
            Err(error) => state.message = error,
        },
        Event::Open(path) => {
            if let Some(index) = editor
                .tabs
                .iter()
                .position(|tab| tab.document.relative == path)
            {
                editor.selected = Some(index);
                editor.home = false;
            } else {
                return load(state.root.clone(), path);
            }
        }
        Event::OpenAt(path, line, column, length) => {
            editor.pending_cursor = (line > 0).then(|| (path.clone(), line, column, length));
            if editor.tabs.iter().any(|tab| tab.document.relative == path) {
                select_path(editor, &path);
                apply_cursor(editor);
            } else {
                return load(state.root.clone(), path);
            }
        }
        Event::OpenText(relative, text) => {
            let content = text_editor::Content::with_text(&text);
            if let Some(index) = editor
                .tabs
                .iter()
                .position(|tab| tab.document.relative == relative)
            {
                editor.tabs[index].content = content;
                editor.selected = Some(index);
            } else if editor.tabs.len() < 24 {
                editor.tabs.push(Tab {
                    content,
                    document: Document {
                        relative,
                        text,
                        fingerprint: Vec::new(),
                    },
                    dirty: false,
                    readonly: true,
                    undo: Default::default(),
                    redo: Default::default(),
                });
                editor.selected = Some(editor.tabs.len() - 1);
            } else {
                state.message = "Cierra una pestaña antes de abrir más de 24.".into();
                return Task::none();
            }
            editor.home = false;
        }
        Event::CloseCurrent => {
            if !editor.home
                && let Some(index) = editor.selected
            {
                return update(state, Event::Close(index));
            }
        }
        Event::Restored(documents, active) => apply_restored(editor, documents, active),
        Event::Loaded(result) => match result {
            Ok(document) => {
                session::note_file(state, &document.relative);
                let editor = &mut state.editor;
                if let Some(index) = editor
                    .tabs
                    .iter()
                    .position(|tab| tab.document.relative == document.relative)
                {
                    editor.selected = Some(index);
                } else {
                    if editor.tabs.len() >= 24 {
                        state.message =
                            "Cierra una pestaña antes de abrir más de 24 archivos.".into();
                        return Task::none();
                    }
                    editor.tabs.push(Tab {
                        content: text_editor::Content::with_text(&document.text),
                        document,
                        dirty: false,
                        readonly: false,
                        undo: Default::default(),
                        redo: Default::default(),
                    });
                    editor.selected = Some(editor.tabs.len() - 1);
                }
                editor.home = false;
                apply_cursor(editor);
            }
            Err(error) => state.message = error,
        },
        Event::Select(index) => {
            editor.selected = Some(index);
            editor.home = false;
        }
        Event::Home => editor.home = true,
        Event::Edit(action) => {
            if !editor.home
                && let Some(tab) = editor.selected.and_then(|i| editor.tabs.get_mut(i))
            {
                if tab.readonly && matches!(&action, text_editor::Action::Edit(_)) {
                    return Task::none();
                }
                if matches!(&action, text_editor::Action::Edit(_)) {
                    tab.undo.push_back(tab.content.text());
                    tab.redo.clear();
                    while tab.undo.len() > 64
                        || tab.undo.iter().map(String::len).sum::<usize>() > 8 * 1024 * 1024
                    {
                        tab.undo.pop_front();
                    }
                }
                tab.content.perform(action);
                tab.dirty = tab.content.text() != tab.document.text;
            }
        }
        Event::Search => editor.search = !editor.search,
        Event::Query(query) => editor.query = query,
        Event::FindNext => {
            if let Some(tab) = editor.selected.and_then(|i| editor.tabs.get_mut(i)) {
                find_next(tab, &editor.query);
            }
        }
        Event::Undo | Event::Redo => {
            if let Some(tab) = editor.selected.and_then(|i| editor.tabs.get_mut(i)) {
                let (from, to) = if matches!(event, Event::Undo) {
                    (&mut tab.undo, &mut tab.redo)
                } else {
                    (&mut tab.redo, &mut tab.undo)
                };
                if let Some(text) = from.pop_back() {
                    to.push_back(tab.content.text());
                    tab.content = text_editor::Content::with_text(&text);
                    tab.dirty = text != tab.document.text;
                }
            }
        }
        Event::Save => {
            if editor.saving || editor.home {
                return Task::none();
            }
            if let Some(tab) = editor.selected.and_then(|i| editor.tabs.get(i)) {
                if !tab.dirty {
                    return Task::none();
                }
                let document = tab.document.clone();
                let relative = document.relative.clone();
                let text = tab.content.text();
                let root = state.root.clone();
                editor.saving = true;
                state.message = "Guardando archivo con respaldo…".into();
                return Task::perform(
                    async move {
                        jobs::offload(move || project_files::save(&root, &document, &text))
                            .await
                            .unwrap_or_else(|| Err("Falló el guardado".into()))
                    },
                    move |result| Message::Editor(Event::Saved(relative.clone(), result)),
                );
            }
        }
        Event::Saved(path, result) => {
            editor.saving = false;
            match result {
                Ok(document) => {
                    if let Some(tab) = editor.tabs.iter_mut().find(|t| t.document.relative == path)
                    {
                        tab.dirty = tab.content.text() != document.text;
                        tab.document = document;
                    }
                    state.message = "Archivo guardado. Respaldo anterior conservado.".into();
                    return git::poll(state);
                }
                Err(error) => state.message = error,
            }
        }
        Event::Close(index) => {
            if editor.closing.is_some() {
                return Task::none();
            }
            if editor.saving {
                return Task::none();
            }
            if editor.tabs.get(index).is_some_and(|tab| tab.dirty) {
                editor.closing = Some(index);
            } else {
                close(editor, index);
            }
        }
        Event::Discard => {
            if let Some(index) = editor.closing.take() {
                close(editor, index);
            }
        }
        Event::Cancel => {
            editor.closing = None;
            editor.exiting = false;
        }
        Event::Filter(value) => editor.filter = value,
        Event::Exit => {
            if editor.saving {
                state.message = "Espera a que termine el guardado.".into();
            } else if editor.tabs.iter().any(|tab| tab.dirty)
                || state.terminal.session.is_some()
                || state.terminal.opening
            {
                editor.exiting = true;
            } else {
                session::flush(state);
                return iced::exit();
            }
        }
        Event::ExitDiscard => {
            session::flush(state);
            return iced::exit();
        }
    }
    Task::none()
}

fn close(editor: &mut State, index: usize) {
    if index < editor.tabs.len() {
        editor.tabs.remove(index);
        editor.selected = editor.selected.and_then(|selected| {
            if editor.tabs.is_empty() {
                None
            } else if selected > index {
                Some(selected - 1)
            } else {
                Some(selected.min(editor.tabs.len() - 1))
            }
        });
        if editor.selected.is_none() {
            editor.home = true;
        }
    }
}
