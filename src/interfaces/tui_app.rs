//! The state of the TUI: which page is open, what each page has loaded, the modal in front and
//! the work running in the background. Pages read this and render; use cases do the work.
use super::preview::{self, Apply, Preview};
use crate::features::{
    agents::{self, AgentCatalog},
    catalog_view::{CatalogView, inspect_catalog},
    doctor::{DoctorReport, diagnose_project},
    git::{GitCommit, GitStatus, Repository},
    project_catalog::{ProjectCatalog, inspect_registered_projects},
    project_overview::{ProjectOverview, inspect_project},
};
use ratatui::layout::Rect;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Home,
    Setup,
    Catalog,
    Agents,
    Git,
    Connect,
    Health,
    Projects,
}

impl Page {
    pub const ALL: [Page; 8] = [
        Page::Home,
        Page::Setup,
        Page::Catalog,
        Page::Agents,
        Page::Git,
        Page::Connect,
        Page::Health,
        Page::Projects,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Home => "Inicio",
            Page::Setup => "Preparar",
            Page::Catalog => "Catálogo",
            Page::Agents => "Agentes",
            Page::Git => "Git",
            Page::Connect => "Conectar",
            Page::Health => "Salud",
            Page::Projects => "Proyectos",
        }
    }

    pub fn index(self) -> usize {
        Page::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    pub fn step(self, delta: isize) -> Page {
        let len = Page::ALL.len() as isize;
        Page::ALL[(self.index() as isize + delta).rem_euclid(len) as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Info,
    Ok,
    Error,
}

pub struct Toast {
    pub text: String,
    pub kind: Kind,
    pub at: Instant,
}

/// The selected row of a list, kept inside the list as it grows and shrinks.
#[derive(Default, Clone, Copy, Debug)]
pub struct Sel(pub usize);

impl Sel {
    pub fn go(&mut self, delta: isize, len: usize) {
        self.0 = if len == 0 {
            0
        } else {
            (self.0 as isize + delta).clamp(0, len as isize - 1) as usize
        };
    }

    pub fn at(self, len: usize) -> usize {
        self.0.min(len.saturating_sub(1))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CatalogTab {
    #[default]
    Skills,
    Packs,
    Mods,
}

pub struct GitSnapshot {
    pub status: GitStatus,
    pub commits: Vec<GitCommit>,
}

/// What a background thread reports back.
pub enum Done {
    Applied(Result<String, String>),
    Git(Result<Option<GitSnapshot>, String>),
}

pub struct App {
    pub path: PathBuf,
    pub overview: ProjectOverview,
    pub page: Page,
    pub toast: Option<Toast>,
    pub modal: Option<Preview>,
    pub help: bool,
    /// What is running in the background, for the status line.
    pub busy: Option<String>,
    pub tick: usize,
    pub quit: bool,
    pub agents: AgentCatalog,
    pub catalog: Option<Result<CatalogView, String>>,
    pub git: Option<Result<Option<GitSnapshot>, String>>,
    pub doctor: Option<Result<DoctorReport, String>>,
    pub projects: Option<Result<ProjectCatalog, String>>,
    pub steps: Sel,
    pub setup: Sel,
    pub catalog_tab: CatalogTab,
    pub catalog_rows: Sel,
    pub agent_rows: Sel,
    pub git_rows: Sel,
    pub git_history: bool,
    pub connect: Sel,
    pub health_scroll: u16,
    pub project_rows: Sel,
    /// Where each tab was drawn, so a click can find it.
    pub tab_hits: Vec<(Rect, Page)>,
    /// An agent the person asked to open; the loop leaves the TUI to run it.
    pub launch: Option<usize>,
    sender: Sender<Done>,
    receiver: Receiver<Done>,
}

impl App {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let overview = inspect_project(&path)?;
        let (sender, receiver) = channel();
        Ok(Self {
            agents: agents::list(&PathBuf::from(&overview.project.root)),
            overview,
            path,
            page: Page::Home,
            toast: None,
            modal: None,
            help: false,
            busy: None,
            tick: 0,
            quit: false,
            catalog: None,
            git: None,
            doctor: None,
            projects: None,
            steps: Sel::default(),
            setup: Sel::default(),
            catalog_tab: CatalogTab::default(),
            catalog_rows: Sel::default(),
            agent_rows: Sel::default(),
            git_rows: Sel::default(),
            git_history: false,
            connect: Sel::default(),
            health_scroll: 0,
            project_rows: Sel::default(),
            tab_hits: Vec::new(),
            launch: None,
            sender,
            receiver,
        })
    }

    pub fn trusted(&self) -> bool {
        self.overview.workspace.trusted
    }

    pub fn say(&mut self, kind: Kind, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            kind,
            at: Instant::now(),
        });
    }

    /// Opens a page, loading what it shows the first time.
    pub fn go(&mut self, page: Page) {
        self.page = page;
        self.load(page, false);
    }

    /// Reads what `page` needs; `force` reads it again.
    pub fn load(&mut self, page: Page, force: bool) {
        let root = PathBuf::from(&self.overview.project.root);
        match page {
            Page::Catalog if force || self.catalog.is_none() => {
                self.catalog = Some(inspect_catalog(&root));
            }
            Page::Health | Page::Connect if force || self.doctor.is_none() => {
                self.doctor = Some(diagnose_project(&root));
            }
            Page::Projects if force || self.projects.is_none() => {
                self.projects = Some(inspect_registered_projects());
            }
            Page::Git if (force || self.git.is_none()) && self.busy.is_none() => {
                let (sender, trusted) = (self.sender.clone(), self.trusted());
                self.busy = Some("Leyendo Git".into());
                std::thread::spawn(move || {
                    let _ = sender.send(Done::Git(read_git(&root, trusted)));
                });
            }
            _ => {}
        }
    }

    /// Reads everything again after a change.
    pub fn reload_all(&mut self) {
        if let Ok(overview) = inspect_project(&self.path) {
            self.agents = agents::list(&PathBuf::from(&overview.project.root));
            self.overview = overview;
        }
        self.catalog = None;
        self.doctor = None;
        self.projects = None;
        self.git = None;
        self.load(self.page, true);
    }

    /// Applies a previewed change in the background.
    pub fn apply(&mut self, change: Apply) {
        let sender = self.sender.clone();
        self.busy = Some("Aplicando".into());
        std::thread::spawn(move || {
            let _ = sender.send(Done::Applied(preview::apply(change)));
        });
    }

    /// Takes in one finished piece of background work.
    pub fn complete(&mut self, done: Done) {
        self.busy = None;
        match done {
            Done::Applied(Ok(message)) => {
                self.modal = None;
                self.say(Kind::Ok, message);
                self.reload_all();
            }
            Done::Applied(Err(error)) => self.say(Kind::Error, error),
            Done::Git(result) => self.git = Some(result),
        }
    }

    /// Takes in whatever the background threads finished.
    pub fn poll(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        while let Ok(done) = self.receiver.try_recv() {
            self.complete(done);
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|t| t.kind != Kind::Error && t.at.elapsed() > Duration::from_secs(8))
        {
            self.toast = None;
        }
    }
}

fn read_git(root: &std::path::Path, trusted: bool) -> Result<Option<GitSnapshot>, String> {
    let Some(repository) = Repository::open(root, trusted).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    Ok(Some(GitSnapshot {
        status: repository.status().map_err(|e| e.to_string())?,
        commits: repository.log(40).map_err(|e| e.to_string())?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_wrap_around_in_both_directions() {
        assert_eq!(Page::Home.step(-1), Page::Projects);
        assert_eq!(Page::Projects.step(1), Page::Home);
        assert_eq!(Page::Setup.step(2), Page::Agents);
    }

    #[test]
    fn a_selection_stays_inside_its_list() {
        let mut sel = Sel::default();
        sel.go(-3, 4);
        assert_eq!(sel.0, 0);
        sel.go(10, 4);
        assert_eq!(sel.0, 3);
        sel.go(1, 0);
        assert_eq!(sel.at(0), 0);
        assert_eq!(
            Sel(9).at(3),
            2,
            "a list that shrank keeps the selection in range"
        );
    }
}
