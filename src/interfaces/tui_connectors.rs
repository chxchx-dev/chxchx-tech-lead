//! Conectar: the MCP connectors that give an agent hands in Excel, Power BI, databases… and the
//! project memory servers. Intro previews adding (or removing) the selected one.
use super::app::{App, Kind};
use super::preview::Preview;
use super::theme::*;
use crate::features::mcp_connectors::{Connector, connectors};
use crate::features::mcp_connectors_plan::{
    ConnectorState, ConnectorsChange, Standing, connector_states, default_clients, plan_connectors,
};
use crate::features::mcp_setup::McpClient;
use ratatui::crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use std::collections::BTreeMap;

/// A value being typed for a connector parameter (a database path).
pub struct Prompt {
    pub connector: String,
    /// Parameters still to ask, the first one being typed.
    pub pending: Vec<(String, String, String)>,
    pub values: BTreeMap<String, String>,
    pub text: String,
}

#[derive(Clone, Copy)]
pub enum Row {
    /// The memory servers (index in `tui_actions::actions(Connect)`).
    Memory(usize),
    Connector(usize),
}

pub fn rows() -> Vec<Row> {
    let memory = super::actions::actions(super::app::Page::Connect).len();
    (0..memory)
        .map(Row::Memory)
        .chain((0..connectors().len()).map(Row::Connector))
        .collect()
}

pub fn states(app: &App) -> Vec<ConnectorState> {
    connector_states(&app.path, &McpClient::parse("all").unwrap_or_default()).unwrap_or_default()
}

fn mark(standing: Standing) -> (&'static str, Color) {
    match standing {
        Standing::Configured => ("✓", SUCCESS),
        Standing::Different => ("≠", WARNING),
        Standing::Absent => ("·", FAINT),
    }
}

fn summary(state: Option<&ConnectorState>) -> Span<'static> {
    let Some(state) = state else {
        return Span::raw("");
    };
    let configured = state
        .standing
        .iter()
        .filter(|(_, s)| *s == Standing::Configured)
        .count();
    if configured > 0 {
        Span::styled(format!(" ✓ {configured}"), Style::new().fg(SUCCESS).bold())
    } else if state
        .standing
        .iter()
        .any(|(_, s)| *s == Standing::Different)
    {
        Span::styled(" ≠", Style::new().fg(WARNING))
    } else {
        Span::raw("")
    }
}

fn origin_color(origin: &str) -> Color {
    match origin {
        "oficial" => SUCCESS,
        "referencia" => CYAN,
        _ => MUTED,
    }
}

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let all = rows();
    let selected = app.connect.at(all.len());
    let states = states(app);
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(area);
    let memory = super::actions::actions(super::app::Page::Connect);
    let mut items: Vec<ListItem> = Vec::new();
    let mut list_row = 0;
    let mut category = "";
    for (position, row) in all.iter().enumerate() {
        match *row {
            Row::Memory(index) => {
                if index == 0 {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "MEMORIA DEL PROYECTO",
                        Style::new().fg(FAINT).bold(),
                    ))));
                }
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("  {}", memory[index].title),
                    text(),
                ))));
            }
            Row::Connector(index) => {
                let c = &connectors()[index];
                if c.category != category {
                    category = &c.category;
                    items.push(ListItem::new(Line::from(Span::styled(
                        c.category.to_uppercase(),
                        Style::new().fg(FAINT).bold(),
                    ))));
                }
                let state = states.iter().find(|s| s.name == c.name);
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(format!("  {}", c.title), text()),
                    summary(state),
                ])));
            }
        }
        // Section headers take rows of their own: remember where the selected one landed.
        if position == selected {
            list_row = items.len() - 1;
        }
    }
    let mut state = ListState::default().with_selected(Some(list_row));
    frame.render_stateful_widget(
        List::new(items)
            .block(focused_panel(" Conectar "))
            .highlight_style(Style::new().fg(Color::Black).bg(CYAN).bold()),
        left,
        &mut state,
    );
    let lines = match all.get(selected) {
        Some(Row::Memory(index)) => memory_detail(app, *index, right.width),
        Some(Row::Connector(index)) => detail(
            &connectors()[*index],
            states.iter().find(|s| s.name == connectors()[*index].name),
            right.width,
        ),
        None => Vec::new(),
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" Detalle ").padding(ratatui::widgets::Padding::horizontal(1))),
        right,
    );
}

fn memory_detail(app: &App, index: usize, width: u16) -> Vec<Line<'static>> {
    let actions = super::actions::actions(super::app::Page::Connect);
    let mut lines = vec![
        Line::from(Span::styled(
            actions[index].title,
            Style::new().fg(CYAN).bold(),
        )),
        Line::default(),
    ];
    lines.extend(
        wrap(actions[index].detail, width.saturating_sub(4) as usize)
            .into_iter()
            .map(|l| Line::from(Span::styled(l, text()))),
    );
    if let Some(Ok(report)) = &app.doctor {
        lines.push(Line::default());
        for item in &report.mcp {
            let ok = matches!(item.status, "OK" | "ok");
            lines.push(Line::from(vec![
                Span::styled(
                    if ok { "✓ " } else { "✗ " },
                    Style::new().fg(if ok { SUCCESS } else { WARNING }).bold(),
                ),
                Span::styled(format!("{} · {}", item.client, item.server), text()),
            ]));
        }
    }
    lines
}

fn detail(c: &Connector, state: Option<&ConnectorState>, width: u16) -> Vec<Line<'static>> {
    let wrap_to = width.saturating_sub(4) as usize;
    let mut lines = vec![
        Line::from(Span::styled(c.title.clone(), Style::new().fg(CYAN).bold())),
        Line::from(vec![
            badge(c.origin.clone(), origin_color(&c.origin)),
            Span::raw(" "),
            badge(c.category.clone(), PURPLE),
        ]),
        Line::default(),
    ];
    lines.extend(
        wrap(&c.description, wrap_to)
            .into_iter()
            .map(|l| Line::from(Span::styled(l, text()))),
    );
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "Necesita",
        Style::new().fg(MUTED).bold(),
    )));
    for program in &c.requires {
        let missing = state.is_some_and(|s| s.missing_requirements.contains(program));
        lines.push(check(Some(!missing), format!("{program} en el PATH")));
    }
    for env in &c.env {
        let missing = state.is_some_and(|s| s.missing_env.contains(&env.name));
        let label = format!(
            "{}{}  {}",
            env.name,
            if env.required { "" } else { " (opcional)" },
            env.description
        );
        lines.push(if env.required {
            check(Some(!missing), label)
        } else {
            check(None, label)
        });
    }
    for param in &c.params {
        lines.push(check(
            None,
            format!("{}: {}", param.name, param.description),
        ));
    }
    if !c.note.is_empty() {
        lines.push(Line::default());
        lines.extend(
            wrap(&c.note, wrap_to)
                .into_iter()
                .map(|l| Line::from(Span::styled(l, Style::new().fg(WARNING)))),
        );
    }
    if let Some(state) = state {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "En este proyecto",
            Style::new().fg(MUTED).bold(),
        )));
        for (client, standing) in &state.standing {
            let (glyph, color) = mark(*standing);
            let what = match standing {
                Standing::Configured => "configurado",
                Standing::Different => "existe con otra configuración",
                Standing::Absent => "sin configurar",
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{glyph} "), Style::new().fg(color).bold()),
                Span::styled(format!("{} · {what}", client.label()), text()),
            ]));
        }
    }
    if !c.docs.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(c.docs.clone(), faint())));
    }
    lines
}

/// Opens the preview for the selected connector: adding it, or removing it when it is already
/// configured in every agent that would be touched.
fn open_preview(app: &mut App, index: usize, values: BTreeMap<String, String>) {
    let c = &connectors()[index];
    let root = std::path::PathBuf::from(&app.overview.project.root);
    let mut clients = default_clients(&root);
    if clients.is_empty() {
        clients = McpClient::parse("all").unwrap_or_default();
    }
    let configured = states(app)
        .iter()
        .find(|s| s.name == c.name)
        .is_some_and(|s| {
            clients.iter().all(|client| {
                s.standing
                    .iter()
                    .any(|(k, st)| k == client && *st == Standing::Configured)
            })
        });
    let (change, title) = if configured {
        (
            ConnectorsChange::Remove {
                names: vec![c.name.clone()],
            },
            format!("Quitar {}", c.title),
        )
    } else {
        (
            ConnectorsChange::Add {
                names: vec![c.name.clone()],
                values,
            },
            format!("Añadir {}", c.title),
        )
    };
    match plan_connectors(&app.path, &change, &clients) {
        Ok(plan) => app.modal = Some(Preview::connectors(&title, plan)),
        Err(error) => app.say(Kind::Error, error),
    }
}

pub fn key(app: &mut App, code: KeyCode) {
    let all = rows();
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.connect.go(-1, all.len()),
        KeyCode::Down | KeyCode::Char('j') => app.connect.go(1, all.len()),
        KeyCode::Enter => match all.get(app.connect.at(all.len())) {
            Some(Row::Memory(index)) => {
                let run = super::actions::actions(super::app::Page::Connect).remove(*index);
                match (run.run)(app) {
                    Ok(preview) => app.modal = Some(preview),
                    Err(error) => app.say(Kind::Error, error),
                }
            }
            Some(Row::Connector(index)) => {
                let c = &connectors()[*index];
                if c.params.is_empty() {
                    open_preview(app, *index, BTreeMap::new());
                } else {
                    let mut pending: Vec<_> = c
                        .params
                        .iter()
                        .map(|p| (p.name.clone(), p.description.clone(), p.placeholder.clone()))
                        .collect();
                    pending.reverse();
                    app.prompt = Some(Prompt {
                        connector: c.name.clone(),
                        pending,
                        values: BTreeMap::new(),
                        text: String::new(),
                    });
                }
            }
            None => {}
        },
        _ => {}
    }
}

/// Typing a parameter value: Intro takes it and asks the next one, then opens the preview.
pub fn prompt_key(app: &mut App, code: KeyCode) {
    let Some(prompt) = app.prompt.as_mut() else {
        return;
    };
    match code {
        KeyCode::Esc => app.prompt = None,
        KeyCode::Backspace => {
            prompt.text.pop();
        }
        KeyCode::Char(c) => prompt.text.push(c),
        KeyCode::Enter if !prompt.text.trim().is_empty() => {
            if let Some((name, _, _)) = prompt.pending.pop() {
                prompt.values.insert(name, prompt.text.trim().to_owned());
                prompt.text.clear();
            }
            if prompt.pending.is_empty() {
                let (connector, values) = (prompt.connector.clone(), prompt.values.clone());
                app.prompt = None;
                if let Some(index) = connectors().iter().position(|c| c.name == connector) {
                    open_preview(app, index, values);
                }
            }
        }
        _ => {}
    }
}

pub fn render_prompt(frame: &mut Frame<'_>, prompt: &Prompt) {
    let area = frame.area();
    let width = 70.min(area.width.saturating_sub(4));
    let rect = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height / 3,
        width,
        5,
    );
    frame.render_widget(ratatui::widgets::Clear, rect);
    let (name, description, placeholder) = prompt.pending.last().cloned().unwrap_or_default();
    let shown = if prompt.text.is_empty() {
        Span::styled(placeholder, faint())
    } else {
        Span::styled(format!("{}▌", prompt.text), text())
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(description, muted())),
            Line::from(vec![
                Span::styled(format!("{name}: "), Style::new().fg(CYAN).bold()),
                shown,
            ]),
        ])
        .block(
            focused_panel(format!(" Valor para {} ", prompt.connector)).title_bottom(Line::from(
                [key_hint("Intro", "Aceptar"), key_hint("Esc", "Cancelar")].concat(),
            )),
        ),
        rect,
    );
}
