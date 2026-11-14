use super::*;

pub(super) fn update(state: &mut Dashboard, message: Message) -> Task<Message> {
    let high_frequency = matches!(&message, Message::Terminal(terminal::Event::Tick));
    let task = dispatch(state, message);
    if !high_frequency {
        session::capture(state);
    }
    task
}

fn dispatch(state: &mut Dashboard, message: Message) -> Task<Message> {
    if let Message::Terminal(event) = message {
        return terminal::update(state, event);
    }
    if let Message::Agents(event) = message {
        return agents::update(state, event);
    }
    if let Message::Editor(event) = message {
        return editor::update(state, event);
    }
    if let Message::Shell(event) = message {
        return shell::update(state, event);
    }
    if let Message::Session(event) = message {
        return session::update(state, event);
    }
    if let Message::Catalog(event) = message {
        return catalog::update(state, event);
    }
    if let Message::Menu(event) = message {
        return menu::update(state, event);
    }
    if let Message::Git(event) = message {
        return git::update(state, event);
    }
    if let Message::Search(event) = message {
        return search::update(state, event);
    }
    if let Message::Sequence(messages) = message {
        return Task::batch(messages.into_iter().map(Task::done));
    }
    if let Message::Done(done) = &message {
        return jobs::finish(state, done);
    }
    if (state.init_busy && !matches!(&message, Message::InitApplied(_)))
        || (state.sync_busy && !matches!(&message, Message::SyncApplied(_)))
        || (state.mcp_busy && !matches!(&message, Message::McpApplied(_)))
    {
        return Task::none();
    }
    match message {
        Message::Editor(_)
        | Message::Terminal(_)
        | Message::Agents(_)
        | Message::Shell(_)
        | Message::Catalog(_)
        | Message::Session(_)
        | Message::Menu(_)
        | Message::Search(_)
        | Message::Git(_)
        | Message::Sequence(_)
        | Message::Done(_) => unreachable!(),
        Message::Navigate(page) => {
            state.page = page;
            state.editor.home = true;
            if page == Page::Catalog && state.catalog.view.is_none() {
                return catalog::update(state, catalog::Event::Load);
            }
        }
        Message::Dismiss => {
            plans::clear_previews(state);
            state.message.clear();
        }
        Message::Refresh => return plans::refresh(state),
        Message::PreviewInit { minimal } => return plans::init(state, minimal),
        Message::ApplyInit => {
            if let Some(plan) = state.init_preview.clone() {
                state.init_busy = true;
                state.message = "Aplicando init en segundo plano…".to_owned();
                return Task::perform(
                    async move {
                        jobs::offload(move || {
                            crate::features::project_setup::apply_project_init(&plan)
                        })
                        .await
                        .unwrap_or_else(|| Err("Falló el worker de init".to_owned()))
                    },
                    Message::InitApplied,
                );
            }
        }
        Message::PreviewSync => return plans::sync(state),
        Message::ApplySync => {
            if let Some(plan) = state.sync_preview.clone() {
                state.sync_busy = true;
                state.message = "Sincronizando proyectos en segundo plano…".to_owned();
                return Task::perform(
                    async move {
                        jobs::offload(move || {
                            crate::features::project_sync::apply_projects_sync(&plan)
                        })
                        .await
                        .unwrap_or_else(|| Err("Falló el worker de sincronización".to_owned()))
                    },
                    Message::SyncApplied,
                );
            }
        }
        Message::SyncApplied(result) => {
            state.sync_busy = false;
            state.sync_preview = None;
            match result {
                Ok(result) => {
                    state.message = result
                        .outcomes
                        .iter()
                        .map(|outcome| {
                            if outcome.changed_files.is_empty() {
                                format!("{} · sin cambios", outcome.project_root)
                            } else {
                                format!("{} · sincronizado", outcome.project_root)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" · ");
                    if state.message.is_empty() {
                        state.message = "No hubo proyectos sincronizados.".to_owned();
                    }
                    let backups = result
                        .outcomes
                        .iter()
                        .filter_map(|outcome| outcome.backup.as_deref())
                        .collect::<Vec<_>>();
                    if !backups.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Backups: {}", backups.join(", ")));
                    }
                    if !result.warnings.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                    }
                }
                Err(error) => state.message = error,
            }
        }
        Message::InitApplied(result) => {
            state.init_busy = false;
            match result {
                Ok(result) => {
                    state.message = result.completed_actions.join(" · ");
                    if !result.backups.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Backup: {}", result.backups.join(", ")));
                    }
                    if !result.warnings.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                    }
                    state.init_preview = None;
                    return plans::refresh(state);
                }
                Err(error) => state.message = error,
            }
        }
        Message::PreviewRollback => return plans::rollback(state),
        Message::ApplyRollback => return plans::apply_rollback(state),
        Message::PreviewTrust { desired } => return plans::trust(state, desired),
        Message::ApplyTrust => return plans::apply_trust(state),
        Message::PreviewMcp => return plans::mcp(state),
        Message::Doctor => return plans::diagnose(state),
        Message::Projects => return plans::projects(state),
        Message::ApplyMcp => {
            if state.mcp_busy || state.init_busy {
                return Task::none();
            }
            if let Some(plan) = state.mcp_preview.clone() {
                state.mcp_busy = true;
                state.message = "Configurando MCP en segundo plano…".to_owned();
                return Task::perform(
                    async move {
                        jobs::offload(move || {
                            crate::features::mcp_setup::apply_mcp_integration(&plan)
                        })
                        .await
                        .unwrap_or_else(|| Err("Falló el worker de integración MCP".to_owned()))
                    },
                    Message::McpApplied,
                );
            }
        }
        Message::McpApplied(result) => {
            state.mcp_busy = false;
            state.mcp_preview = None;
            match result {
                Ok(result) => {
                    state.message = result.completed.join(" · ");
                    if !result.warnings.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                    }
                }
                Err(error) => state.message = error,
            }
        }
    }
    Task::none()
}
