//! Inicio: the project at a glance and the steps to get the most out of ChxChx.
use super::app::{App, Page};
use super::theme::*;
use crate::features::project_skills::enabled_skills;
use ratatui::prelude::*;
use ratatui::widgets::{Gauge, List, ListItem, ListState, Padding, Paragraph};

const BANNER: [&str; 6] = [
    " ██████╗██╗  ██╗██╗  ██╗ ██████╗██╗  ██╗██╗  ██╗",
    "██╔════╝██║  ██║╚██╗██╔╝██╔════╝██║  ██║╚██╗██╔╝",
    "██║     ███████║ ╚███╔╝ ██║     ███████║ ╚███╔╝ ",
    "██║     ██╔══██║ ██╔██╗ ██║     ██╔══██║ ██╔██╗ ",
    "╚██████╗██║  ██║██╔╝ ██╗╚██████╗██║  ██║██╔╝ ██╗",
    " ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝",
];

pub struct Step {
    pub title: &'static str,
    pub detail: &'static str,
    /// `None` when ChxChx cannot tell from here.
    pub done: Option<bool>,
    pub page: Page,
}

pub fn steps(app: &App) -> Vec<Step> {
    let root = std::path::PathBuf::from(&app.overview.project.root);
    let skills = enabled_skills(&root).map(|s| !s.is_empty()).ok();
    let agents = app.agents.agents.iter().any(|a| a.available());
    vec![
        Step {
            title: "Prepara tu proyecto",
            detail: "Crea la configuración y las guías para asistentes; ves los archivos antes de aplicar.",
            done: Some(app.overview.workspace.config_exists),
            page: Page::Setup,
        },
        Step {
            title: "Concede confianza local",
            detail: "Sin confianza ningún agente ni terminal ejecuta nada. Es tu decisión y se puede revocar.",
            done: Some(app.trusted()),
            page: Page::Setup,
        },
        Step {
            title: "Encuentra tus agentes",
            detail: "Claude Code, Codex y OpenCode se detectan solos; ábrelos desde aquí.",
            done: Some(agents),
            page: Page::Agents,
        },
        Step {
            title: "Elige las skills del proyecto",
            detail: "Guían a tus agentes con las prácticas de tu stack; hay recomendadas para ti.",
            done: skills,
            page: Page::Catalog,
        },
        Step {
            title: "Conecta tus herramientas (MCP)",
            detail: "Deja que tus asistentes usen la memoria y las herramientas del proyecto.",
            done: None,
            page: Page::Connect,
        },
        Step {
            title: "Revisa la salud del entorno",
            detail: "Qué herramientas faltan y qué está mal configurado, sin ejecutar nada.",
            done: None,
            page: Page::Health,
        },
    ]
}

fn banner(frame: &mut Frame<'_>, area: Rect) {
    let lines: Vec<Line> = BANNER
        .iter()
        .enumerate()
        .map(|(i, row)| {
            Line::from(Span::styled(
                *row,
                Style::new().fg(mix(CYAN, BLUE, i as f32 / 5.0)).bold(),
            ))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn project_card(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let project = &app.overview.project;
    let detection = &app.overview.detection;
    let row = |label: &'static str, value: String| {
        Line::from(vec![
            Span::styled(format!("{label:<8}"), faint()),
            Span::styled(value, text()),
        ])
    };
    let lines = vec![
        row("Nombre", project.name.clone()),
        row("Ruta", project.root.clone()),
        row("Perfil", detection.profile.clone()),
        row(
            "Marcas",
            if project.detected_markers.is_empty() {
                "ninguna conocida".into()
            } else {
                project.detected_markers.join(", ")
            },
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(panel(" Proyecto ").padding(Padding::horizontal(1))),
        area,
    );
}

fn state_card(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let ws = &app.overview.workspace;
    let config = match (ws.config_exists, ws.config_parseable, ws.config_supported) {
        (false, _, _) => (Some(false), "Configuración: sin preparar"),
        (true, Some(false), _) => (Some(false), "Configuración: TOML mal formado"),
        (true, Some(true), Some(false)) => (Some(false), "Configuración: no compatible"),
        (true, _, _) => (Some(true), "Configuración: compatible"),
    };
    let available = app.agents.agents.iter().filter(|a| a.available()).count();
    let lines = vec![
        check(Some(app.overview.project.git_repository), "Repositorio Git"),
        check(config.0, config.1),
        check(Some(app.trusted()), "Confianza local"),
        check(
            Some(available > 0),
            format!("Agentes disponibles: {available}"),
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(panel(" Estado ").padding(Padding::horizontal(1))),
        area,
    );
}

fn tags(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let d = &app.overview.detection;
    let mut spans = Vec::new();
    // A word that is both a stack and a language (cpp, rust) appears once.
    let mut seen = std::collections::BTreeSet::new();
    for (items, color) in [
        (&d.stacks, CYAN),
        (&d.languages, PURPLE),
        (&d.package_managers, ORANGE),
        (&d.infrastructure, SUCCESS),
    ] {
        for item in items.iter().filter(|item| seen.insert(item.as_str())) {
            spans.push(badge(item.clone(), color));
            spans.push(Span::raw(" "));
        }
    }
    if spans.is_empty() {
        spans.push(Span::styled(
            "No se detectó ninguna tecnología conocida.",
            muted(),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .wrap(ratatui::widgets::Wrap { trim: true })
            .block(panel(" Tecnología detectada ").padding(Padding::horizontal(1))),
        area,
    );
}

fn learning(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let steps = steps(app);
    let known = steps.iter().filter(|s| s.done.is_some()).count();
    let done = steps.iter().filter(|s| s.done == Some(true)).count();
    let block = panel(" Primeros pasos ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [bar, list] = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(inner);
    frame.render_widget(
        Gauge::default()
            .ratio(if known == 0 {
                0.0
            } else {
                done as f64 / known as f64
            })
            .label(format!("{done} de {known} listos"))
            .gauge_style(Style::new().fg(CYAN).bg(RAISED)),
        Rect::new(bar.x, bar.y, bar.width, 1),
    );
    let items: Vec<ListItem> = steps
        .iter()
        .map(|step| {
            let (mark, color) = match step.done {
                Some(true) => ("✓", SUCCESS),
                Some(false) => ("○", WARNING),
                None => ("›", FAINT),
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{mark} "), Style::new().fg(color).bold()),
                    Span::styled(step.title, text().bold()),
                    Span::styled(format!("  → {}", step.page.title()), faint()),
                ]),
                Line::from(Span::styled(format!("  {}", step.detail), muted())),
            ])
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.steps.at(items.len())));
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(Style::new().bg(ACTIVE))
            .highlight_symbol("▸ "),
        list,
        &mut state,
    );
}

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let tall = area.height >= 34;
    let [head, cards, chips, rest] = Layout::vertical([
        Constraint::Length(if tall { 7 } else { 1 }),
        Constraint::Length(6),
        Constraint::Length(4),
        Constraint::Min(6),
    ])
    .areas(area);
    if tall {
        banner(frame, Rect::new(head.x, head.y, head.width, 6));
        frame.render_widget(
            Paragraph::new("Control plane local para proyectos con agentes de IA")
                .style(muted())
                .alignment(Alignment::Center),
            Rect::new(head.x, head.y + 6, head.width, 1),
        );
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("ChxChx", Style::new().fg(CYAN).bold()),
                Span::styled(
                    " · control plane local para proyectos con agentes de IA",
                    muted(),
                ),
            ]))
            .alignment(Alignment::Center),
            head,
        );
    }
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(cards);
    project_card(frame, app, left);
    state_card(frame, app, right);
    tags(frame, app, chips);
    learning(frame, app, rest);
}

pub fn key(app: &mut App, code: ratatui::crossterm::event::KeyCode) {
    use ratatui::crossterm::event::KeyCode::*;
    let steps = steps(app);
    match code {
        Up | Char('k') => app.steps.go(-1, steps.len()),
        Down | Char('j') => app.steps.go(1, steps.len()),
        Enter => {
            if let Some(step) = steps.get(app.steps.at(steps.len())) {
                app.go(step.page);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_is_a_clean_rectangle() {
        let width = BANNER[0].chars().count();
        assert!(BANNER.iter().all(|row| row.chars().count() == width));
    }
}
