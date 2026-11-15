//! Opening a chat: finding the CLI, the saved conversation and the agent's models.
use super::*;

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
        mode: "workspace-write".to_owned(),
        network: false,
        approved: Vec::new(),
        once: Vec::new(),
        options: None,
        model_id: None,
        effort: None,
        preparing: true,
        images: Vec::new(),
        sent: Vec::new(),
        perms_open: false,
        models_open: false,
        model_query: String::new(),
        turn_started: None,
        durations: Vec::new(),
        expanded: Vec::new(),
        rendered: Vec::new(),
        picks: Vec::new(),
        problem: None,
    });
    state.chat.active = state.chat.chats.len() - 1;
    let root = state.root.clone();
    let known = state.chat.cli.clone();
    let id = agent.id.clone();
    Task::perform(
        async move {
            jobs::offload(move || -> Prepared {
                let cli = known.or_else(|| bridge::find_cli(&root));
                match &cli {
                    Some(cli) => (
                        Some(cli.clone()),
                        bridge::restore_chat(cli, &root, &agent.id),
                        bridge::agent_options(cli, &root, &agent.id),
                    ),
                    None => (None, Err(bridge::missing_cli_hint()), Err(String::new())),
                }
            })
            .await
            .unwrap_or_else(|| {
                (
                    None,
                    Err("No se pudo preparar el chat".into()),
                    Err(String::new()),
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
    let (cli, saved, options) = prepared;
    if cli.is_some() {
        state.chat.cli = cli.clone();
    }
    let Some(chat) = state.chat.chats.iter_mut().find(|c| c.agent.id == id) else {
        return;
    };
    chat.preparing = false;
    match saved {
        Ok(saved) => {
            chat.model.session_id = saved.session_id.clone();
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
        }
        Err(error) if cli.is_none() => chat.problem = Some(error.clone()),
        Err(_) => {}
    }
    if let Ok(options) = options {
        if !options.available {
            chat.problem = Some(format!(
                "{} no está instalado. {}",
                chat.agent.label(),
                options.install_hint
            ));
        }
        chat.effort = Some(options.default_effort.clone()).filter(|e| !e.is_empty());
        chat.model_id = Some(options.default_model.clone()).filter(|m| !m.is_empty());
        chat.options = Some(options.clone());
    }
    sync(chat);
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
                iced::widget::markdown::Content::parse(&entry.text)
            } else {
                iced::widget::markdown::Content::new()
            }
        };
        match chat.rendered.get_mut(index) {
            Some(cached) if cached.0 == wanted => {}
            Some(cached) => *cached = (wanted, content()),
            None => chat.rendered.push((wanted, content())),
        }
    }
}
