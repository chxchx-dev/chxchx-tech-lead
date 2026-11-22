//! What the client knows, as global signals. Only tasks that run inside Dioxus write them: the
//! browser's own callbacks (socket, page events) go through the engine's channel instead.

use crate::model;
use crate::net;
use crate::route::Route;
use chxchx_protocol::{
    AccessMode, AgentSummary, ChatSummary, Device, Job, JobEvent, JobId, NodeStatus,
    ProjectSummary, Provider, Role,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const SESSION_KEY: &str = "chxchx.session";
const PREFS_KEY: &str = "chxchx.prefs";
/// Events kept per job in memory; a longer turn keeps its latest state but not every line.
pub const MAX_EVENTS: usize = 5_000;

/// This device's link to the node. The token is what the node gave it when it was paired.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub device: Device,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Conn {
    Connecting,
    Online,
    Offline(String),
}

/// A job, and the events heard about it so far (in order, without gaps).
#[derive(Clone, Debug, PartialEq)]
pub struct JobView {
    pub job: Job,
    pub events: Vec<JobEvent>,
    pub last_seq: u64,
}

impl JobView {
    pub fn terminal(&self) -> bool {
        self.job.state.is_terminal()
    }
}

/// What opening a project's chat loaded: its agents, and the conversations as they were then.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProjectData {
    pub agents: Option<Vec<AgentSummary>>,
    pub chats: Option<Vec<ChatSummary>>,
    /// When the chat was opened. Jobs from then on are shown live; earlier ones are in `chats`.
    pub opened_ms: u64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub text: String,
    pub bad: bool,
}

/// Choices that survive a reload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    pub access: AccessMode,
    pub network: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        // Reading is the safe start; editing is one tap away in the chat.
        Self {
            access: AccessMode::ReadOnly,
            network: false,
        }
    }
}

fn load_session() -> Option<Session> {
    serde_json::from_str(&net::load(SESSION_KEY)?).ok()
}

fn load_prefs() -> Prefs {
    net::load(PREFS_KEY)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub static SESSION: GlobalSignal<Option<Session>> = Signal::global(load_session);
pub static CONN: GlobalSignal<Conn> = Signal::global(|| Conn::Connecting);
pub static ROUTE: GlobalSignal<Route> = Signal::global(|| Route::from_hash(&net::hash()));
pub static NODE: GlobalSignal<Option<NodeStatus>> = Signal::global(|| None);
pub static PROJECTS: GlobalSignal<Option<Vec<ProjectSummary>>> = Signal::global(|| None);
pub static JOBS: GlobalSignal<BTreeMap<chxchx_protocol::JobId, JobView>> =
    Signal::global(BTreeMap::new);
pub static PROJECT_DATA: GlobalSignal<BTreeMap<String, ProjectData>> =
    Signal::global(BTreeMap::new);
pub static DEVICES: GlobalSignal<Option<Vec<Device>>> = Signal::global(|| None);
pub static TOAST: GlobalSignal<Option<Toast>> = Signal::global(|| None);
pub static PREFS: GlobalSignal<Prefs> = Signal::global(load_prefs);
/// The person's own name, line and picture (public, so it is there before the phone is linked).
/// Until the node answers it is the product's.
pub static IDENTITY: GlobalSignal<chxchx_protocol::IdentityView> =
    Signal::global(|| chxchx_protocol::Identity::default().view(None, None));
/// True while the phone is bringing in a change of the person's identity (a short "syncing" notice).
pub static SYNCING: GlobalSignal<bool> = Signal::global(|| false);
/// The time, refreshed every few seconds so "hace 5 min" stays true on a screen left open.
pub static CLOCK: GlobalSignal<u64> = Signal::global(net::now_ms);

/// Runs a data load that must finish even if the screen that asked for it goes away. Dioxus
/// cancels a task when its component unmounts, which would leave a half-loaded screen "loading"
/// forever; these loads only write global signals, so they belong to the app, not the component.
pub fn detach(task: impl std::future::Future<Output = ()> + 'static) {
    dioxus::core::spawn_forever(task);
}

pub fn token() -> Option<String> {
    SESSION.read().as_ref().map(|session| session.token.clone())
}

pub fn role() -> Role {
    SESSION
        .read()
        .as_ref()
        .map_or(Role::Reader, |session| session.device.role)
}

pub fn can_operate() -> bool {
    role() >= Role::Operator
}

pub fn save_session(session: &Session) {
    if let Ok(text) = serde_json::to_string(session) {
        net::save(SESSION_KEY, &text);
    }
}

pub fn forget_session() {
    net::forget(SESSION_KEY);
}

pub fn save_prefs(prefs: &Prefs) {
    if let Ok(text) = serde_json::to_string(prefs) {
        net::save(PREFS_KEY, &text);
    }
}

pub fn toast(text: impl Into<String>, bad: bool) {
    *TOAST.write() = Some(Toast {
        text: text.into(),
        bad,
    });
}

/// Jobs of a chat, newest last: those that are running, or began after the chat was opened.
pub fn live_jobs(project: &str, provider: Provider, opened_ms: u64) -> Vec<JobView> {
    JOBS.read()
        .values()
        .filter(|view| match &view.job.spec {
            chxchx_protocol::JobSpec::AgentTurn(turn) => {
                turn.project.as_str() == project
                    && turn.provider == provider
                    && (!view.terminal() || view.job.created_ms >= opened_ms)
            }
            _ => false,
        })
        .cloned()
        .collect()
}

/// The agent session to continue in a chat: the latest one a live job announced, or the one the
/// saved conversation had.
pub fn continuation(project: &str, provider: Provider) -> Option<String> {
    let live = JOBS
        .read()
        .values()
        .rev()
        .filter(|view| match &view.job.spec {
            chxchx_protocol::JobSpec::AgentTurn(turn) => {
                turn.project.as_str() == project && turn.provider == provider
            }
            _ => false,
        })
        .find_map(|view| model::session_of(&view.events));
    live.or_else(|| {
        PROJECT_DATA
            .read()
            .get(project)
            .and_then(|data| data.chats.as_ref())
            .and_then(|chats| snapshot_of(chats, provider))
            .map(|chat| chat.session_id)
            .filter(|session| !session.is_empty())
    })
}

/// The saved conversation of a provider among a project's.
pub fn snapshot_of(chats: &[ChatSummary], provider: Provider) -> Option<ChatSummary> {
    chats
        .iter()
        .find(|chat| chat.provider.eq_ignore_ascii_case(provider.as_str()))
        .cloned()
}

/// Cards waiting for the person, across every job still running.
pub fn waiting_for_me() -> Vec<(JobId, String)> {
    JOBS.read()
        .values()
        .filter(|view| !view.terminal())
        .filter_map(|view| {
            let items = model::fold(&view.events, false);
            (model::waiting(&items) > 0)
                .then(|| (view.job.id.clone(), model::describe(&view.job.spec)))
        })
        .collect()
}

/// The picture to show: the person's own when they set one, else the product's.
pub fn avatar_src(product_default: &str) -> String {
    match IDENTITY.read().avatar_rev {
        Some(revision) => format!("/identity/avatar.png?v={revision}"),
        None => product_default.to_owned(),
    }
}
