//! Salud: what the doctor found about tools, project files, trust and MCP, read-only.
use super::app::App;
use super::theme::*;
use crate::features::doctor::DoctorReport;
use ratatui::crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

fn status_color(status: &str) -> Color {
    match status {
        "OK" | "ok" => SUCCESS,
        "FALTA" | "AVISO" => WARNING,
        "ERROR" => DANGER,
        _ => MUTED,
    }
}

fn count(report: &DoctorReport, status: &str) -> usize {
    report
        .findings
        .iter()
        .filter(|f| f.status == status)
        .count()
}

pub fn lines(report: &DoctorReport) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        badge(format!("{} bien", count(report, "OK")), SUCCESS),
        Span::raw(" "),
        badge(format!("{} faltan", count(report, "FALTA")), WARNING),
        Span::raw(" "),
        badge(format!("{} con error", count(report, "ERROR")), DANGER),
        Span::styled(format!("   perfil {}", report.profile), faint()),
    ])];
    let mut area = "";
    for finding in &report.findings {
        if finding.area != area {
            area = finding.area;
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                area.to_uppercase(),
                Style::new().fg(CYAN).bold(),
            )));
        }
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<6}", finding.status),
                Style::new().fg(status_color(finding.status)).bold(),
            ),
            Span::styled(finding.subject.clone(), text()),
            Span::styled(format!("  {}", finding.detail), faint()),
        ]));
    }
    if !report.mcp.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "MCP",
            Style::new().fg(CYAN).bold(),
        )));
        for item in &report.mcp {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<6}", item.status),
                    Style::new().fg(status_color(item.status)).bold(),
                ),
                Span::styled(format!("{} · {}", item.client, item.server), text()),
                Span::styled(format!("  {} · {}", item.scope, item.detail), faint()),
            ]));
        }
    }
    lines
}

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let body = match &app.doctor {
        Some(Ok(report)) => lines(report),
        Some(Err(error)) => vec![Line::from(Span::styled(
            error.clone(),
            Style::new().fg(DANGER),
        ))],
        None => vec![Line::from(Span::styled("Diagnosticando…", muted()))],
    };
    let max = (body.len() as u16).saturating_sub(area.height.saturating_sub(2));
    app.health_scroll = app.health_scroll.min(max);
    frame.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .scroll((app.health_scroll, 0))
            .block(
                focused_panel(" Salud · solo lectura ")
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            ),
        area,
    );
}

pub fn key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.health_scroll = app.health_scroll.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            app.health_scroll = app.health_scroll.saturating_add(1)
        }
        KeyCode::PageUp => app.health_scroll = app.health_scroll.saturating_sub(10),
        KeyCode::PageDown => app.health_scroll = app.health_scroll.saturating_add(10),
        KeyCode::Home => app.health_scroll = 0,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_read_by_color() {
        assert_eq!(status_color("OK"), SUCCESS);
        assert_eq!(status_color("FALTA"), WARNING);
        assert_eq!(status_color("ERROR"), DANGER);
        assert_eq!(status_color("otra cosa"), MUTED);
    }
}
