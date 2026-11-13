use crate::features::project_overview::{ProjectOverview, inspect_project};
use crate::features::project_setup::ProjectInitPlan;
use iced::widget::{Space, button, container, row, text};
use iced::{Element, Fill, Task};

#[path = "studio_actions.rs"]
mod actions;
#[path = "studio_brand.rs"]
mod brand;
#[path = "studio_catalog.rs"]
mod catalog;
#[path = "studio_ide_chrome.rs"]
mod chrome;
#[path = "studio_design.rs"]
mod design;
#[path = "studio_ide_dock.rs"]
mod dock;
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
#[path = "studio_menu.rs"]
mod menu;
#[path = "studio_pages.rs"]
mod pages;
#[path = "studio_ide_panels.rs"]
mod panels;
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
#[path = "studio_toolbar.rs"]
mod toolbar;
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
    terminal: terminal::State,
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
    iced::application(
        move || {
            let (session, warning) = session::boot(&path, loaded.clone());
            let mut state = Dashboard {
                terminal: terminal::State::default(),
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
        if state.git.is_repository() && state.git.unavailable.is_none() {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_secs(8))
                    .map(|_| Message::Git(git::Event::Poll)),
            );
        }
        if state.terminal.session.is_some() {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_millis(100))
                    .map(|_| Message::Terminal(terminal::Event::Tick)),
            );
        }
        iced::Subscription::batch(subscriptions)
    })
    .run()
    .map_err(|error| format!("No se pudo abrir ChxChx Studio: {error}"))
}
