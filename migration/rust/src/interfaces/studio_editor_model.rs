use super::*;
use std::path::Path;

pub(in crate::interfaces::studio) struct Tab {
    pub document: Document,
    pub content: text_editor::Content,
    pub dirty: bool,
    /// Generated views (Git diffs): selectable, never edited, saved or restored.
    pub readonly: bool,
    pub undo: std::collections::VecDeque<String>,
    pub redo: std::collections::VecDeque<String>,
}

pub(in crate::interfaces::studio) struct State {
    pub folders: BTreeMap<PathBuf, Vec<Entry>>,
    pub expanded: BTreeSet<PathBuf>,
    pub tabs: Vec<Tab>,
    pub selected: Option<usize>,
    pub home: bool,
    pub filter: String,
    pub query: String,
    pub search: bool,
    pub explorer: bool,
    pub activity: bool,
    pub saving: bool,
    pub closing: Option<usize>,
    pub exiting: bool,
    /// Match to select once the file requested by a search result has loaded.
    pub pending_cursor: Option<(PathBuf, usize, usize, usize)>,
}

impl State {
    pub fn new() -> Self {
        Self {
            folders: BTreeMap::new(),
            expanded: BTreeSet::from([PathBuf::new()]),
            tabs: Vec::new(),
            selected: None,
            home: true,
            filter: String::new(),
            query: String::new(),
            search: false,
            explorer: true,
            activity: false,
            saving: false,
            closing: None,
            exiting: false,
            pending_cursor: None,
        }
    }
}

/// Reopens the tabs of the previous session; files that vanished are skipped.
pub(in crate::interfaces::studio) fn restore(
    root: PathBuf,
    tabs: Vec<String>,
    active: Option<String>,
) -> Task<Message> {
    Task::perform(
        async move {
            jobs::offload(move || {
                tabs.iter()
                    .filter_map(|tab| project_files::read(&root, std::path::Path::new(tab)).ok())
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default()
        },
        move |documents| Message::Editor(Event::Restored(documents, active.clone())),
    )
}

pub(in crate::interfaces::studio) fn find_next(tab: &mut Tab, query: &str) {
    if query.is_empty() {
        return;
    }
    let cursor = tab.content.cursor().position;
    let lines: Vec<String> = tab
        .content
        .lines()
        .map(|line| line.text.to_string())
        .collect();
    for offset in 0..=lines.len() {
        let line = (cursor.line + offset) % lines.len();
        let skip = if offset == 0 {
            cursor.column.saturating_add(1)
        } else {
            0
        };
        let start = lines[line]
            .char_indices()
            .nth(skip)
            .map_or(lines[line].len(), |(i, _)| i);
        if let Some(found) = lines[line][start..].find(query) {
            let column = lines[line][..start + found].chars().count();
            tab.content.move_to(text_editor::Cursor {
                position: text_editor::Position {
                    line,
                    column: column + query.chars().count(),
                },
                selection: Some(text_editor::Position { line, column }),
            });
            break;
        }
    }
}

pub(in crate::interfaces::studio) fn select_path(editor: &mut State, path: &Path) {
    if let Some(index) = editor
        .tabs
        .iter()
        .position(|tab| tab.document.relative == path)
    {
        editor.selected = Some(index);
        editor.home = false;
    }
}

/// Moves the cursor to the pending search match, selecting the matched text.
pub(in crate::interfaces::studio) fn apply_cursor(editor: &mut State) {
    let Some((path, line, column, length)) = editor.pending_cursor.take() else {
        return;
    };
    if let Some(tab) = editor
        .tabs
        .iter_mut()
        .find(|tab| tab.document.relative == path)
    {
        let line = (line - 1).min(tab.content.line_count().saturating_sub(1));
        tab.content.move_to(text_editor::Cursor {
            position: text_editor::Position {
                line,
                column: column + length,
            },
            selection: Some(text_editor::Position { line, column }),
        });
    }
}

/// Adds the reopened tabs of the previous session; files that vanished were skipped.
pub(in crate::interfaces::studio) fn apply_restored(
    editor: &mut State,
    documents: Vec<Document>,
    active: Option<String>,
) {
    for document in documents {
        if editor.tabs.len() < 24
            && !editor
                .tabs
                .iter()
                .any(|tab| tab.document.relative == document.relative)
        {
            editor.tabs.push(Tab {
                content: text_editor::Content::with_text(&document.text),
                document,
                dirty: false,
                readonly: false,
                undo: Default::default(),
                redo: Default::default(),
            });
        }
    }
    // Only show the editor when the user left it open; otherwise stay on Home.
    let wanted = active.as_deref().and_then(|active| {
        editor
            .tabs
            .iter()
            .position(|tab| tab.document.relative.to_string_lossy().replace('\\', "/") == active)
    });
    if let Some(index) = wanted {
        editor.selected = Some(index);
        editor.home = false;
    } else if editor.selected.is_none() && !editor.tabs.is_empty() {
        editor.selected = Some(0);
    }
}

/// Minimal application state for unit tests of the Studio modules.
#[cfg(test)]
pub(in crate::interfaces::studio) fn fixture() -> Dashboard {
    Dashboard {
        editor: State::new(),
        terminal: Default::default(),
        agents: Default::default(),
        shell: Default::default(),
        catalog: Default::default(),
        session: session::fixture(),
        menu: Default::default(),
        search: Default::default(),
        git: Default::default(),
        page: Page::Home,
        root: PathBuf::from("."),
        overview: Err("fixture".into()),
        init_preview: None,
        sync_preview: None,
        rollback_preview: None,
        trust_preview: None,
        mcp_preview: None,
        projects_catalog: None,
        doctor_report: None,
        init_busy: false,
        sync_busy: false,
        mcp_busy: false,
        pending: 0,
        overview_loading: false,
        message: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(state: &mut Dashboard, name: &str) {
        let _ = update(
            state,
            Event::Loaded(Ok(Document {
                relative: PathBuf::from(name),
                text: "abc".into(),
                fingerprint: vec![],
            })),
        );
    }

    #[test]
    fn edits_undo_redo_and_dirty_state_follow_the_document() {
        let mut state = fixture();
        open(&mut state, "a.rs");
        let _ = update(
            &mut state,
            Event::Edit(text_editor::Action::Edit(text_editor::Edit::Insert('x'))),
        );
        assert!(state.editor.tabs[0].dirty);
        let changed = state.editor.tabs[0].content.text();
        let _ = update(&mut state, Event::Undo);
        assert_eq!(state.editor.tabs[0].content.text(), "abc");
        assert!(!state.editor.tabs[0].dirty);
        let _ = update(&mut state, Event::Redo);
        assert_eq!(state.editor.tabs[0].content.text(), changed);
        assert!(state.editor.tabs[0].dirty);
    }

    #[test]
    fn a_search_result_selects_its_match_whether_or_not_the_file_was_open() {
        let mut state = fixture();
        open(&mut state, "a.rs");
        let _ = update(&mut state, Event::OpenAt(PathBuf::from("a.rs"), 1, 1, 2));
        assert_eq!(
            state.editor.tabs[0].content.selection().as_deref(),
            Some("bc")
        );

        // A file that is not open yet loads first; the match is applied on arrival.
        let _ = update(&mut state, Event::OpenAt(PathBuf::from("b.rs"), 1, 0, 1));
        assert!(state.editor.pending_cursor.is_some());
        open(&mut state, "b.rs");
        assert_eq!(state.editor.selected, Some(1));
        assert_eq!(
            state.editor.tabs[1].content.selection().as_deref(),
            Some("a")
        );
        assert!(state.editor.pending_cursor.is_none());
    }

    #[test]
    fn pending_close_cannot_shift_to_another_dirty_tab() {
        let mut state = fixture();
        open(&mut state, "a.rs");
        open(&mut state, "b.rs");
        state.editor.tabs[0].dirty = true;
        let _ = update(&mut state, Event::Close(0));
        let _ = update(&mut state, Event::Close(1));
        assert_eq!(state.editor.tabs.len(), 2);
        assert_eq!(state.editor.closing, Some(0));
        let _ = update(&mut state, Event::Cancel);
        assert_eq!(state.editor.tabs.len(), 2);
    }
}
