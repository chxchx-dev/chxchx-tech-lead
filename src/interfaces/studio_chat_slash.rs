//! `/commands` in the composer: the chat's own actions plus the skills and custom commands
//! the project gives the agent. Typing `/` opens a menu above the message box.
use super::super::*;
use super::{Chat, Event, send};
use crate::integrations::slash_commands::Found;
use iced::widget::{column, text_editor};
use serde_json::json;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Local {
    New,
    Model,
    Permissions,
    Terminal,
    Retry,
    Skills,
    Help,
}

/// The actions of the chat itself, which never reach the agent.
const LOCAL: [(&str, &str, Local); 7] = [
    ("nuevo", "Empieza un chat de cero", Local::New),
    ("modelo", "Elige modelo y esfuerzo", Local::Model),
    (
        "permisos",
        "Cambia qué puede hacer sin preguntar",
        Local::Permissions,
    ),
    (
        "terminal",
        "Abre este agente en una terminal",
        Local::Terminal,
    ),
    ("reintentar", "Repite tu último mensaje", Local::Retry),
    (
        "skills",
        "Lista las skills y comandos del proyecto",
        Local::Skills,
    ),
    ("ayuda", "Muestra los comandos disponibles", Local::Help),
];

const MENU_ROWS: usize = 8;

#[derive(Clone, Debug)]
pub(super) struct Item {
    pub name: String,
    pub hint: String,
    pub kind: &'static str,
}

/// What follows the `/` while the first word is still being typed.
pub(super) fn typing(chat: &Chat) -> Option<String> {
    let text = chat.composer.text();
    let word = text.strip_prefix('/')?;
    (!word.contains(char::is_whitespace)).then(|| word.to_lowercase())
}

pub(super) fn items(chat: &Chat, query: &str) -> Vec<Item> {
    let local = LOCAL.iter().map(|(name, hint, _)| Item {
        name: (*name).to_owned(),
        hint: (*hint).to_owned(),
        kind: "chat",
    });
    let found = chat.commands.iter().map(|f| Item {
        name: f.name.clone(),
        hint: f.description.clone(),
        kind: f.kind,
    });
    let mut all: Vec<Item> = local.chain(found).collect();
    all.retain(|item| item.name.to_lowercase().contains(query));
    // Names that start with what was typed come first.
    all.sort_by_key(|item| !item.name.to_lowercase().starts_with(query));
    all.truncate(MENU_ROWS);
    all
}

pub(super) fn menu(chat: &Chat) -> Option<Element<'_, Message>> {
    let query = typing(chat)?;
    let shown = items(chat, &query);
    if shown.is_empty() {
        return None;
    }
    let mut list = column![].spacing(1);
    for (index, item) in shown.into_iter().enumerate() {
        let tint = match item.kind {
            "chat" => design::ACCENT,
            _ => design::TEXT,
        };
        let message = send(Event::Slash(item.name.clone()));
        list = list.push(
            button(
                row![
                    text(format!("/{}", item.name)).size(12).color(tint),
                    text(item.hint)
                        .size(11)
                        .color(design::MUTED)
                        .wrapping(iced::widget::text::Wrapping::None),
                    Space::new().width(Fill),
                    text(item.kind).size(10).color(design::FAINT),
                ]
                .spacing(10)
                .align_y(iced::Center),
            )
            .width(Fill)
            .padding([5, 10])
            .style(styles::ghost(index == 0))
            .on_press(message),
        );
    }
    Some(
        container(list)
            .padding(4)
            .width(Fill)
            .style(styles::composer_card)
            .into(),
    )
}

/// Puts `/name ` in the composer, ready for its arguments.
pub(super) fn complete(chat: &mut Chat, name: &str) {
    chat.composer = text_editor::Content::with_text(&format!("/{name} "));
    chat.composer
        .perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}

/// Enter with the menu open: finish the word, or send it when it is already a command.
pub(super) fn enter(state: &mut Dashboard) -> bool {
    let Some(chat) = state.chat.active_chat() else {
        return false;
    };
    let Some(query) = typing(chat) else {
        return false;
    };
    let exact = items(chat, &query)
        .iter()
        .any(|i| i.name.to_lowercase() == query);
    if exact || query.is_empty() {
        return false;
    }
    let first = items(chat, &query).into_iter().next();
    if let (Some(item), Some(chat)) = (first, state.chat.chats.get_mut(state.chat.active)) {
        complete(chat, &item.name);
        return true;
    }
    false
}

fn note(chat: &mut Chat, text: String) {
    chat.model.apply(&json!({
        "schema": super::model::SCHEMA, "schema_version": super::model::SCHEMA_VERSION,
        "type": "notice", "message": text,
    }));
    super::sync(chat);
}

fn listing(commands: &[Found]) -> String {
    if commands.is_empty() {
        return "Este proyecto no tiene skills ni comandos para este agente.".to_owned();
    }
    let mut text = String::from("Skills y comandos del proyecto (escribe / para usarlos):");
    for found in commands {
        text.push_str(&format!("\n/{} · {}", found.name, found.description));
    }
    text
}

fn help() -> String {
    let mut text = String::from("Comandos del chat:");
    for (name, hint, _) in LOCAL {
        text.push_str(&format!("\n/{name} · {hint}"));
    }
    text.push_str("\nTambién puedes usar las skills y comandos del proyecto: /skills los lista.");
    text
}

/// What to tell the agent for a project command. Claude reads `/name` itself; the others get the
/// instruction spelled out.
pub(super) fn prompt_for(agent: &str, found: &Found, args: &str) -> String {
    let args = args.trim();
    if agent == "claude" {
        return format!("/{} {args}", found.name).trim_end().to_owned();
    }
    let place = match (found.kind, agent) {
        ("skill", _) => format!(".agents/skills/{}/SKILL.md", found.name),
        (_, "opencode") => format!(".opencode/command/{}.md", found.name),
        _ => found.name.clone(),
    };
    let base = format!(
        "Aplica {} «{}» de este proyecto (sus instrucciones están en {place}).",
        if found.kind == "skill" {
            "la skill"
        } else {
            "el comando"
        },
        found.name
    );
    if args.is_empty() {
        base
    } else {
        format!("{base}\n\n{args}")
    }
}

/// What a message that starts with `/` turns into.
pub(super) enum Outcome {
    /// Not a command the chat knows: send the text as it is.
    Plain,
    /// A chat action already ran.
    Done(Task<Message>),
    /// A project command: `(prompt for the agent, what the transcript shows)`.
    Send(String, String),
}

pub(super) fn resolve(state: &mut Dashboard, text: &str) -> Outcome {
    let Some(body) = text.strip_prefix('/') else {
        return Outcome::Plain;
    };
    let (name, args) = body.split_once(char::is_whitespace).unwrap_or((body, ""));
    let name = name.to_lowercase();
    if let Some((_, _, action)) = LOCAL.iter().find(|(n, _, _)| *n == name) {
        return Outcome::Done(run(state, *action));
    }
    let Some(chat) = state.chat.active_chat() else {
        return Outcome::Plain;
    };
    match chat.commands.iter().find(|f| f.name.to_lowercase() == name) {
        Some(found) => Outcome::Send(prompt_for(&chat.agent.id, found, args), text.to_owned()),
        None => Outcome::Plain,
    }
}

fn run(state: &mut Dashboard, action: Local) -> Task<Message> {
    let id = state.chat.active_chat().map(|c| c.agent.id.clone());
    let mut task = Task::none();
    match action {
        Local::New => super::new_chat(state),
        Local::Retry => super::turns::retry(state),
        Local::Terminal => {
            if let Some(id) = id {
                task = Task::done(Message::Agents(super::super::agents::Event::Terminal(id)));
            }
        }
        _ => {
            if let Some(chat) = state.chat.chats.get_mut(state.chat.active) {
                match action {
                    Local::Model => {
                        chat.models_open = true;
                        chat.perms_open = false;
                    }
                    Local::Permissions => {
                        chat.perms_open = true;
                        chat.models_open = false;
                    }
                    Local::Skills => {
                        let text = listing(&chat.commands);
                        note(chat, text);
                    }
                    _ => note(chat, help()),
                }
            }
        }
    }
    if action != Local::New
        && let Some(chat) = state.chat.chats.get_mut(state.chat.active)
    {
        chat.composer = text_editor::Content::new();
    }
    task
}
