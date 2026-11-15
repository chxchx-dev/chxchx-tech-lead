//! Proyectos: the projects ChxChx knows about, and syncing their guides in one go.
use super::app::{App, Kind};
use super::preview::Preview;
use super::theme::*;
use crate::features::project_sync::plan_projects_sync;
use ratatui::crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let catalog = match &app.projects {
        Some(Ok(catalog)) => catalog,
        Some(Err(error)) => {
            frame.render_widget(
                Paragraph::new(error.clone())
                    .style(Style::new().fg(DANGER))
                    .block(panel(" Proyectos ")),
                area,
            );
            return;
        }
        None => return,
    };
    if catalog.projects.is_empty() {
        frame.render_widget(
            Paragraph::new("Todavía no hay proyectos registrados.\n\nPrepara uno desde la página Preparar y aparecerá aquí.")
                .style(muted())
                .block(panel(" Proyectos ")),
            area,
        );
        return;
    }
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).areas(area);
    let selected = app.project_rows.at(catalog.projects.len());
    let items: Vec<ListItem> = catalog
        .projects
        .iter()
        .map(|p| {
            let (mark, color) = if p.available {
                ("●", SUCCESS)
            } else {
                ("○", DANGER)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{mark} "), Style::new().fg(color)),
                Span::styled(p.name.clone(), text()),
                Span::styled(
                    if p.is_current { "  último" } else { "" },
                    Style::new().fg(CYAN),
                ),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(focused_panel(" Registrados "))
            .highlight_style(Style::new().bg(ACTIVE).bold())
            .highlight_symbol("▸ "),
        left,
        &mut state,
    );
    let p = &catalog.projects[selected];
    let row = |label: &'static str, value: String| {
        Line::from(vec![
            Span::styled(format!("{label:<9}"), faint()),
            Span::styled(value, text()),
        ])
    };
    let lines = vec![
        Line::from(Span::styled(p.name.clone(), Style::new().fg(CYAN).bold())),
        Line::from(if p.available {
            badge("disponible", SUCCESS)
        } else {
            badge("no encontrado", DANGER)
        }),
        Line::default(),
        row("Alias", p.alias.clone()),
        row("Perfil", p.profile.clone()),
        row(
            "Stacks",
            if p.stacks.is_empty() {
                "ninguno".into()
            } else {
                p.stacks.join(", ")
            },
        ),
        row("Ruta", p.path.clone().unwrap_or_else(|| "<ausente>".into())),
        Line::default(),
        Line::from(Span::styled(
            "s abre la vista previa para actualizar las guías de todos los proyectos disponibles.",
            faint(),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" Detalle ").padding(ratatui::widgets::Padding::horizontal(1))),
        right,
    );
}

pub fn key(app: &mut App, code: KeyCode) {
    let len = match &app.projects {
        Some(Ok(c)) => c.projects.len(),
        _ => 0,
    };
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.project_rows.go(-1, len),
        KeyCode::Down | KeyCode::Char('j') => app.project_rows.go(1, len),
        KeyCode::Char('s') => match plan_projects_sync() {
            Ok(plan) => app.modal = Some(Preview::sync(plan)),
            Err(error) => app.say(Kind::Error, error),
        },
        _ => {}
    }
}
