//! What the editor offers on top of plain text: the Code/Preview view of Markdown and formatting
//! with the project's own tool (on request, or on every save when the project asks for it).
use super::*;
use crate::integrations::formatter;
use iced::widget::{markdown, text_editor};
use std::hash::{Hash, Hasher};

/// How a Markdown file is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::interfaces::studio) enum ViewMode {
    #[default]
    Code,
    Preview,
    Split,
}

pub(in crate::interfaces::studio) fn is_markdown(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdx"))
}

pub(in crate::interfaces::studio) fn mode_of(editor: &State, path: &std::path::Path) -> ViewMode {
    if !is_markdown(path) {
        return ViewMode::Code;
    }
    editor.modes.get(path).copied().unwrap_or_default()
}

fn digest(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Keeps the rendered Markdown of the open file current. Only files shown with a preview are parsed.
pub(super) fn refresh_preview(editor: &mut State) {
    let wanted = editor
        .selected
        .filter(|_| !editor.home)
        .and_then(|i| editor.tabs.get(i))
        .filter(|tab| mode_of(editor, &tab.document.relative) != ViewMode::Code)
        .map(|tab| (tab.document.relative.clone(), tab.content.text()));
    let Some((path, text)) = wanted else {
        editor.preview = None;
        return;
    };
    let key = digest(&text);
    if editor
        .preview
        .as_ref()
        .is_some_and(|(p, k, _)| *p == path && *k == key)
    {
        return;
    }
    editor.preview = Some((path, key, markdown::Content::parse(&text)));
}

pub(super) fn set_mode(editor: &mut State, mode: ViewMode) {
    let Some(path) = editor
        .selected
        .and_then(|i| editor.tabs.get(i))
        .map(|tab| tab.document.relative.clone())
    else {
        return;
    };
    editor.modes.insert(path, mode);
    refresh_preview(editor);
}

type Outcome = Option<Result<(&'static str, String), String>>;

/// Formats the open file off the UI thread. `save` continues with the save once it is done.
pub(super) fn format(state: &mut Dashboard, save: bool, forced: bool) -> Task<Message> {
    let Some(tab) = state
        .editor
        .selected
        .filter(|_| !state.editor.home)
        .and_then(|i| state.editor.tabs.get(i))
        .filter(|tab| !tab.readonly)
    else {
        return Task::none();
    };
    let (root, path, original) = (
        state.root.clone(),
        tab.document.relative.clone(),
        tab.content.text(),
    );
    if forced {
        state.message = "Formateando…".into();
    }
    let sent = original.clone();
    let target = path.clone();
    Task::perform(
        async move {
            jobs::offload(move || -> Outcome {
                let tool = formatter::formatter_for(&root, &target)?;
                Some(formatter::format(&root, &tool, &sent).map(|text| (tool.name, text)))
            })
            .await
            .flatten()
            .or_else(|| {
                forced.then(|| {
                    Err("No hay un formateador instalado para este tipo de archivo.".into())
                })
            })
        },
        move |outcome| {
            Message::Editor(Event::Formatted(
                path.clone(),
                original.clone(),
                outcome,
                save,
            ))
        },
    )
}

/// Whether saving should format first: only when the project asks for it.
pub(super) fn required(state: &Dashboard) -> bool {
    formatter::format_on_save(&state.root)
}

pub(super) fn formatted(
    state: &mut Dashboard,
    path: &std::path::Path,
    original: &str,
    outcome: Outcome,
    save: bool,
) -> Task<Message> {
    let editor = &mut state.editor;
    match outcome {
        Some(Ok((name, text))) => {
            if let Some(tab) = editor.tabs.iter_mut().find(|t| t.document.relative == path)
                && tab.content.text() == original
            {
                if text == original {
                    state.message = format!("Ya está formateado ({name}).");
                } else {
                    let cursor = tab.content.cursor();
                    tab.undo.push_back(original.to_owned());
                    tab.redo.clear();
                    tab.content = text_editor::Content::with_text(&text);
                    let line = cursor
                        .position
                        .line
                        .min(tab.content.line_count().saturating_sub(1));
                    tab.content.move_to(text_editor::Cursor {
                        position: text_editor::Position {
                            line,
                            column: cursor.position.column,
                        },
                        selection: None,
                    });
                    tab.dirty = text != tab.document.text;
                    state.message = format!("Formateado con {name}. Puedes deshacerlo con Ctrl+Z.");
                }
            } else {
                // The text changed while the tool ran: keep what the person typed.
                state.message = "El archivo cambió mientras se formateaba; no se aplicó.".into();
            }
        }
        Some(Err(error)) => state.message = error,
        None => {}
    }
    refresh_preview(&mut state.editor);
    if save {
        return Task::done(Message::Editor(Event::SaveNow));
    }
    Task::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn only_markdown_files_have_a_preview() {
        assert!(is_markdown(Path::new("docs/LEEME.MD")));
        assert!(!is_markdown(Path::new("src/main.rs")));
        let editor = State::new();
        assert_eq!(mode_of(&editor, Path::new("main.rs")), ViewMode::Code);
    }

    #[test]
    fn the_preview_follows_the_text_and_goes_away_in_code_view() {
        let mut state = super::super::fixture();
        let editor = &mut state.editor;
        editor.tabs.push(Tab {
            content: text_editor::Content::with_text("# Hola"),
            document: project_files::Document {
                relative: "notas.md".into(),
                text: "# Hola".into(),
                fingerprint: Vec::new(),
            },
            dirty: false,
            readonly: false,
            undo: Default::default(),
            redo: Default::default(),
        });
        editor.selected = Some(editor.tabs.len() - 1);
        editor.home = false;
        refresh_preview(editor);
        assert!(editor.preview.is_none(), "code view parses nothing");
        set_mode(editor, ViewMode::Split);
        let first = editor.preview.as_ref().map(|p| p.1).unwrap();
        let tab = editor.tabs.last_mut().unwrap();
        tab.content = text_editor::Content::with_text("# Otro");
        refresh_preview(editor);
        assert_ne!(editor.preview.as_ref().map(|p| p.1).unwrap(), first);
        set_mode(editor, ViewMode::Code);
        assert!(editor.preview.is_none());
    }

    #[test]
    fn formatted_text_replaces_the_file_only_if_it_did_not_change_meanwhile() {
        let mut state = super::super::fixture();
        state.editor.tabs.push(Tab {
            content: text_editor::Content::with_text("a  =  1"),
            document: project_files::Document {
                relative: "x.py".into(),
                text: "a  =  1".into(),
                fingerprint: Vec::new(),
            },
            dirty: false,
            readonly: false,
            undo: Default::default(),
            redo: Default::default(),
        });
        let path = Path::new("x.py");
        let _ = formatted(
            &mut state,
            path,
            "a  =  1",
            Some(Ok(("ruff", "a = 1\n".into()))),
            false,
        );
        let tab = state.editor.tabs.last().unwrap();
        assert_eq!(tab.content.text().trim_end(), "a = 1");
        assert!(tab.dirty && tab.undo.len() == 1);
        // Typing after the request was sent: the answer is stale and must not clobber it.
        let _ = formatted(
            &mut state,
            path,
            "a  =  1",
            Some(Ok(("ruff", "b\n".into()))),
            false,
        );
        assert_eq!(
            state.editor.tabs.last().unwrap().content.text().trim_end(),
            "a = 1"
        );
        assert!(state.message.contains("cambió"));
    }
}
