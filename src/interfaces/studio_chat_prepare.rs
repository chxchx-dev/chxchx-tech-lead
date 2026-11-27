//! Opening a chat: finding the CLI, the saved conversation and the agent's models.
use super::*;
use crate::integrations::slash_commands;

pub(super) fn open(state: &mut Dashboard, agent: Agent) -> Task<Message> {
    panes::show_chat(state, &agent.id);
    if let Some(index) = state.chat.chats.iter().position(|c| c.agent.id == agent.id) {
        state.chat.active = index;
        return Task::none();
    }
    state.chat.chats.push(Chat {
        agent: agent.clone(),
        model: model::Model::default(),
        composer: text_editor::Content::new(),
        turn: None,
        turn_replies: Vec::new(),
        mode: "workspace-write".to_owned(),
        network: false,
        approved: Vec::new(),
        once: Vec::new(),
        options: None,
        model_id: None,
        effort: None,
        plan: None,
        follow_up: None,
        spend: Vec::new(),
        turn_pick: None,
        usage_mark: (0, 0, None),
        resume_recap: String::new(),
        recovered_memories: String::new(),
        preparing: true,
        images: Vec::new(),
        sent: Vec::new(),
        perms_open: false,
        models_open: false,
        model_query: String::new(),
        turn_started: None,
        durations: Vec::new(),
        history: Vec::new(),
        recall: None,
        selecting: Vec::new(),
        draft: String::new(),
        expanded: Vec::new(),
        rendered: Vec::new(),
        picks: Vec::new(),
        problem: None,
        commands: Vec::new(),
    });
    state.chat.active = state.chat.chats.len() - 1;
    let root = state.root.clone();
    let id = agent.id.clone();
    let provider = super::turns::state_provider(agent.provider).to_owned();
    let executable = agent
        .executable
        .as_deref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    Task::perform(
        async move {
            jobs::offload(move || -> Prepared {
                // Looks for the agent-adapter service (Codex, OpenCode) before the first turn.
                bridge::prepare_native(&agent, &root);
                #[cfg(feature = "hive")]
                crate::features::hive::context::refresh(&root);
                let options = crate::integrations::agent_catalog::options(&agent, &root);
                let provider_memories =
                    crate::features::explicit_memory::prepare_provider_history(&root);
                (
                    bridge::restore_chat_for_provider(&root, &agent.id, &provider, &executable),
                    Ok(options),
                    slash_commands::discover(&root, &agent.id),
                    provider_memories,
                )
            })
            .await
            .unwrap_or_else(|| {
                (
                    bridge::SavedChat::default(),
                    Err("la preparación en segundo plano terminó sin resultado".to_owned()),
                    Vec::new(),
                    (
                        String::new(),
                        Err("la recuperación de memoria terminó sin resultado".to_owned()),
                    ),
                )
            })
        },
        move |prepared| {
            Message::Chat(Event::Prepared(
                id.clone(),
                PreparedBox(std::sync::Arc::new(prepared)),
            ))
        },
    )
}

pub(super) fn prepared_for(state: &mut Dashboard, id: &str, prepared: &Prepared) {
    let (saved, options, commands, (recovered_memories, memory_result)) = prepared;
    let Some(chat) = state.chat.chats.iter_mut().find(|c| c.agent.id == id) else {
        return;
    };
    chat.preparing = false;
    chat.commands = commands.clone();
    chat.recovered_memories = recovered_memories.clone();
    chat.model.session_id = saved.session_id.clone();
    chat.resume_recap = saved.resume_recap.clone();
    for (role, body) in &saved.messages {
        if role == "user" {
            chat.model.add_user(body.clone());
        } else {
            chat.model.apply(&json!({
                "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION,
                "type": "message", "role": "assistant", "text": body,
            }));
        }
    }
    if let Some(hint) = bridge::missing_runner_hint(&chat.agent) {
        chat.problem = Some(hint);
    }
    if let Err(error) = memory_result {
        state.message = format!("No pude recuperar memorias del historial local: {error}");
    }
    if let Ok(options) = options {
        if !options.available {
            chat.problem = Some(format!(
                "{} no está instalado. {}",
                chat.agent.label(),
                options.install_hint
            ));
        }
        // A model chosen before the chat was ready (a local one) wins over the agent's default.
        if chat.model_id.is_none() {
            chat.effort = Some(options.default_effort.clone()).filter(|e| !e.is_empty());
            chat.model_id = Some(options.default_model.clone()).filter(|m| !m.is_empty());
        }
        chat.options = Some(options.clone());
        ensure_listed(chat);
    }
    sync(chat);
}

/// Chooses `id` for the next messages of the chat.
pub(super) fn use_model(chat: &mut Chat, id: &str) {
    chat.model_id = Some(id.to_owned());
    chat.effort = None;
    ensure_listed(chat);
}

/// The model in use appears in the picker even when the agent's own list does not know it yet
/// (a local model connected a moment ago).
fn ensure_listed(chat: &mut Chat) {
    if let (Some(id), Some(options)) = (chat.model_id.clone(), chat.options.as_mut())
        && !options.models.iter().any(|m| m.id == id)
    {
        options.models.push(bridge::ModelOption {
            label: id.clone(),
            id,
            ..Default::default()
        });
    }
}

/// Keeps the rendered Markdown of every assistant message up to date.
pub(super) fn sync(chat: &mut Chat) {
    // Old entries are hidden in the view: their text is freed here and their Markdown is not kept.
    chat.model.compact(VISIBLE_ENTRIES);
    let hidden = chat.model.entries.len().saturating_sub(VISIBLE_ENTRIES);
    chat.rendered.truncate(chat.model.entries.len());
    for (index, entry) in chat.model.entries.iter().enumerate() {
        let shown = index >= hidden && entry.kind == Kind::Assistant;
        let wanted = if shown { entry.text.len() } else { 0 };
        let content = || {
            if shown {
                iced::widget::markdown::Content::parse(&crate::features::chat_links::linkify(
                    &entry.text,
                ))
            } else {
                iced::widget::markdown::Content::new()
            }
        };
        match chat.rendered.get_mut(index) {
            Some(cached) if cached.0 == wanted => {}
            // A streaming answer only grows: parse the new tail, not the whole text again.
            Some(cached)
                if shown
                    && cached.0 > 0
                    && cached.0 < wanted
                    && entry.text.is_char_boundary(cached.0)
                    && !touches_a_link(&entry.text, cached.0) =>
            {
                cached.1.push_str(&entry.text[cached.0..]);
                cached.0 = wanted;
            }
            Some(cached) => *cached = (wanted, content()),
            None => chat.rendered.push((wanted, content())),
        }
    }
}

/// Whether the part of a message after `from`, or the end of what was already parsed, holds a link:
/// links are made clickable over the whole text, so a link being written is parsed again from the start.
pub(super) fn touches_a_link(text: &str, from: usize) -> bool {
    let mut start = from.saturating_sub(300);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    text[start..].contains("http")
}
