//! The preview every change goes through: what will happen, then `y` to apply it. Each plan
//! of the product becomes the same modal, and applying runs off the UI thread.
use super::theme::*;
use crate::features::{
    mcp_setup::{McpIntegrationPlan, apply_mcp_integration},
    project_mods::{ModsPlan, apply_mods_plan},
    project_rollback::{ProjectRollbackPlan, apply_project_rollback},
    project_setup::{ProjectInitPlan, apply_project_init},
    project_skills::{SkillsPlan, apply_skills},
    project_sync::{ProjectSyncPlan, apply_projects_sync},
    project_trust::{ProjectTrustPlan, apply_project_trust},
};
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph, Wrap};

/// A change that was previewed and can be applied.
pub enum Apply {
    Init(ProjectInitPlan),
    Trust(ProjectTrustPlan),
    Rollback(ProjectRollbackPlan),
    Mcp(McpIntegrationPlan),
    Sync(ProjectSyncPlan),
    Skills(SkillsPlan),
    Mods(ModsPlan),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Plain,
    Add,
    Warn,
    Note,
}

pub struct Preview {
    pub title: String,
    pub lines: Vec<(Tone, String)>,
    /// Why it cannot be applied, when it cannot.
    pub blocked: Option<String>,
    pub apply: Option<Apply>,
    pub scroll: u16,
}

impl Preview {
    fn new(title: &str) -> Self {
        Self {
            title: title.to_owned(),
            lines: Vec::new(),
            blocked: None,
            apply: None,
            scroll: 0,
        }
    }

    fn actions(mut self, actions: &[String]) -> Self {
        self.lines
            .extend(actions.iter().map(|a| (Tone::Add, a.clone())));
        self
    }

    fn warnings(mut self, warnings: &[String]) -> Self {
        self.lines
            .extend(warnings.iter().map(|w| (Tone::Warn, w.clone())));
        self
    }

    /// Applicable only when nothing blocks it and something would change.
    fn finish(mut self, blocked: &Option<String>, changes: bool, apply: Apply) -> Self {
        self.blocked = blocked.clone();
        if blocked.is_none() && changes {
            self.apply = Some(apply);
        } else if blocked.is_none() {
            self.lines.push((
                Tone::Note,
                "Todo está al día: no hay nada que cambiar.".into(),
            ));
        }
        self
    }

    pub fn init(plan: ProjectInitPlan, minimal: bool) -> Self {
        let title = if minimal {
            "Preparar proyecto · mínimo"
        } else {
            "Preparar proyecto"
        };
        let (blocked, changes) = (plan.blocked_reason.clone(), !plan.actions.is_empty());
        Self::new(title)
            .actions(&plan.actions)
            .warnings(&plan.warnings)
            .finish(&blocked, changes, Apply::Init(plan))
    }

    pub fn trust(plan: ProjectTrustPlan) -> Self {
        let title = if plan.desired_trust {
            "Dar confianza local"
        } else {
            "Revocar confianza local"
        };
        let mut preview = Self::new(title);
        preview.lines.push((
            Tone::Plain,
            format!(
                "Proyecto: {} · hoy {}",
                plan.project_root,
                if plan.currently_trusted {
                    "es confiable"
                } else {
                    "no es confiable"
                }
            ),
        ));
        preview = preview.actions(&plan.actions);
        preview.lines.push((
            Tone::Note,
            "Guardar la confianza no ejecuta ningún proceso.".into(),
        ));
        let changes = plan.changes_state;
        preview.finish(&None, changes, Apply::Trust(plan))
    }

    pub fn rollback(plan: ProjectRollbackPlan) -> Self {
        let mut preview = Self::new("Restaurar desde el último respaldo");
        preview
            .lines
            .push((Tone::Plain, format!("Respaldo: {}", plan.backup_path)));
        let (blocked, changes) = (plan.blocked_reason.clone(), !plan.actions.is_empty());
        preview
            .actions(&plan.actions)
            .finish(&blocked, changes, Apply::Rollback(plan))
    }

    pub fn mcp(plan: McpIntegrationPlan) -> Self {
        let mut preview = Self::new("Conectar herramientas (MCP)");
        preview.lines.push((
            Tone::Plain,
            format!("Clientes: {}", plan.client_labels.join(", ")),
        ));
        let (blocked, changes) = (plan.blocked_reason.clone(), !plan.actions.is_empty());
        preview
            .actions(&plan.actions)
            .warnings(&plan.warnings)
            .finish(&blocked, changes, Apply::Mcp(plan))
    }

    pub fn sync(plan: ProjectSyncPlan) -> Self {
        let mut preview = Self::new("Sincronizar proyectos registrados");
        let mut changes = false;
        for entry in &plan.entries {
            preview.lines.push((
                Tone::Plain,
                format!("{} · {}", entry.alias, entry.project_root),
            ));
            if let Some(reason) = &entry.blocked_reason {
                preview
                    .lines
                    .push((Tone::Warn, format!("  omitido: {reason}")));
            } else if entry.changes.is_empty() {
                preview.lines.push((Tone::Note, "  sin cambios".into()));
            } else {
                changes = true;
                preview
                    .lines
                    .extend(entry.changes.iter().map(|c| (Tone::Add, format!("  {c}"))));
            }
        }
        let warnings = plan.warnings.clone();
        preview
            .warnings(&warnings)
            .finish(&None, changes, Apply::Sync(plan))
    }

    pub fn skills(title: &str, plan: SkillsPlan) -> Self {
        let (blocked, changes) = (plan.blocked_reason.clone(), plan.changes_state());
        Self::new(title).actions(&plan.actions.clone()).finish(
            &blocked,
            changes,
            Apply::Skills(plan),
        )
    }

    pub fn mods(title: &str, plan: ModsPlan) -> Self {
        let (blocked, changes) = (plan.blocked_reason.clone(), plan.changes_state());
        Self::new(title)
            .actions(&plan.actions.clone())
            .finish(&blocked, changes, Apply::Mods(plan))
    }
}

fn join(mut message: String, label: &str, items: &[String]) -> String {
    if !items.is_empty() {
        message.push_str(&format!(" · {label}: {}", items.join("; ")));
    }
    message
}

/// Applies a previewed change and says, in one line, what happened.
pub fn apply(change: Apply) -> Result<String, String> {
    match change {
        Apply::Init(plan) => apply_project_init(&plan).map(|done| {
            let message = join(
                done.completed_actions.join(" · "),
                "Respaldo",
                &done.backups,
            );
            join(message, "Avisos", &done.warnings)
        }),
        Apply::Trust(plan) => apply_project_trust(&plan).map(|backup| {
            let text = if plan.desired_trust {
                "Confianza local guardada."
            } else {
                "Confianza local revocada."
            };
            join(
                text.to_owned(),
                "Respaldo",
                &backup.into_iter().collect::<Vec<_>>(),
            )
        }),
        Apply::Rollback(plan) => apply_project_rollback(&plan).map(|(actions, safety)| {
            join(
                actions.join(" · "),
                "Respaldo de seguridad",
                &safety.into_iter().collect::<Vec<_>>(),
            )
        }),
        Apply::Mcp(plan) => apply_mcp_integration(&plan)
            .map(|done| join(done.completed.join(" · "), "Avisos", &done.warnings)),
        Apply::Sync(plan) => apply_projects_sync(&plan).map(|done| {
            let message = done
                .outcomes
                .iter()
                .map(|o| {
                    let state = if o.changed_files.is_empty() {
                        "sin cambios"
                    } else {
                        "sincronizado"
                    };
                    format!("{} · {state}", o.project_root)
                })
                .collect::<Vec<_>>()
                .join(" · ");
            join(message, "Avisos", &done.warnings)
        }),
        Apply::Skills(plan) => apply_skills(&plan).map(|done| {
            join(
                format!("Skills habilitadas: {}", done.enabled.join(", ")),
                "Respaldo",
                &done.backups,
            )
        }),
        Apply::Mods(plan) => apply_mods_plan(&plan)
            .map(|done| join(done.completed.join(" · "), "Respaldo", &done.backups)),
    }
}

fn tone_style(tone: Tone) -> (&'static str, Style) {
    match tone {
        Tone::Plain => ("", text()),
        Tone::Add => ("+ ", Style::new().fg(SUCCESS)),
        Tone::Warn => ("! ", Style::new().fg(WARNING)),
        Tone::Note => ("· ", muted()),
    }
}

/// The modal, centered over whatever page is behind it.
pub fn render(frame: &mut Frame<'_>, preview: &Preview, running: bool) {
    let area = frame.area();
    let width = area.width.saturating_sub(8).clamp(40.min(area.width), 100);
    let height = area.height.saturating_sub(4).clamp(8.min(area.height), 30);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    let mut lines: Vec<Line> = preview
        .lines
        .iter()
        .map(|(tone, line)| {
            let (mark, style) = tone_style(*tone);
            Line::from(Span::styled(format!("{mark}{line}"), style))
        })
        .collect();
    if let Some(reason) = &preview.blocked {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ No se puede aplicar: {reason}"),
            Style::new().fg(DANGER).bold(),
        )));
    }
    let hint = if running {
        Line::from(Span::styled(
            " Aplicando… ",
            Style::new().fg(WARNING).bold(),
        ))
    } else if preview.apply.is_some() {
        Line::from(
            [
                key_hint("y", "Aplicar"),
                key_hint("Esc", "Cancelar"),
                key_hint("↑↓", "Desplazar"),
            ]
            .concat(),
        )
    } else {
        Line::from([key_hint("Esc", "Cerrar"), key_hint("↑↓", "Desplazar")].concat())
    };
    let block = focused_panel(format!(" {} ", preview.title)).title_bottom(hint);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((preview.scroll, 0))
            .block(block.padding(ratatui::widgets::Padding::horizontal(1))),
        rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preview_with_nothing_to_do_cannot_be_applied() {
        let preview = Preview::new("x").finish(&None, false, Apply::Init(sample_init()));
        assert!(preview.apply.is_none() && preview.blocked.is_none());
        assert!(preview.lines.iter().any(|(t, _)| *t == Tone::Note));
    }

    #[test]
    fn a_blocked_preview_says_why_and_offers_no_apply() {
        let preview = Preview::new("x").finish(
            &Some("config rota".into()),
            true,
            Apply::Init(sample_init()),
        );
        assert!(preview.apply.is_none());
        assert_eq!(preview.blocked.as_deref(), Some("config rota"));
    }

    #[test]
    fn a_change_that_can_happen_is_offered_for_confirmation() {
        let preview = Preview::new("x").finish(&None, true, Apply::Init(sample_init()));
        assert!(preview.apply.is_some());
    }

    fn sample_init() -> ProjectInitPlan {
        let dir = std::env::temp_dir();
        crate::features::project_setup::plan_project_init(&dir, true)
            .expect("a plan of a plain folder")
    }
}
