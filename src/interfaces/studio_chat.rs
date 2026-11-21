//! Visual chat with an agent: Studio owns its transcript and UI while the Python adapter still
//! runs provider turns and reports permissions, usage and context release over JSONL.
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
#[path = "studio_chat_art.rs"]
mod art;
#[path = "studio_chat_asks.rs"]
mod asks;
#[path = "studio_chat_cards.rs"]
mod cards;
#[path = "studio_chat_composer.rs"]
mod composer;
#[path = "studio_chat_models.rs"]
mod models;
#[path = "studio_chat_node.rs"]
pub(super) mod node;
#[path = "studio_chat_perms.rs"]
mod perms;
#[path = "studio_chat_prepare.rs"]
mod prepare;
#[path = "studio_chat_slash.rs"]
mod slash;
#[path = "studio_chat_usage.rs"]
mod usage;
use prepare::{open, prepared_for, sync, use_model};
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
    /// Assistant messages produced by the current turn, saved when the provider exits.
    pub turn_replies: Vec<String>,
    pub mode: String,
    pub network: bool,
    /// Rules approved for the rest of the chat.
    pub approved: Vec<String>,
    /// Rules approved for the next turn only.
    pub once: Vec<String>,
    pub options: Option<AgentOptions>,
    pub model_id: Option<String>,
    pub effort: Option<String>,
    /// Recap supplied to the adapter for a provider session Rust found missing.
    pub resume_recap: String,
    pub recovered_memories: String,
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
    /// What the person sent in this chat (oldest first), the one `↑` brought back and the text
    /// they had typed before browsing.
    pub history: Vec<String>,
    pub recall: Option<usize>,
    /// Messages shown as selectable text (the entry and its read-only editor).
    pub selecting: Vec<(usize, text_editor::Content)>,
    pub draft: String,
    /// Tool and reasoning cards the person expanded.
    pub expanded: Vec<usize>,
    /// Rendered Markdown of each entry (keyed by the text length it was built from).
    pub rendered: Vec<(usize, iced::widget::markdown::Content)>,
    /// Options picked on question cards: (entry, question) -> labels.
    pub picks: Vec<(usize, usize, String)>,
    /// Why the chat cannot talk to the agent (no CLI, no trust…).
    pub problem: Option<String>,
    /// Skills and commands the project gives this agent (`/name`).
    pub commands: Vec<crate::integrations::slash_commands::Found>,
}

#[derive(Default)]
pub(super) struct State {
    pub chats: Vec<Chat>,
    pub active: usize,
}

impl State {
    pub fn active_chat(&self) -> Option<&Chat> {
        self.chats.get(self.active)
    }

    pub fn has_chat(&self, id: &str) -> bool {
        self.chats.iter().any(|chat| chat.agent.id == id)
    }

    /// A turn is being followed (here or on the node): the heartbeat that wakes the chat runs.
    pub fn following(&self) -> bool {
        self.chats.iter().any(|chat| chat.turn.is_some())
    }
}

type Prepared = (
    SavedChat,
    Result<AgentOptions, String>,
    Vec<crate::integrations::slash_commands::Found>,
    (String, Result<usize, String>),
);

#[derive(Clone, Debug)]
pub(super) enum Event {
    Open(Agent),
    /// Opens the agent's chat with this model already chosen (a local model).
    OpenWith(Agent, String),
    Prepared(String, PreparedBox),
    /// An event of the chat shown in the pane of this agent.
    For(String, Box<Event>),
    Edit(text_editor::Action),
    Send,
    Stop,
    Tick,
    /// Looks for turns the node is running that no chat follows yet.
    Adopt,
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
    ProviderMemoriesPrepared(String, String, Result<usize, String>),
    /// A `/command` picked in the menu.
    Slash(String),
    /// Enter while the `/` menu is open.
    SlashEnter,
    ModelQuery(String),
    /// Sends the last message again.
    Retry,
    /// Forgets an approved rule: (rule, remembered for the chat).
    Revoke(String, bool),
    /// Expands or collapses a card.
    Toggle(usize),
    /// Copies the text of an entry (the view does not clone long answers every frame).
    CopyEntry(usize),
    /// Copies the last answer of the agent.
    CopyLast,
    /// Switches a message between rendered and selectable text.
    Select(usize),
    /// A mouse or key action on a selectable message (never edits it).
    Selected(usize, text_editor::Action),
    /// `↑` / `↓` in the message box: earlier messages of this chat.
    HistoryPrev,
    HistoryNext,
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
        Event::OpenWith(agent, model) => {
            let id = agent.id.clone();
            let task = open(state, agent);
            with_agent(state, &id, |chat| use_model(chat, &model));
            return task;
        }
        Event::Prepared(id, prepared) => {
            prepared_for(state, &id, &prepared.0);
            // A turn the node is already running for this chat is picked up as soon as it opens.
            return node::adopt(state);
        }
        Event::ProviderMemoriesPrepared(id, context, result) => {
            with_agent(state, &id, |chat| {
                chat.preparing = false;
                chat.recovered_memories = context;
            });
            if let Err(error) = result {
                state.message = format!("No pude recuperar memorias del historial local: {error}");
            }
        }
        Event::For(id, inner) => {
            if let Some(index) = state.chat.chats.iter().position(|c| c.agent.id == id) {
                state.chat.active = index;
            }
            return update(state, *inner);
        }
        Event::Tick => tick(state),
        Event::Adopt => return node::adopt(state),
        Event::Send => return submit(state),
        Event::Slash(name) => with_chat(state, |chat| slash::complete(chat, &name)),
        Event::SlashEnter => {
            if !slash::enter(state) {
                return submit(state);
            }
        }
        Event::Stop => with_chat(state, |chat| {
            if let Some(turn) = chat.turn.as_mut() {
                turn.cancel();
            }
        }),
        Event::Edit(action) => with_chat(state, |chat| {
            if matches!(action, text_editor::Action::Edit(_)) {
                chat.recall = None;
            }
            chat.composer.perform(action)
        }),
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
        Event::CopyEntry(index) => {
            let text = state
                .chat
                .active_chat()
                .and_then(|c| c.model.entries.get(index))
                .map(|e| e.text.clone());
            if let Some(text) = text {
                state.message = "Copiado al portapapeles.".to_owned();
                return iced::clipboard::write(text);
            }
        }
        Event::CopyLast => {
            let text = state.chat.active_chat().and_then(|c| {
                c.model
                    .entries
                    .iter()
                    .rev()
                    .find(|e| e.kind == Kind::Assistant && !e.text.trim().is_empty())
                    .map(|e| e.text.clone())
            });
            match text {
                Some(text) => {
                    state.message = "Última respuesta copiada.".to_owned();
                    return iced::clipboard::write(text);
                }
                None => state.message = "Todavía no hay una respuesta que copiar.".to_owned(),
            }
        }
        Event::Select(index) => with_chat(state, |chat| {
            if let Some(at) = chat.selecting.iter().position(|(entry, _)| *entry == index) {
                chat.selecting.remove(at);
            } else if let Some(entry) = chat.model.entries.get(index) {
                let content = text_editor::Content::with_text(&entry.text);
                chat.selecting.push((index, content));
            }
        }),
        Event::Selected(index, action) => with_chat(state, |chat| {
            // Selecting and copying are allowed; typing into the transcript is not.
            if !action.is_edit()
                && let Some((_, content)) = chat.selecting.iter_mut().find(|(e, _)| *e == index)
            {
                content.perform(action);
            }
        }),
        Event::HistoryPrev => with_chat(state, |chat| turns::history_step(chat, true)),
        Event::HistoryNext => with_chat(state, |chat| turns::history_step(chat, false)),
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
        Event::NewChat => return new_chat(state),
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
        if let Some(turn) = chat.turn.as_mut() {
            for event in turn.poll() {
                if event.get("type").and_then(Value::as_str) == Some("message")
                    && event.get("role").and_then(Value::as_str) == Some("assistant")
                    && let Some(text) = event.get("text").and_then(Value::as_str)
                {
                    chat.turn_replies.push(text.to_owned());
                }
                chat.model.apply(&event);
            }
        }
        let finished = chat.turn.as_ref().is_some_and(|turn| turn.exit.is_some());
        if finished {
            let mut turn = chat.turn.take().expect("finished turn exists");
            let code = turn.exit.unwrap_or(0);
            let stderr = turn.stderr_tail();
            let cancelled = turn.was_cancelled();
            if turn.is_handoff() {
                let session = turn.handoff_target_session().unwrap_or_default().to_owned();
                let text = turn.take_handoff_text();
                if code == 0 && !cancelled {
                    match crate::features::chat_handoff::save(
                        &state.root,
                        turns::state_provider(chat.agent.provider),
                        &session,
                        &text,
                    ) {
                        Ok(()) => {
                            let _ = chat.model.apply(&json!({
                                "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION,
                                "type": "notice", "message": "Contexto liberado automáticamente: la conversación sigue con todo lo anterior.",
                            }));
                        }
                        Err(error) => {
                            let _ = chat.model.apply(&json!({
                                "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION,
                                "type": "notice", "message": format!("No pude guardar el traspaso de contexto: {error}"),
                            }));
                        }
                    }
                } else {
                    let detail = stderr.trim();
                    let message = if detail.is_empty() {
                        "No pude liberar el contexto; sigo con la conversación actual.".to_owned()
                    } else {
                        format!(
                            "No pude liberar el contexto; sigo con la conversación actual. {detail}"
                        )
                    };
                    chat.model.apply(&json!({
                        "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION,
                        "type": "notice", "message": message,
                    }));
                }
                sync(chat);
                continue;
            }
            let release_context = code == 0
                && !cancelled
                && turn.needs_context_release()
                && !chat.model.session_id.is_empty();
            let consumed_handoff = turn.take_consumed_handoff_session();
            let remote = turn.is_remote();
            drop(turn);
            let replies = std::mem::take(&mut chat.turn_replies);
            // The node that ran the turn also saved the conversation, the explicit memory and the
            // journal: saving them here as well would write every answer twice.
            let saved_state = if remote {
                Ok(())
            } else {
                crate::features::chat_state::end_turn(
                    &state.root,
                    &chat.agent.id,
                    turns::state_provider(chat.agent.provider),
                    Some(&chat.model.session_id),
                    replies.iter().map(String::as_str),
                )
            };
            if let Err(error) = saved_state {
                state.message = format!("No pude guardar el historial del chat: {error}");
            } else if !remote {
                if let Err(error) =
                    crate::features::explicit_memory::capture_studio_memories(&state.root)
                {
                    state.message =
                        format!("No pude guardar la memoria explícita del proyecto: {error}");
                }
                if let Err(error) =
                    crate::features::chat_journal::record_latest_chat(&state.root, &chat.agent.id)
                {
                    state.message = format!("No pude guardar el resumen de la sesión: {error}");
                }
            }
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
            if code != 0 && !cancelled && !chat.model.entries.iter().any(|e| e.kind == Kind::Error)
            {
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
            if code == 0
                && !cancelled
                && let Some(previous_session) = consumed_handoff
                && chat.model.session_id != previous_session
                && let Err(error) = crate::features::chat_handoff::consume(
                    &state.root,
                    turns::state_provider(chat.agent.provider),
                    &previous_session,
                )
            {
                state.message = format!("No pude retirar el traspaso ya recibido: {error}");
            }
            if release_context {
                let spec = handoff_turn_spec(chat, &state.root);
                match Turn::launch_handoff(&spec) {
                    Ok(handoff) => chat.turn = Some(handoff),
                    Err(error) => {
                        let _ = chat.model.apply(&json!({
                            "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION,
                            "type": "notice", "message": format!("No pude liberar el contexto; sigo con la conversación actual. {error}"),
                        }));
                    }
                }
            }
        }
        sync(chat);
    }
}

fn handoff_turn_spec(chat: &Chat, root: &std::path::Path) -> TurnSpec {
    TurnSpec {
        agent: chat.agent.id.clone(),
        provider: turns::state_provider(chat.agent.provider).to_owned(),
        project: root.to_path_buf(),
        session: Some(chat.model.session_id.clone()),
        mode: "read-only".to_owned(),
        model: chat.model_id.clone(),
        effort: chat.effort.clone(),
        agent_command: chat.agent.command.clone(),
        executable: chat.agent.executable.clone(),
        agent_cwd: chat.agent.cwd.clone(),
        // A summary only follows a turn Studio already ran in this trusted project.
        trusted: true,
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "studio_chat_tests.rs"]
mod tests;
