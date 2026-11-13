//! Previews and read-only checks. Each runs in the background and fills one panel.
use super::*;
use crate::features::{doctor, mcp_setup, project_catalog, project_rollback, project_setup};
use crate::features::{project_sync, project_trust};

pub(super) fn clear_previews(state: &mut Dashboard) {
    state.init_preview = None;
    state.sync_preview = None;
    state.rollback_preview = None;
    state.trust_preview = None;
    state.mcp_preview = None;
    state.catalog.skills_plan = None;
    state.catalog.mods_plan = None;
}

/// Reads the project again without blocking the window.
pub(super) fn refresh(state: &mut Dashboard) -> Task<Message> {
    let root = state.root.clone();
    state.overview_loading = true;
    jobs::run(
        state,
        move || inspect_project(&root),
        |state, overview| {
            state.overview_loading = false;
            state.overview = overview;
            Task::none()
        },
    )
}

fn show<T: Send + 'static>(
    state: &mut Dashboard,
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
    notice: &'static str,
    store: impl FnOnce(&mut Dashboard, T) + Send + 'static,
) -> Task<Message> {
    jobs::run(state, job, move |state, result| {
        match result {
            Ok(value) => {
                clear_previews(state);
                state.message = notice.to_owned();
                store(state, value);
            }
            Err(error) => state.message = error,
        }
        Task::none()
    })
}

pub(super) fn init(state: &mut Dashboard, minimal: bool) -> Task<Message> {
    let root = state.root.clone();
    show(
        state,
        move || project_setup::plan_project_init(&root, minimal),
        "Revisa las acciones antes de aplicarlas.",
        |state, plan| {
            if plan.blocked_reason.is_some() {
                state.message = "La configuración existente requiere atención.".to_owned();
            }
            state.init_preview = Some(plan);
        },
    )
}

pub(super) fn sync(state: &mut Dashboard) -> Task<Message> {
    show(
        state,
        project_sync::plan_projects_sync,
        "Revisa los bloques administrados antes de sincronizar.",
        |state, plan| state.sync_preview = Some(plan),
    )
}

pub(super) fn rollback(state: &mut Dashboard) -> Task<Message> {
    let root = state.root.clone();
    show(
        state,
        move || project_rollback::plan_project_rollback(&root),
        "Revisa los archivos que se restaurarán.",
        |state, plan| state.rollback_preview = Some(plan),
    )
}

pub(super) fn trust(state: &mut Dashboard, desired: bool) -> Task<Message> {
    let root = state.root.clone();
    show(
        state,
        move || project_trust::plan_project_trust(&root, desired),
        "Revisa el cambio de confianza antes de guardarlo.",
        |state, plan| state.trust_preview = Some(plan),
    )
}

pub(super) fn mcp(state: &mut Dashboard) -> Task<Message> {
    let root = state.root.clone();
    show(
        state,
        move || {
            mcp_setup::McpClient::parse("all")
                .and_then(|clients| mcp_setup::plan_mcp_integration(&root, &clients, false))
        },
        "Revisa el alcance de MCP antes de aplicar.",
        |state, plan| state.mcp_preview = Some(plan),
    )
}

pub(super) fn diagnose(state: &mut Dashboard) -> Task<Message> {
    let root = state.root.clone();
    jobs::run(
        state,
        move || doctor::diagnose_project(&root),
        |state, result| {
            match result {
                Ok(report) => {
                    state.message = "Diagnóstico solo lectura completado.".to_owned();
                    state.doctor_report = Some(report);
                }
                Err(error) => state.message = error,
            }
            Task::none()
        },
    )
}

pub(super) fn projects(state: &mut Dashboard) -> Task<Message> {
    jobs::run(
        state,
        project_catalog::inspect_registered_projects,
        |state, result| {
            match result {
                Ok(catalog) => {
                    state.projects_catalog = Some(catalog);
                    state.message = "Lista de proyectos actualizada.".to_owned();
                }
                Err(error) => state.message = error,
            }
            Task::none()
        },
    )
}

pub(super) fn apply_rollback(state: &mut Dashboard) -> Task<Message> {
    let Some(plan) = state.rollback_preview.clone() else {
        return Task::none();
    };
    jobs::run(
        state,
        move || project_rollback::apply_project_rollback(&plan),
        |state, result| {
            match result {
                Ok((actions, safety_backup)) => {
                    state.message = actions.join(" · ");
                    if let Some(backup) = safety_backup {
                        state
                            .message
                            .push_str(&format!(" · Backup de seguridad: {backup}"));
                    }
                    state.rollback_preview = None;
                    return refresh(state);
                }
                Err(error) => state.message = error,
            }
            Task::none()
        },
    )
}

pub(super) fn apply_trust(state: &mut Dashboard) -> Task<Message> {
    let Some(plan) = state.trust_preview.clone() else {
        return Task::none();
    };
    let (changed, desired) = (plan.changes_state, plan.desired_trust);
    jobs::run(
        state,
        move || project_trust::apply_project_trust(&plan),
        move |state, result| {
            match result {
                Ok(backup) => {
                    state.message = if !changed {
                        "Sin cambios; la confianza ya estaba en ese estado.".to_owned()
                    } else if desired {
                        "Confianza local guardada.".to_owned()
                    } else {
                        "Confianza local revocada.".to_owned()
                    };
                    if let Some(backup) = backup {
                        state.message.push_str(&format!(" · Backup: {backup}"));
                    }
                    state.trust_preview = None;
                    return refresh(state);
                }
                Err(error) => state.message = error,
            }
            Task::none()
        },
    )
}
