//! Visual chat with an agent: the legacy product's chat (permissions, access levels, tokens,
//! context release) over the `chxchx-tech agent chat` protocol. The Python CLI stays the
//! authority; this module only drives it and shows what it reports.
use super::*;
use crate::features::agents::Agent;
use crate::features::chat_model::{self as model, Kind};
use crate::integrations::chat_bridge::{self as bridge, AgentOptions, SavedChat, Turn, TurnSpec};
use iced::widget::text_editor;
use serde_json::{Value, json};
use std::path::PathBuf;

#[path = "studio_chat_images.rs"]
mod images;
use images::Image;
#[path = "studio_chat_asks.rs"]
mod asks;
#[path = "studio_chat_cards.rs"]
mod cards;
#[path = "studio_chat_composer.rs"]
mod composer;
#[path = "studio_chat_models.rs"]
mod models;
#[path = "studio_chat_perms.rs"]
mod perms;
#[path = "studio_chat_prepare.rs"]
mod prepare;
#[path = "studio_chat_usage.rs"]
mod usage;
use prepare::{open, prepared_for, sync};
#[path = "studio_chat_turns.rs"]
mod turns;
use turns::{decide, new_chat, pick, reply, submit};
#[path = "studio_chat_view.rs"]
mod view_impl;
pub(in crate::interfaces::studio) use view_impl::view;

thread_local! {
    /// The agent whose pane is being drawn: its buttons act on that chat, whichever pane has the focus.
    static CURRENT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Builds messages for the chat being drawn *now*. Widgets that make their message later
/// (a pick list, a key binding) must take this while the view is built: when they run, the
/// last chat drawn is another one, and the change would land in the wrong chat.
pub(super) fn sender() -> impl Fn(Event) -> Message + Clone + 'static {
    let id = CURRENT.with(|id| id.borrow().clone());
    move |event| Message::Chat(Event::For(id.clone(), Box::new(event)))
}

/// The message for an event of the chat being drawn.
pub(super) fn send(event: Event) -> Message {
    CURRENT.with(|id| Message::Chat(Event::For(id.borrow().clone(), Box::new(event))))
}

pub(super) const ACCESS: [(&str, &str); 4] = [
    ("read-only", "Solo lectura"),
    ("workspace-write", "Editar archivos"),
    ("workspace-commands", "Editar y ejecutar"),
    ("full-access", "Acceso total"),
];

/// Messages drawn (and kept in full) at once: every frame of a scroll lays them all out, and
/// older ones only cost memory.
pub(super) const VISIBLE_ENTRIES: usize = 60;

pub(super) const RETRY: &str =
    "Ya tienes el permiso que faltaba. Vuelve a intentar lo que quedó bloqueado.";

pub(super) struct Chat {
    pub agent: Agent,
    pub model: model::Model,
    pub composer: text_editor::Content,
    pub turn: Option<Turn>,
    pub mode: String,
    pub network: bool,
    /// Rules approved for the rest of the chat.
    pub approved: Vec<String>,
    /// Rules approved for the next turn only.
    pub once: Vec<String>,
    pub options: Option<AgentOptions>,
    pub model_id: Option<String>,
    pub effort: Option<String>,
    pub preparing: bool,
    /// Images waiting in the composer.
    pub images: Vec<Image>,
    /// Images each sent message carried: (entry, images).
    pub sent: Vec<(usize, Vec<Image>)>,
    /// The permissions panel is open.
    pub perms_open: bool,
    /// The model picker is open, and what the person typed to filter it.
    pub models_open: bool,
    pub model_query: String,
    /// When the running turn started, and how long each answered turn took: (entry, seconds).
    pub turn_started: Option<std::time::Instant>,
    pub durations: Vec<(usize, u64)>,
    /// Tool and reasoning cards the person expanded.
    pub expanded: Vec<usize>,
    /// Rendered Markdown of each entry (keyed by the text length it was built from).
    pub rendered: Vec<(usize, iced::widget::markdown::Content)>,
    /// Options picked on question cards: (entry, question) -> labels.
    pub picks: Vec<(usize, usize, String)>,
    /// Why the chat cannot talk to the agent (no CLI, no trust…).
    pub problem: Option<String>,
}

#[derive(Default)]
pub(super) struct State {
    pub chats: Vec<Chat>,
    pub active: usize,
    cli: Option<PathBuf>,
}

impl State {
    pub fn active_chat(&self) -> Option<&Chat> {
        self.chats.get(self.active)
    }

    pub fn has_chat(&self, id: &str) -> bool {
        self.chats.iter().any(|chat| chat.agent.id == id)
    }

    pub fn busy(&self) -> bool {
        self.chats.iter().any(|chat| chat.turn.is_some())
    }
}

type Prepared = (
    Option<PathBuf>,
    Result<SavedChat, String>,
    Result<AgentOptions, String>,
);

#[derive(Clone, Debug)]
pub(super) enum Event {
    Open(Agent),
    Prepared(String, PreparedBox),
    /// An event of the chat shown in the pane of this agent.
    For(String, Box<Event>),
    Edit(text_editor::Action),
    Send,
    Stop,
    Tick,
    /// Answer the permission card at this entry: a choice, or `None` to deny.
    Decide(usize, Option<usize>),
    /// Picks (or unpicks) an option of a question card: entry, question, label.
    Pick(usize, usize, String),
    /// Sends the picked options of the question card at this entry.
    Reply(usize),
    /// Links of the Markdown are shown, not followed.
    Link,
    /// The "Adjuntar" button.
    Attach,
    Picked(String, Vec<PathBuf>),
    Dropped(PathBuf),
    /// Images prepared for the agent with this id.
    Loaded(
        String,
        Result<Vec<crate::integrations::chat_images::Prepared>, String>,
    ),
    RemoveImage(usize),
    Paste,
    PasteText(String),
    Pasted(String, Option<String>),
    /// Opens or closes the permissions panel.
    Permissions,
    /// Opens or closes the model picker.
    ModelMenu,
    ModelQuery(String),
    /// Sends the last message again.
    Retry,
    /// Forgets an approved rule: (rule, remembered for the chat).
    Revoke(String, bool),
    /// Expands or collapses a card.
    Toggle(usize),
    Copy(String),
    /// Puts a starter text in the composer.
    Starter(String),
    Access(String),
    Network(bool),
    Model(String),
    Effort(String),
    NewChat,
}

/// `Prepared` made cloneable for messages.
#[derive(Clone, Debug)]
pub(super) struct PreparedBox(pub std::sync::Arc<Prepared>);

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Open(agent) => return open(state, agent),
        Event::Prepared(id, prepared) => prepared_for(state, &id, &prepared.0),
        Event::For(id, inner) => {
            if let Some(index) = state.chat.chats.iter().position(|c| c.agent.id == id) {
                state.chat.active = index;
            }
            return update(state, *inner);
        }
        Event::Tick => tick(state),
        Event::Send => submit(state),
        Event::Stop => with_chat(state, |chat| {
            if let Some(turn) = chat.turn.as_mut() {
                turn.cancel();
            }
        }),
        Event::Edit(action) => with_chat(state, |chat| chat.composer.perform(action)),
        Event::Decide(index, choice) => decide(state, index, choice),
        Event::Pick(entry, question, label) => {
            with_chat(state, |chat| pick(chat, entry, question, label))
        }
        Event::Reply(index) => reply(state, index),
        Event::Link => {}
        Event::Attach => return images::attach(state),
        Event::Picked(id, paths) => return images::picked(id, paths),
        Event::Dropped(path) => return images::dropped(state, path),
        Event::Loaded(id, result) => images::loaded(state, &id, result),
        Event::RemoveImage(index) => with_chat(state, |chat| images::remove(chat, index)),
        Event::Paste => return images::paste(state),
        Event::PasteText(id) => return images::paste_text(id),
        Event::Pasted(id, text) => with_agent(state, &id, |chat| {
            if let Some(text) = text {
                chat.composer
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        std::sync::Arc::new(text),
                    )));
            }
        }),
        Event::Permissions => with_chat(state, |chat| {
            chat.perms_open = !chat.perms_open;
            chat.models_open = false;
        }),
        Event::ModelMenu => with_chat(state, |chat| {
            chat.models_open = !chat.models_open;
            chat.perms_open = false;
            chat.model_query.clear();
        }),
        Event::ModelQuery(query) => with_chat(state, |chat| chat.model_query = query),
        Event::Retry => turns::retry(state),
        Event::Revoke(rule, remembered) => with_chat(state, |chat| {
            let list = if remembered {
                &mut chat.approved
            } else {
                &mut chat.once
            };
            list.retain(|r| *r != rule);
        }),
        Event::Toggle(entry) => with_chat(state, |chat| {
            match chat.expanded.iter().position(|e| *e == entry) {
                Some(at) => {
                    chat.expanded.remove(at);
                }
                None => chat.expanded.push(entry),
            }
        }),
        Event::Copy(text) => return iced::clipboard::write(text),
        Event::Starter(text) => with_chat(state, |chat| {
            chat.composer = text_editor::Content::with_text(&text)
        }),
        Event::Access(mode) => with_chat(state, |chat| chat.mode = mode),
        Event::Network(on) => with_chat(state, |chat| chat.network = on),
        Event::Model(id) => with_chat(state, |chat| {
            chat.effort = chat
                .options
                .as_ref()
                .and_then(|o| o.models.iter().find(|m| m.id == id))
                .map(|m| m.default_effort.clone())
                .filter(|e| !e.is_empty());
            chat.model_id = Some(id);
            chat.models_open = false;
        }),
        Event::Effort(effort) => with_chat(state, |chat| chat.effort = Some(effort)),
        Event::NewChat => new_chat(state),
    }
    Task::none()
}

fn with_agent(state: &mut Dashboard, id: &str, apply: impl FnOnce(&mut Chat)) {
    if let Some(chat) = state.chat.chats.iter_mut().find(|c| c.agent.id == id) {
        apply(chat);
    }
}

fn with_chat(state: &mut Dashboard, apply: impl FnOnce(&mut Chat)) {
    let index = state.chat.active;
    if let Some(chat) = state.chat.chats.get_mut(index) {
        apply(chat);
    }
}

fn tick(state: &mut Dashboard) {
    for chat in &mut state.chat.chats {
        let Some(turn) = chat.turn.as_mut() else {
            continue;
        };
        for event in turn.poll() {
            chat.model.apply(&event);
        }
        let finished = turn.exit.is_some();
        if finished {
            let code = turn.exit.unwrap_or(0);
            let stderr = turn.stderr_tail();
            chat.turn = None;
            if let Some(started) = chat.turn_started.take()
                && let Some(last) = chat
                    .model
                    .entries
                    .iter()
                    .rposition(|e| e.kind == Kind::Assistant)
            {
                chat.durations.push((last, started.elapsed().as_secs()));
            }
            crate::integrations::process_memory::give_back_soon();
            images::cleanup(chat);
            chat.model.expire_live_requests();
            if code != 0 && !chat.model.entries.iter().any(|e| e.kind == Kind::Error) {
                let detail = stderr.trim();
                let message = if detail.is_empty() {
                    format!("El agente terminó con el código {code}.")
                } else {
                    format!("El agente terminó con el código {code}: {detail}")
                };
                chat.model.apply(&json!({
                    "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION,
                    "type": "error", "message": message,
                }));
            }
        }
        sync(chat);
    }
}

#[cfg(test)]
#[path = "studio_chat_tests.rs"]
mod tests;
