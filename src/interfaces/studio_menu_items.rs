//! The commands listed by each menu, and when they are available.
use super::shell::{DockTab, Event as ShellEvent, Side};
use super::{Menu, *};

pub(super) enum Entry {
    Separator,
    Item {
        label: &'static str,
        hint: &'static str,
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
        item("Ajustes…", "Ctrl+,", Message::Navigate(Page::Settings)),
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

fn item(label: &'static str, hint: &'static str, message: Message) -> Entry {
    Entry::Item {
        label,
        hint,
        message: Some(message),
    }
}

fn gated(label: &'static str, hint: &'static str, enabled: bool, message: Message) -> Entry {
    Entry::Item {
        label,
        hint,
        message: enabled.then_some(message),
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
        Menu::File => vec![
            gated(
                "Guardar",
                "Ctrl+S",
                tab.is_some_and(|tab| tab.dirty) && !state.editor.saving,
                Message::Editor(editor::Event::Save),
            ),
            gated(
                "Cerrar pestaña",
                "Ctrl+W",
                editing,
                Message::Editor(editor::Event::Close(state.editor.selected.unwrap_or(0))),
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
        ],
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
                "Buscar en el archivo",
                "Ctrl+F",
                editing,
                Message::Editor(editor::Event::Search),
            ),
            item(
                "Buscar en el proyecto",
                "Ctrl+Shift+F",
                Message::Shell(ShellEvent::ShowSide(Side::Search)),
            ),
        ],
        Menu::View => vec![
            item(
                "Explorador",
                "Ctrl+B",
                Message::Shell(ShellEvent::ShowSide(Side::Explorer)),
            ),
            item(
                "Buscar",
                "Ctrl+Shift+F",
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
                "Ctrl+J",
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
                "Ctrl+,",
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
