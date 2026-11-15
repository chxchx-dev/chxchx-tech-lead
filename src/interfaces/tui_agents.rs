//! Agentes: the assistants found on this machine. Intro opens one right here, in the same
//! terminal, and returns to the TUI when it exits.
use super::app::{App, Kind};
use super::theme::*;
use crate::features::agents::{Agent, Provider};
use ratatui::crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

fn tint(provider: Provider) -> Color {
    match provider {
        Provider::Claude => ORANGE,
        Provider::Codex => SUCCESS,
        Provider::OpenCode => PURPLE,
        Provider::Other => CYAN,
    }
}

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(area);
    let agents = &app.agents.agents;
    let selected = app.agent_rows.at(agents.len());
    let items: Vec<ListItem> = agents
        .iter()
        .map(|a| {
            let (mark, color) = if a.available() {
                ("●", SUCCESS)
            } else {
                ("○", WARNING)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{mark} "), Style::new().fg(color)),
                Span::styled(a.label(), Style::new().fg(tint(a.provider)).bold()),
                Span::styled(format!("  {}", a.id), faint()),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected((!agents.is_empty()).then_some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(focused_panel(" Agentes "))
            .highlight_style(Style::new().bg(ACTIVE).bold())
            .highlight_symbol("▸ "),
        left,
        &mut state,
    );
    let mut lines: Vec<Line> = Vec::new();
    if let Some(agent) = agents.get(selected) {
        lines.push(Line::from(Span::styled(
            agent.label(),
            Style::new().fg(tint(agent.provider)).bold(),
        )));
        lines.push(Line::from(if agent.available() {
            badge("disponible", SUCCESS)
        } else {
            badge("no instalado", WARNING)
        }));
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("Comando  ", faint()),
            Span::styled(agent.command.join(" "), text()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Origen   ", faint()),
            Span::styled(
                if agent.configured {
                    "configurado en el proyecto"
                } else {
                    "detectado en este equipo"
                },
                text(),
            ),
        ]));
        if let Some(path) = &agent.executable {
            lines.push(Line::from(vec![
                Span::styled("Ruta     ", faint()),
                Span::styled(path.display().to_string(), text()),
            ]));
        } else if !agent.provider.install_hint().is_empty() {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled("Instálalo con", muted())));
            lines.push(Line::from(Span::styled(
                format!("  {}", agent.provider.install_hint()),
                Style::new().fg(CYAN),
            )));
        }
        lines.push(Line::default());
        lines.push(if app.trusted() {
            check(Some(true), "Proyecto confiable: puede ejecutarse aquí")
        } else {
            check(Some(false), "Falta la confianza local (página Preparar)")
        });
    } else {
        lines.push(Line::from(Span::styled(
            "No se encontraron agentes instalados.",
            muted(),
        )));
        lines.push(Line::default());
        for provider in Provider::KNOWN {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", provider.label()),
                    Style::new().fg(tint(provider)).bold(),
                ),
                Span::styled(provider.install_hint(), text()),
            ]));
        }
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "¿Chat visual, editor o modelos locales? Abre ChxChx Studio: chxchx-studio .",
        faint(),
    )));
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" Detalle ").padding(ratatui::widgets::Padding::horizontal(1))),
        right,
    );
}

/// The agent that Intro would open, if it can be opened.
pub fn launchable(app: &App, index: usize) -> Result<&Agent, String> {
    let agent = app
        .agents
        .agents
        .get(index)
        .ok_or("No hay un agente elegido.")?;
    if !agent.available() {
        return Err(format!(
            "{} no está instalado. Instálalo con: {}",
            agent.label(),
            agent.provider.install_hint()
        ));
    }
    if !app.trusted() {
        return Err(
            "Concede confianza local al proyecto (página Preparar) antes de abrir un agente."
                .into(),
        );
    }
    Ok(agent)
}

pub fn key(app: &mut App, code: KeyCode) {
    let len = app.agents.agents.len();
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.agent_rows.go(-1, len),
        KeyCode::Down | KeyCode::Char('j') => app.agent_rows.go(1, len),
        KeyCode::Enter => {
            let index = app.agent_rows.at(len);
            match launchable(app, index) {
                Ok(_) => app.launch = Some(index),
                Err(error) => app.say(Kind::Error, error),
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_identify_each_provider() {
        assert_ne!(tint(Provider::Claude), tint(Provider::Codex));
        assert_ne!(tint(Provider::Codex), tint(Provider::OpenCode));
    }
}
