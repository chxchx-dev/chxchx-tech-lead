//! What the person does in a chat: send, stop, answer a card, start over.
use super::*;

fn trusted(state: &Dashboard) -> bool {
    state.overview.as_ref().is_ok_and(|p| p.workspace.trusted)
}

pub(super) fn submit(state: &mut Dashboard) -> Task<Message> {
    let text = state
        .chat
        .active_chat()
        .map(|c| c.composer.text().trim().to_owned())
        .unwrap_or_default();
    let has_images = state
        .chat
        .active_chat()
        .is_some_and(|c| !c.images.is_empty());
    if text.is_empty() && !has_images {
        return Task::none();
    }
    let (prompt, display) = match slash::resolve(state, &text) {
        slash::Outcome::Done(task) => return task,
        slash::Outcome::Send(prompt, display) => (prompt, Some(display)),
        slash::Outcome::Plain => (text.clone(), None),
    };
    if start_turn_as(state, &prompt, display.as_deref(), true) {
        with_chat(state, |chat| {
            chat.composer = text_editor::Content::new();
            chat.recall = None;
            chat.draft.clear();
            if !text.is_empty() && chat.history.last() != Some(&text) {
                chat.history.push(text);
            }
        });
    }
    Task::none()
}

/// Starts a turn with `prompt`; false when it could not start (the reason is shown).
fn start_turn(state: &mut Dashboard, prompt: &str, show: bool) -> bool {
    start_turn_as(state, prompt, None, show)
}

/// Like `start_turn`, with the text the transcript shows when it differs from what the agent reads.
fn start_turn_as(state: &mut Dashboard, prompt: &str, display: Option<&str>, show: bool) -> bool {
    if !trusted(state) {
        state.message = "Concede confianza al proyecto para hablar con un agente.".into();
        return false;
    }
    let (root, cli) = (state.root.clone(), state.chat.cli.clone());
    let Some(chat) = state.chat.chats.get_mut(state.chat.active) else {
        return false;
    };
    if chat.turn.is_some() || chat.preparing || chat.problem.is_some() {
        return false;
    }
    let Some(cli) = cli else {
        state.message = bridge::missing_cli_hint();
        return false;
    };
    let mut allow = chat.approved.clone();
    allow.append(&mut chat.once);
    let attached = if show {
        std::mem::take(&mut chat.images)
    } else {
        Vec::new()
    };
    let spec = TurnSpec {
        agent: chat.agent.id.clone(),
        project: root,
        prompt: with_images(prompt, attached.len()),
        resume_recap: chat.resume_recap.clone(),
        images: attached.iter().map(|i| i.prepared.path.clone()).collect(),
        session: Some(chat.model.session_id.clone()).filter(|s| !s.is_empty()),
        mode: chat.mode.clone(),
        network: chat.network,
        allow,
        model: chat.model_id.clone(),
        effort: chat.effort.clone(),
    };
    let visible_prompt = shown(display.unwrap_or(prompt), attached.len());
    match Turn::start(&cli, &spec) {
        Ok(turn) => {
            if show {
                let _ = crate::features::chat_state::begin_turn(
                    &spec.project,
                    &spec.agent,
                    state_provider(chat.agent.provider),
                    spec.session.as_deref(),
                    &visible_prompt,
                );
                chat.model.add_user(visible_prompt);
                chat.resume_recap.clear();
                if !attached.is_empty() {
                    chat.sent.push((chat.model.entries.len() - 1, attached));
                }
            }
            chat.turn_replies.clear();
            chat.turn = Some(turn);
            chat.turn_started = Some(std::time::Instant::now());
            sync(chat);
            true
        }
        Err(error) => {
            chat.images = attached;
            state.message = error;
            false
        }
    }
}

pub(super) fn state_provider(provider: crate::features::agents::Provider) -> &'static str {
    match provider {
        crate::features::agents::Provider::Claude => "Claude",
        crate::features::agents::Provider::Codex => "Codex",
        crate::features::agents::Provider::OpenCode => "OpenCode",
        crate::features::agents::Provider::Other => "Agente",
    }
}

/// What the agent reads: the request, plus how to treat the pictures.
fn with_images(prompt: &str, count: usize) -> String {
    match (count, prompt.trim().is_empty()) {
        (0, _) => prompt.to_owned(),
        (_, true) => format!("{IMAGE_HINT} Resuelve lo que muestran."),
        (_, false) => format!("{prompt}\n\n{IMAGE_HINT}"),
    }
}

/// What the transcript shows for the person's message.
fn shown(prompt: &str, count: usize) -> String {
    if prompt.trim().is_empty() && count > 0 {
        "Imágenes adjuntas".to_owned()
    } else {
        prompt.to_owned()
    }
}

const IMAGE_HINT: &str = "(Adjunto imágenes de este proyecto: ve directo a lo que muestran y a lo que necesito, sin describirlas por completo.)";

pub(super) fn decide(state: &mut Dashboard, index: usize, choice: Option<usize>) {
    let Some(chat) = state.chat.chats.get_mut(state.chat.active) else {
        return;
    };
    let Some(entry) = chat.model.entries.get(index) else {
        return;
    };
    let picked = choice.and_then(|c| entry.choices.get(c)).cloned();
    let label = picked.as_ref().map(|c| c.label.clone()).unwrap_or_default();
    let request_id = entry.request_id.clone();
    if let Some(picked) = &picked {
        if picked.persist {
            chat.approved.extend(picked.allow.iter().cloned());
        } else if request_id.is_empty() {
            chat.once.extend(picked.allow.iter().cloned());
        }
        if request_id.is_empty() {
            chat.network |= picked.network;
            if let Some(mode) = &picked.mode {
                chat.mode = mode.clone();
            }
        }
    }
    if !request_id.is_empty() {
        let mut answer = json!({
            "type": "answer", "request_id": request_id,
            "behavior": if picked.is_some() { "allow" } else { "deny" },
        });
        if let Some(scope) = picked.as_ref().and_then(|p| p.scope.clone()) {
            answer["scope"] = Value::String(scope);
        }
        if let Some(turn) = chat.turn.as_mut()
            && let Err(error) = turn.answer(&answer)
        {
            state.message = error;
        }
    }
    chat.model.decide(index, &label);
    // A card from a turn that already ended is answered by trying again.
    if picked.is_some() && request_id.is_empty() {
        start_turn(state, RETRY, false);
    }
}

pub(super) fn pick(chat: &mut Chat, entry: usize, question: usize, label: String) {
    let multi = chat
        .model
        .entries
        .get(entry)
        .and_then(|e| e.questions.get(question))
        .is_some_and(|q| q.multi_select);
    let same = |p: &(usize, usize, String)| p.0 == entry && p.1 == question;
    if let Some(at) = chat.picks.iter().position(|p| same(p) && p.2 == label) {
        chat.picks.remove(at);
        return;
    }
    if !multi {
        chat.picks.retain(|p| !same(p));
    }
    chat.picks.push((entry, question, label));
}

pub(super) fn reply(state: &mut Dashboard, index: usize) {
    let Some(chat) = state.chat.chats.get_mut(state.chat.active) else {
        return;
    };
    let Some(entry) = chat.model.entries.get(index) else {
        return;
    };
    let answers: Vec<(String, String)> = entry
        .questions
        .iter()
        .enumerate()
        .filter_map(|(q, question)| {
            let labels: Vec<_> = chat
                .picks
                .iter()
                .filter(|p| p.0 == index && p.1 == q)
                .map(|p| p.2.as_str())
                .collect();
            (!labels.is_empty()).then(|| (question.question.clone(), labels.join(", ")))
        })
        .collect();
    if answers.is_empty() {
        return;
    }
    let map: serde_json::Map<String, Value> = answers
        .iter()
        .map(|(q, a)| (q.clone(), Value::String(a.clone())))
        .collect();
    let answer = json!({"type": "answer", "request_id": entry.request_id, "answers": map});
    let summary = answers
        .iter()
        .map(|(_, a)| a.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if let Some(turn) = chat.turn.as_mut()
        && let Err(error) = turn.answer(&answer)
    {
        state.message = error;
    }
    chat.picks.retain(|p| p.0 != index);
    chat.model.decide(index, &summary);
}

pub(super) fn new_chat(state: &mut Dashboard) {
    let root = state.root.clone();
    with_chat(state, |chat| {
        if let Some(turn) = chat.turn.as_mut() {
            turn.cancel();
        }
        chat.turn = None;
        chat.model.clear();
        chat.turn_replies.clear();
        chat.resume_recap.clear();
        chat.once.clear();
        chat.approved.clear();
        chat.composer = text_editor::Content::new();
        chat.rendered.clear();
        chat.selecting.clear();
        chat.picks.clear();
        let id = chat.agent.id.clone();
        std::thread::spawn(move || bridge::reset_chat(&root, &id));
    });
}

#[cfg(test)]
pub(super) fn with_images_for_tests(prompt: &str, count: usize) -> String {
    with_images(prompt, count)
}

#[cfg(test)]
pub(super) fn shown_for_tests(prompt: &str, count: usize) -> String {
    shown(prompt, count)
}

/// Sends the last message the person wrote again (after an error or a poor answer).
pub(super) fn retry(state: &mut Dashboard) {
    let last = state.chat.active_chat().and_then(|chat| {
        chat.model
            .entries
            .iter()
            .rev()
            .find(|e| e.kind == model::Kind::User)
            .map(|e| e.text.clone())
    });
    if let Some(text) = last
        && !text.trim().is_empty()
        && text != "Imágenes adjuntas"
    {
        start_turn(state, &text, true);
    }
}

/// Brings back an earlier message (`older`) or walks toward the newest and the unsent draft.
pub(super) fn history_step(chat: &mut Chat, older: bool) {
    let next = match (chat.recall, older) {
        (None, true) if !chat.history.is_empty() => {
            chat.draft = chat.composer.text();
            Some(chat.history.len() - 1)
        }
        (Some(at), true) => Some(at.saturating_sub(1)),
        (Some(at), false) if at + 1 < chat.history.len() => Some(at + 1),
        (Some(_), false) => None,
        _ => return,
    };
    let text = match next {
        Some(at) => chat.history[at].clone(),
        None => std::mem::take(&mut chat.draft),
    };
    chat.recall = next;
    chat.composer = text_editor::Content::with_text(&text);
    chat.composer
        .perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}
