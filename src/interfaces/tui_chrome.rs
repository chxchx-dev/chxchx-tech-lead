//! What surrounds every page: the header with the tabs, the footer with the keys that work
//! right now, the toast, and the help overlay.
use super::app::{App, CatalogTab, Kind, Page};
use super::theme::*;
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph};

pub const MIN_WIDTH: u16 = 64;
pub const MIN_HEIGHT: u16 = 18;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// The header, the page area and the footer; returns the page area.
pub fn frame(frame: &mut Frame<'_>, app: &mut App) -> Rect {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .areas(frame.area());
    render_header(frame, app, header);
    render_footer(frame, app, footer);
    body
}

fn render_header(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let [top, tabs, rule] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let project = &app.overview.project;
    let trust = if app.trusted() {
        badge("confiable", SUCCESS)
    } else {
        badge("sin confianza", WARNING)
    };
    let left = Line::from(vec![
        Span::styled(" ◆ ", Style::new().fg(CYAN).bold()),
        Span::styled("ChxChx", Style::new().fg(TEXT).bold()),
        Span::styled("  ·  ", faint()),
        Span::styled(project.name.clone(), text()),
    ]);
    frame.render_widget(Paragraph::new(left), top);
    let right = Line::from(vec![
        if project.git_repository {
            badge("git", CYAN)
        } else {
            badge("sin git", FAINT)
        },
        Span::raw(" "),
        trust,
        Span::raw(" "),
    ])
    .right_aligned();
    frame.render_widget(Paragraph::new(right), top);

    // Tabs, drawn by hand so a click can find the one it landed on.
    app.tab_hits.clear();
    let mut spans = vec![Span::raw(" ")];
    let mut x = tabs.x + 1;
    for (number, page) in Page::ALL.iter().enumerate() {
        let label = format!(" {} {} ", number + 1, page.title());
        let width = label.chars().count() as u16;
        let style = if *page == app.page {
            Style::new().fg(Color::Black).bg(CYAN).bold()
        } else {
            muted()
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
        app.tab_hits.push((Rect::new(x, tabs.y, width, 1), *page));
        x += width + 1;
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), tabs);
    frame.render_widget(
        Paragraph::new("─".repeat(rule.width as usize)).style(Style::new().fg(BORDER)),
        rule,
    );
}

fn page_hints(app: &App) -> Vec<Span<'static>> {
    let mut hints = Vec::new();
    match app.page {
        Page::Home => {
            hints.extend(key_hint("↑↓", "Elegir paso"));
            hints.extend(key_hint("Intro", "Ir"));
        }
        Page::Setup => {
            hints.extend(key_hint("↑↓", "Elegir"));
            hints.extend(key_hint("Intro", "Previsualizar"));
        }
        Page::Connect => {
            hints.extend(key_hint("↑↓", "Elegir"));
            hints.extend(key_hint("Intro", "Añadir o quitar"));
        }
        Page::Catalog => {
            hints.extend(key_hint("←→", "Skills/Packs/Mods"));
            hints.extend(key_hint("↑↓", "Elegir"));
            hints.extend(key_hint("Intro", "Activar/quitar"));
            if app.catalog_tab != CatalogTab::Packs {
                hints.extend(key_hint("a", "Recomendados"));
            }
        }
        Page::Agents => {
            hints.extend(key_hint("↑↓", "Elegir"));
            hints.extend(key_hint("Intro", "Abrir en esta terminal"));
        }
        Page::Git => {
            hints.extend(key_hint("←→", "Cambios/Historial"));
            hints.extend(key_hint("↑↓", "Elegir"));
        }
        Page::Health => hints.extend(key_hint("↑↓", "Desplazar")),
        Page::Projects => {
            hints.extend(key_hint("↑↓", "Elegir"));
            hints.extend(key_hint("s", "Sincronizar"));
        }
    }
    hints.extend(key_hint("Tab", "Página"));
    hints.extend(key_hint("r", "Releer"));
    hints.extend(key_hint("?", "Ayuda"));
    hints.extend(key_hint("q", "Salir"));
    hints
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let [status, keys] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    let line = if let Some(job) = &app.busy {
        Line::from(vec![
            Span::styled(
                format!(" {} ", SPINNER[app.tick % SPINNER.len()]),
                Style::new().fg(CYAN).bold(),
            ),
            Span::styled(format!("{job}…"), muted()),
        ])
    } else if let Some(toast) = &app.toast {
        let (mark, color) = match toast.kind {
            Kind::Ok => ("✓", SUCCESS),
            Kind::Error => ("✗", DANGER),
            Kind::Info => ("ℹ", CYAN),
        };
        Line::from(vec![
            Span::styled(format!(" {mark} "), Style::new().fg(color).bold()),
            Span::styled(toast.text.clone(), Style::new().fg(color)),
        ])
    } else {
        Line::from(Span::styled(
            format!(" {}", app.overview.project.root),
            faint(),
        ))
    };
    frame.render_widget(Paragraph::new(line), status);
    frame.render_widget(Paragraph::new(Line::from(page_hints(app))), keys);
}

pub fn too_small(frame: &mut Frame<'_>) {
    let area = frame.area();
    frame.render_widget(
        Paragraph::new(format!(
            "Agranda la terminal (mínimo {MIN_WIDTH}×{MIN_HEIGHT}).\nAhora: {}×{}.",
            area.width, area.height
        ))
        .style(Style::new().fg(WARNING))
        .alignment(Alignment::Center),
        area,
    );
}

const HELP: [(&str, &str); 12] = [
    ("Tab · Mayús+Tab · 1-8", "Cambiar de página"),
    ("↑ ↓ · j k", "Mover la selección o desplazar"),
    ("← → · h l", "Cambiar de lista dentro de la página"),
    (
        "Intro",
        "Ejecutar la acción elegida (siempre con vista previa)",
    ),
    ("y", "Aplicar lo que muestra la vista previa"),
    ("Esc", "Cerrar la vista previa o la ayuda"),
    ("r", "Volver a leer el proyecto"),
    ("s", "Proyectos: sincronizar los registrados"),
    ("a", "Catálogo: aplicar los recomendados"),
    ("Ratón", "Clic en una pestaña · rueda para mover listas"),
    ("?", "Esta ayuda"),
    ("q · Ctrl+C", "Salir"),
];

pub fn help(frame: &mut Frame<'_>) {
    let area = frame.area();
    let width = 78.min(area.width.saturating_sub(4));
    let height = (HELP.len() as u16 + 4).min(area.height);
    let rect = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    let lines: Vec<Line> = HELP
        .iter()
        .map(|(keys, what)| {
            Line::from(vec![
                Span::styled(format!("{keys:<24}"), Style::new().fg(CYAN).bold()),
                Span::styled(*what, text()),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(
            focused_panel(" Atajos ")
                .title_bottom(Line::from(key_hint("Esc", "Cerrar")))
                .padding(ratatui::widgets::Padding::horizontal(1)),
        ),
        rect,
    );
}
