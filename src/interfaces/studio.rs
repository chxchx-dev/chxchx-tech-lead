use crate::features::project_overview::{ProjectOverview, inspect_project};
use crate::features::project_setup::ProjectInitPlan;
use iced::widget::{Space, button, container, row, text};
use iced::{Element, Fill, Task};

#[path = "studio_actions.rs"]
mod actions;
#[path = "studio_agents.rs"]
mod agents;
#[path = "studio_brand.rs"]
mod brand;
#[path = "studio_catalog.rs"]
mod catalog;
#[path = "studio_chat.rs"]
mod chat;
#[path = "studio_ide_chrome.rs"]
mod chrome;
#[path = "studio_design.rs"]
mod design;
#[path = "studio_ide_dock.rs"]
mod dock;
#[cfg(all(test, unix))]
#[path = "studio_e2e_tests.rs"]
mod e2e_tests;
#[path = "studio_editor.rs"]
mod editor;
#[path = "studio_file_types.rs"]
mod file_types;
#[path = "studio_git.rs"]
mod git;
#[path = "studio_git_view.rs"]
mod git_view;
#[path = "studio_home.rs"]
mod home;
#[path = "studio_icons.rs"]
mod icons;
#[path = "studio_ide_view.rs"]
mod ide;
#[path = "studio_jobs.rs"]
mod jobs;
#[path = "studio_view.rs"]
mod layout;
#[path = "studio_learning.rs"]
mod learning;
#[path = "studio_local.rs"]
mod local;
#[path = "studio_menu.rs"]
mod menu;
#[path = "studio_pages.rs"]
mod pages;
#[path = "studio_ide_panels.rs"]
mod panels;
#[path = "studio_panes.rs"]
mod panes;
#[path = "studio_plans.rs"]
mod plans;
#[path = "studio_review.rs"]
mod review;
#[path = "studio_search.rs"]
mod search;
#[path = "studio_session.rs"]
mod session;
#[path = "studio_shell.rs"]
mod shell;
#[path = "studio_ide_status.rs"]
mod status;
#[path = "studio_styles.rs"]
mod styles;
#[path = "studio_ide_tabs.rs"]
mod tabs;
#[path = "studio_terminal.rs"]
mod terminal;
#[path = "studio_terminal_keys.rs"]
mod terminal_keys;
#[path = "studio_terminal_surface.rs"]
mod terminal_surface;
#[path = "studio_toolbar.rs"]
mod toolbar;
#[path = "studio_update.rs"]
mod update;
#[path = "studio_workspace.rs"]
mod workspace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Home,
    Setup,
    Projects,
    Catalog,
    Integrations,
    Diagnostics,
}

impl Page {
    fn key(self) -> &'static str {
        match self {
            Page::Home => "home",
            Page::Setup => "setup",
            Page::Projects => "projects",
            Page::Catalog => "catalog",
            Page::Integrations => "integrations",
            Page::Diagnostics => "diagnostics",
        }
    }

    fn from_key(key: &str) -> Self {
        match key {
            "setup" => Page::Setup,
            "projects" => Page::Projects,
            "catalog" => Page::Catalog,
            "integrations" => Page::Integrations,
            "diagnostics" => Page::Diagnostics,
            _ => Page::Home,
        }
    }
}
use std::path::{Path, PathBuf};

struct Dashboard {
    panes: panes::State,
    chat: chat::State,
    local: local::State,
    update: update::State,
    terminal: terminal::State,
    agents: agents::State,
    editor: editor::State,
    shell: shell::State,
    catalog: catalog::State,
    session: session::State,
    menu: menu::State,
    search: search::State,
    git: git::State,
    page: Page,
    root: PathBuf,
    overview: Result<ProjectOverview, String>,
    init_preview: Option<ProjectInitPlan>,
    sync_preview: Option<crate::features::project_sync::ProjectSyncPlan>,
    rollback_preview: Option<crate::features::project_rollback::ProjectRollbackPlan>,
    trust_preview: Option<crate::features::project_trust::ProjectTrustPlan>,
    mcp_preview: Option<crate::features::mcp_setup::McpIntegrationPlan>,
    projects_catalog: Option<crate::features::project_catalog::ProjectCatalog>,
    doctor_report: Option<crate::features::doctor::DoctorReport>,
    init_busy: bool,
    sync_busy: bool,
    mcp_busy: bool,
    /// Background jobs in flight; the status bar shows a working indicator.
    pending: u32,
    overview_loading: bool,
    message: String,
}

#[derive(Clone, Debug)]
enum Message {
    Terminal(terminal::Event),
    Agents(agents::Event),
    Chat(chat::Event),
    Local(local::Event),
    Update(update::Event),
    Panes(panes::Event),
    /// A reader thread has data (terminal output, an agent's answer).
    Wake,
    Editor(editor::Event),
    Shell(shell::Event),
    Catalog(catalog::Event),
    Session(session::Event),
    Menu(menu::Event),
    Search(search::Event),
    Git(git::Event),
    /// Runs several messages in order (open a section, then start its command).
    Sequence(Vec<Message>),
    Done(jobs::Done),
    Navigate(Page),
    Dismiss,
    Refresh,
    PreviewInit {
        minimal: bool,
    },
    ApplyInit,
    PreviewSync,
    ApplySync,
    PreviewRollback,
    ApplyRollback,
    PreviewTrust {
        desired: bool,
    },
    ApplyTrust,
    PreviewMcp,
    ApplyMcp,
    Projects,
    InitApplied(Result<crate::features::project_setup::ProjectInitApplication, String>),
    SyncApplied(Result<crate::features::project_sync::ProjectSyncApplication, String>),
    Doctor,
    McpApplied(Result<crate::features::mcp_setup::McpIntegrationResult, String>),
}

/// Receives the wake-ups of the reader threads (see `integrations::wake`).
static WAKE_RECEIVER: std::sync::Mutex<
    Option<iced::futures::channel::mpsc::UnboundedReceiver<()>>,
> = std::sync::Mutex::new(None);
/// A wake-up is already on its way: more notifications until it is handled add nothing.
static WAKE_PENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn install_wake() {
    let (sender, receiver) = iced::futures::channel::mpsc::unbounded();
    if let Ok(mut slot) = WAKE_RECEIVER.lock() {
        *slot = Some(receiver);
    }
    crate::integrations::wake::set_hook(move || {
        if !WAKE_PENDING.swap(true, std::sync::atomic::Ordering::AcqRel) {
            let _ = sender.unbounded_send(());
        }
    });
}

fn wake_stream() -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::{SinkExt, StreamExt};
    iced::stream::channel(1, async |mut out| {
        let receiver = WAKE_RECEIVER.lock().ok().and_then(|mut slot| slot.take());
        let Some(mut receiver) = receiver else {
            return std::future::pending().await;
        };
        while receiver.next().await.is_some() {
            WAKE_PENDING.store(false, std::sync::atomic::Ordering::Release);
            if out.send(Message::Wake).await.is_err() {
                break;
            }
            // At most ~30 redraws a second however fast the data arrives.
            let _ =
                jobs::offload(|| std::thread::sleep(std::time::Duration::from_millis(33))).await;
        }
    })
}

pub fn run(path: PathBuf) -> Result<(), String> {
    run_with_file(path, None)
}

pub fn run_with_file(path: PathBuf, file: Option<PathBuf>) -> Result<(), String> {
    let path = std::path::absolute(&path).unwrap_or(path);
    let loaded = crate::features::studio_preferences::load();
    let window_size = loaded
        .preferences
        .layout
        .window
        .map_or(iced::Size::new(1240.0, 640.0), |[width, height]| {
            iced::Size::new(width, height)
        });
    install_wake();
    iced::application(
        move || {
            let (session, warning) = session::boot(&path, loaded.clone());
            let mut state = Dashboard {
                terminal: terminal::State::default(),
                agents: agents::State {
                    auto_chat: std::env::var("CHXCHX_STUDIO_CHAT")
                        .ok()
                        .filter(|id| !id.is_empty()),
                    ..agents::State::default()
                },
                chat: chat::State::default(),
                local: local::State::default(),
                update: update::State::default(),
                panes: panes::State::default(),
                editor: editor::State::new(),
                shell: shell::State::default(),
                catalog: catalog::State::default(),
                session,
                menu: menu::State::default(),
                search: search::State::default(),
                git: git::State::default(),
                page: Page::Home,
                root: path.clone(),
                overview: Err(String::new()),
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
                message: warning.unwrap_or_default(),
            };
            session::apply(&mut state);
            let tasks = [
                editor::list(path.clone(), PathBuf::new()),
                session::restore(&state),
                plans::refresh(&mut state),
                agents::refresh(&mut state),
                git::refresh(&mut state),
                file.as_ref()
                    .map(|file| editor::load(path.clone(), file.clone()))
                    .unwrap_or_else(Task::none),
                if state.page == Page::Catalog {
                    catalog::update(&mut state, catalog::Event::Load)
                } else {
                    Task::none()
                },
            ];
            (state, Task::batch(tasks))
        },
        actions::update,
        ide::view,
    )
    .title(|state: &Dashboard| format!("{} — ChxChx Studio", chrome::project_name(state)))
    .theme(|_: &Dashboard| design::theme())
    .default_font(design::UI)
    .window(iced::window::Settings {
        size: window_size,
        position: iced::window::Position::Centered,
        min_size: Some(iced::Size::new(880.0, 520.0)),
        icon: brand::window_icon(),
        ..Default::default()
    })
    .exit_on_close_request(false)
    .subscription(|state: &Dashboard| {
        let mut subscriptions = vec![
            iced::window::close_requests().map(|_| Message::Editor(editor::Event::Exit)),
            shell::subscription(state),
            session::subscription(state),
        ];
        subscriptions.push(iced::Subscription::run(wake_stream));
        // Polling Git rebuilds and redraws the window: only while it is being looked at, and
        // faster only when the Git view itself is on screen.
        if state.git.is_repository() && state.git.unavailable.is_none() && state.shell.focused {
            let watching = state.editor.explorer && state.shell.side == shell::Side::Git;
            subscriptions.push(
                iced::time::every(std::time::Duration::from_secs(if watching {
                    8
                } else {
                    30
                }))
                .map(|_| Message::Git(git::Event::Poll)),
            );
        }
        if state
            .chat
            .chats
            .iter()
            .any(|chat| state.panes.shows_chat(&chat.agent.id))
        {
            subscriptions.push(iced::event::listen_with(|event, _, _| match event {
                iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                    Some(Message::Chat(chat::Event::Dropped(path)))
                }
                _ => None,
            }));
        }
        if state.local.busy() {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_secs(2))
                    .map(|_| Message::Local(local::Event::Tick)),
            );
        }
        if state.chat.busy() {
            // Safety net only: the agent's output wakes the window by itself.
            subscriptions.push(
                iced::time::every(std::time::Duration::from_secs(2))
                    .map(|_| Message::Chat(chat::Event::Tick)),
            );
        }
        iced::Subscription::batch(subscriptions)
    })
    .run()
    .map_err(|error| format!("No se pudo abrir ChxChx Studio: {error}"))
}
