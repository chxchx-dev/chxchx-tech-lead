//! The commands listed by each menu, and when they are available.
use super::shell::{DockTab, Event as ShellEvent, Side};
use super::{Menu, *};

pub(super) enum Entry {
    Separator,
    Item {
        label: std::borrow::Cow<'static, str>,
        /// The shortcut, as it is bound right now.
        hint: String,
        /// `None` shows the item disabled.
        message: Option<Message>,
    },
    /// An entry that unfolds a second list next to the dropdown.
    Submenu {
        label: &'static str,
        entries: Vec<Entry>,
    },
}

/// "Preferencias": settings, themes, interface size and shortcuts, one click from the File menu.
fn preferences(state: &Dashboard) -> Entry {
    let look = &state.session.preferences().appearance;
    let mark = |on: bool| if on { "✓" } else { "" };
    let mut entries = vec![
        item(
            "Ajustes…",
            key(keymap::Action::OpenSettings),
            Message::Navigate(Page::Settings),
        ),
        Entry::Separator,
    ];
    for (id, name, _) in design::themes::THEMES {
        entries.push(item(
            name,
            mark(look.theme == *id),
            Message::Settings(settings::Event::Theme((*id).to_owned())),
        ));
    }
    entries.push(Entry::Separator);
    for (label, value) in settings::SCALES {
        entries.push(item(
            label,
            mark((look.scale - value).abs() < 0.01),
            Message::Settings(settings::Event::Scale(value)),
        ));
    }
    entries.extend([
        Entry::Separator,
        item("Atajos de teclado…", "", Message::Navigate(Page::Settings)),
        item(
            "Restaurar atajos",
            "",
            Message::Settings(settings::Event::ResetKeys),
        ),
    ]);
    Entry::Submenu {
        label: "Preferencias",
        entries,
    }
}

fn item(label: &'static str, hint: impl Into<String>, message: Message) -> Entry {
    Entry::Item {
        label: label.into(),
        hint: hint.into(),
        message: Some(message),
    }
}

fn gated(label: &'static str, hint: impl Into<String>, enabled: bool, message: Message) -> Entry {
    Entry::Item {
        label: label.into(),
        hint: hint.into(),
        message: enabled.then_some(message),
    }
}

/// A folder name that fits one line of the submenu.
pub(super) fn shortened(name: &str) -> String {
    const MAX: usize = 24;
    if name.chars().count() <= MAX {
        name.to_owned()
    } else {
        let kept: String = name.chars().take(MAX - 1).collect();
        format!("{kept}…")
    }
}

/// The projects opened before, other than this one, ready to open in place of it.
fn recent_projects(state: &Dashboard) -> Entry {
    let here = state.root.canonicalize().ok();
    let mut entries: Vec<Entry> = state
        .session
        .recent_projects()
        .iter()
        .filter(|path| Path::new(path).canonicalize().ok() != here && Path::new(path).is_dir())
        .take(10)
        .map(|path| Entry::Item {
            label: shortened(
                &Path::new(path)
                    .file_name()
                    .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned()),
            )
            .into(),
            hint: String::new(),
            message: Some(Message::Session(session::Event::OpenFolder {
                path: PathBuf::from(path),
                new_window: false,
            })),
        })
        .collect();
    if entries.is_empty() {
        entries.push(gated(
            "Ningún proyecto reciente",
            "",
            false,
            Message::Refresh,
        ));
    }
    Entry::Submenu {
        label: "Abrir reciente",
        entries,
    }
}

/// Selects the open file in the explorer and runs an explorer action on it.
fn on_open_file(path: &Path, action: explorer::Action) -> Message {
    Message::Sequence(vec![
        Message::Explorer(explorer::Event::Select(path.to_path_buf())),
        Message::Explorer(explorer::Event::Do(action)),
    ])
}

/// The keys an action has right now (empty when it has none).
fn key(action: keymap::Action) -> String {
    let binding = keymap::binding(action);
    if binding.is_empty() {
        String::new()
    } else {
        keymap::display(&binding)
    }
}

/// Opens a section first so the review panel of the command is visible.
fn on_page(page: Page, message: Message) -> Message {
    Message::Sequence(vec![Message::Navigate(page), message])
}

pub(super) fn entries(menu: Menu, state: &Dashboard) -> Vec<Entry> {
    use Entry::Separator;
    let tab = tabs::selected_tab(state);
    let editing = tab.is_some();
    let root = state.root.display().to_string();
    match menu {
        Menu::File => {
            let current = tab.map(|tab| tab.document.relative.clone());
            let in_explorer = |action: explorer::Action| {
                Message::Sequence(vec![
                    Message::Shell(ShellEvent::ShowSide(Side::Explorer)),
                    Message::Explorer(explorer::Event::Do(action)),
                ])
            };
            vec![
                item(
                    "Nuevo archivo",
                    key(keymap::Action::NewFile),
                    in_explorer(explorer::Action::NewFile),
                ),
                item(
                    "Nueva carpeta",
                    "",
                    in_explorer(explorer::Action::NewFolder),
                ),
                Separator,
                item(
                    "Abrir proyecto…",
                    key(keymap::Action::OpenProject),
                    Message::Session(session::Event::PickProject { new_window: false }),
                ),
                item(
                    "Abrir proyecto en ventana nueva…",
                    "",
                    Message::Session(session::Event::PickProject { new_window: true }),
                ),
                recent_projects(state),
                Separator,
                gated(
                    "Guardar",
                    key(keymap::Action::Save),
                    tab.is_some_and(|tab| tab.dirty) && !state.editor.saving,
                    Message::Editor(editor::Event::Save),
                ),
                gated(
                    "Guardar todo",
                    key(keymap::Action::SaveAll),
                    state.editor.tabs.iter().any(|tab| tab.dirty) && !state.editor.saving,
                    Message::Editor(editor::Event::SaveAll),
                ),
                gated(
                    "Cerrar pestaña",
                    key(keymap::Action::CloseTab),
                    editing,
                    Message::Editor(editor::Event::Close(state.editor.selected.unwrap_or(0))),
                ),
                gated(
                    "Cerrar todas las pestañas",
                    "",
                    !state.editor.tabs.is_empty(),
                    Message::Editor(editor::Event::CloseAllTabs),
                ),
                Separator,
                gated(
                    "Copiar la ruta del archivo",
                    "",
                    current.is_some(),
                    current.as_deref().map_or(Message::Refresh, |path| {
                        on_open_file(path, explorer::Action::CopyPath)
                    }),
                ),
                gated(
                    "Mostrar en el explorador de archivos",
                    "",
                    current.is_some(),
                    current.as_deref().map_or(Message::Refresh, |path| {
                        on_open_file(path, explorer::Action::Reveal)
                    }),
                ),
                Separator,
                preferences(state),
                Separator,
                item(
                    "Nueva ventana",
                    "",
                    Message::Session(session::Event::OpenProject(root)),
                ),
                item(
                    "Actualizar explorador",
                    "",
                    Message::Editor(editor::Event::RefreshTree),
                ),
                Separator,
                item("Salir", "", Message::Editor(editor::Event::Exit)),
            ]
        }
        Menu::Edit => vec![
            gated(
                "Deshacer",
                "Ctrl+Z",
                editing,
                Message::Editor(editor::Event::Undo),
            ),
            gated(
                "Rehacer",
                "Ctrl+Y",
                editing,
                Message::Editor(editor::Event::Redo),
            ),
            Separator,
            gated(
                "Cortar",
                "Ctrl+X",
                editing,
                Message::Editor(editor::Event::Cut),
            ),
            gated(
                "Copiar",
                "Ctrl+C",
                editing,
                Message::Editor(editor::Event::Copy),
            ),
            gated(
                "Pegar",
                "Ctrl+V",
                editing,
                Message::Editor(editor::Event::Paste),
            ),
            gated(
                "Seleccionar todo",
                "Ctrl+A",
                editing,
                Message::Editor(editor::Event::SelectAll),
            ),
            Separator,
            gated(
                "Buscar en el archivo",
                "Ctrl+F",
                editing,
                Message::Editor(editor::Event::Search),
            ),
            item(
                "Buscar en el proyecto",
                key(keymap::Action::SearchProject),
                Message::Shell(ShellEvent::ShowSide(Side::Search)),
            ),
        ],
        Menu::View => vec![
            item(
                "Explorador",
                key(keymap::Action::ToggleSidebar),
                Message::Shell(ShellEvent::ShowSide(Side::Explorer)),
            ),
            item(
                "Buscar",
                key(keymap::Action::SearchProject),
                Message::Shell(ShellEvent::ShowSide(Side::Search)),
            ),
            item(
                "Agentes",
                "",
                Message::Shell(ShellEvent::ShowSide(Side::Agents)),
            ),
            item(
                "Espacio de trabajo",
                "",
                Message::Shell(ShellEvent::ShowSide(Side::Workspace)),
            ),
            Separator,
            item(
                "Panel inferior",
                key(keymap::Action::ToggleDock),
                Message::Shell(ShellEvent::ToggleDock),
            ),
            item(
                "Terminal",
                "",
                Message::Shell(ShellEvent::Dock(DockTab::Terminal)),
            ),
            item(
                "Actividad",
                "",
                Message::Shell(ShellEvent::Dock(DockTab::Activity)),
            ),
            Separator,
            item(
                "Acercar",
                key(keymap::Action::ZoomIn),
                Message::Settings(settings::Event::Zoom(1)),
            ),
            item(
                "Alejar",
                key(keymap::Action::ZoomOut),
                Message::Settings(settings::Event::Zoom(-1)),
            ),
            item(
                "Tamaño normal",
                key(keymap::Action::ZoomReset),
                Message::Settings(settings::Event::Zoom(0)),
            ),
            item(
                "Pantalla completa",
                key(keymap::Action::Fullscreen),
                Message::Shell(ShellEvent::ToggleFullscreen),
            ),
            Separator,
            item(
                "Restablecer diseño",
                "",
                Message::Shell(ShellEvent::ResetLayout),
            ),
        ],
        Menu::Go => vec![
            item("Inicio", "", Message::Navigate(Page::Home)),
            item("Proyecto y permisos", "", Message::Navigate(Page::Setup)),
            item("Proyectos", "", Message::Navigate(Page::Projects)),
            item(
                "Catálogo: skills, packs y mods",
                "",
                Message::Navigate(Page::Catalog),
            ),
            item(
                "Herramientas / MCP",
                "",
                Message::Navigate(Page::Integrations),
            ),
            item("Diagnóstico", "", Message::Navigate(Page::Diagnostics)),
            item(
                "Ajustes: temas y atajos",
                key(keymap::Action::OpenSettings),
                Message::Navigate(Page::Settings),
            ),
        ],
        Menu::Project => vec![
            item(
                "Preparar proyecto…",
                "",
                on_page(Page::Setup, Message::PreviewInit { minimal: false }),
            ),
            item(
                "Preparar versión compacta…",
                "",
                on_page(Page::Setup, Message::PreviewInit { minimal: true }),
            ),
            Separator,
            item(
                "Revisar confianza…",
                "",
                on_page(Page::Setup, Message::PreviewTrust { desired: true }),
            ),
            item(
                "Revocar confianza…",
                "",
                on_page(Page::Setup, Message::PreviewTrust { desired: false }),
            ),
            item(
                "Restaurar respaldo…",
                "",
                on_page(Page::Setup, Message::PreviewRollback),
            ),
            Separator,
            item(
                "Sincronizar proyectos…",
                "",
                on_page(Page::Projects, Message::PreviewSync),
            ),
            item(
                "Conectar herramientas (MCP)…",
                "",
                on_page(Page::Integrations, Message::PreviewMcp),
            ),
            Separator,
            item(
                "Diagnosticar proyecto",
                "",
                on_page(Page::Diagnostics, Message::Doctor),
            ),
            item("Actualizar proyecto", "", Message::Refresh),
        ],
        Menu::Git => {
            let repo = state.git.is_repository();
            let show = Message::Shell(ShellEvent::ShowSide(Side::Git));
            let in_panel = |message: Message| Message::Sequence(vec![show.clone(), message]);
            vec![
                item("Ver Git", "", show.clone()),
                Separator,
                gated(
                    "Actualizar estado",
                    "",
                    repo,
                    Message::Git(git::Event::Refresh),
                ),
                gated(
                    "Buscar novedades (fetch)",
                    "",
                    repo,
                    Message::Git(git::Event::Fetch),
                ),
                gated("Recibir", "", repo, Message::Git(git::Event::Pull)),
                gated(
                    "Enviar…",
                    "",
                    repo,
                    in_panel(Message::Git(git::Event::Ask(git::Confirm::Push))),
                ),
                Separator,
                gated(
                    "Nueva rama…",
                    "",
                    repo,
                    in_panel(Message::Git(git::Event::Tab(git::Tab::Branches))),
                ),
                gated(
                    "Apartar cambios (stash)",
                    "",
                    repo,
                    Message::Git(git::Event::StashPush),
                ),
                Separator,
                gated(
                    "Inicializar repositorio",
                    "",
                    state.git.loaded && !repo && state.git.unavailable.is_none(),
                    Message::Git(git::Event::Init),
                ),
            ]
        }
        Menu::Help => vec![
            item(
                "Documentación del proyecto",
                "",
                Message::Editor(editor::Event::Open(PathBuf::from("README.md"))),
            ),
            item(
                "Estado del proyecto",
                "",
                Message::Editor(editor::Event::Open(PathBuf::from(".ai/CURRENT_STATE.md"))),
            ),
            Separator,
            item(
                "Buscar actualizaciones…",
                "",
                Message::Update(update::Event::Open),
            ),
            item(
                "Acerca de ChxChx Studio",
                "",
                Message::Shell(ShellEvent::About),
            ),
        ],
    }
}
