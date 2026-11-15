//! Preparar and Conectar: lists of actions, each one opening a preview before anything changes.
use super::app::{App, Kind, Page};
use super::preview::Preview;
use super::theme::*;
use crate::features::{
    mcp_setup::{McpClient, plan_mcp_integration},
    project_rollback::plan_project_rollback,
    project_setup::plan_project_init,
    project_trust::plan_project_trust,
};
use ratatui::crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

pub struct Action {
    pub title: &'static str,
    pub detail: &'static str,
    run: fn(&App) -> Result<Preview, String>,
}

pub fn actions(page: Page) -> Vec<Action> {
    match page {
        Page::Connect => vec![
            Action {
                title: "Conectar herramientas MCP",
                detail: "Configura los servidores MCP (memoria y herramientas del proyecto) en Claude, Codex y OpenCode con sus CLI oficiales. Verás cada cambio antes de aplicarlo.",
                run: |app| {
                    let clients = McpClient::parse("all")?;
                    plan_mcp_integration(&app.path, &clients, false).map(Preview::mcp)
                },
            },
            Action {
                title: "Renovar la configuración MCP",
                detail: "Vuelve a escribir los servidores aunque ya existan, por si cambiaron de ruta o versión.",
                run: |app| {
                    let clients = McpClient::parse("all")?;
                    plan_mcp_integration(&app.path, &clients, true).map(Preview::mcp)
                },
            },
        ],
        _ => vec![
            Action {
                title: "Preparar el proyecto",
                detail: "Crea .ai/, AGENTS.md y CLAUDE.md con tu configuración, memoria y guías. Tu contenido propio se conserva y todo queda con respaldo.",
                run: |app| plan_project_init(&app.path, false).map(|p| Preview::init(p, false)),
            },
            Action {
                title: "Preparación mínima",
                detail: "Solo lo imprescindible: la configuración y las guías, sin carpetas extra. Ideal para repos que no quieres tocar mucho.",
                run: |app| plan_project_init(&app.path, true).map(|p| Preview::init(p, true)),
            },
            Action {
                title: "Dar confianza local",
                detail: "Permite que los agentes y terminales ejecuten procesos en este proyecto. Es una decisión explícita; guardarla no ejecuta nada.",
                run: |app| plan_project_trust(&app.path, true).map(Preview::trust),
            },
            Action {
                title: "Revocar la confianza",
                detail: "Los agentes y terminales dejan de poder ejecutar procesos aquí hasta que vuelvas a confiar.",
                run: |app| plan_project_trust(&app.path, false).map(Preview::trust),
            },
            Action {
                title: "Restaurar desde un respaldo",
                detail: "Devuelve los archivos administrados a su último respaldo. Antes crea otro respaldo del estado actual.",
                run: |app| plan_project_rollback(&app.path).map(Preview::rollback),
            },
        ],
    }
}

fn selection(app: &App, page: Page) -> usize {
    if page == Page::Connect {
        app.connect.at(actions(page).len())
    } else {
        app.setup.at(actions(page).len())
    }
}

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect, page: Page) {
    let list = actions(page);
    let selected = selection(app, page);
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).areas(area);
    let items: Vec<ListItem> = list
        .iter()
        .map(|a| ListItem::new(Line::from(Span::styled(a.title, text()))))
        .collect();
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(focused_panel(format!(" {} ", page.title())))
            .highlight_style(Style::new().fg(Color::Black).bg(CYAN).bold())
            .highlight_symbol(" ▸ "),
        left,
        &mut state,
    );
    let mut lines = vec![
        Line::from(Span::styled(
            list[selected].title,
            Style::new().fg(CYAN).bold(),
        )),
        Line::default(),
    ];
    lines.extend(
        wrap(
            list[selected].detail,
            right.width.saturating_sub(4) as usize,
        )
        .into_iter()
        .map(|l| Line::from(Span::styled(l, text()))),
    );
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "Intro abre la vista previa; nada cambia hasta que pulses y.",
        faint(),
    )));
    if page == Page::Setup {
        lines.push(Line::default());
        lines.push(check(
            Some(app.overview.workspace.config_exists),
            "Proyecto preparado",
        ));
        lines.push(check(Some(app.trusted()), "Confianza local concedida"));
    }
    if page == Page::Connect {
        lines.push(Line::default());
        lines.extend(diagnostics(app));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" Qué hace ").padding(ratatui::widgets::Padding::horizontal(1))),
        right,
    );
}

/// What the doctor found about MCP, as colored lines.
fn diagnostics(app: &App) -> Vec<Line<'static>> {
    let Some(Ok(report)) = &app.doctor else {
        return vec![Line::from(Span::styled(
            "Leyendo el estado de MCP…",
            muted(),
        ))];
    };
    if report.mcp.is_empty() {
        return vec![Line::from(Span::styled(
            "Todavía no hay servidores MCP configurados.",
            muted(),
        ))];
    }
    let mut lines = vec![Line::from(Span::styled(
        "Estado actual",
        Style::new().fg(CYAN).bold(),
    ))];
    for item in &report.mcp {
        let ok = matches!(item.status, "OK" | "ok");
        lines.push(Line::from(vec![
            Span::styled(
                if ok { "✓ " } else { "✗ " },
                Style::new().fg(if ok { SUCCESS } else { WARNING }).bold(),
            ),
            Span::styled(format!("{} · {}", item.client, item.server), text()),
            Span::styled(format!("  {}", item.detail), faint()),
        ]));
    }
    lines
}

pub fn key(app: &mut App, page: Page, code: KeyCode) {
    let len = actions(page).len();
    let sel = if page == Page::Connect {
        &mut app.connect
    } else {
        &mut app.setup
    };
    match code {
        KeyCode::Up | KeyCode::Char('k') => sel.go(-1, len),
        KeyCode::Down | KeyCode::Char('j') => sel.go(1, len),
        KeyCode::Enter => {
            let run = actions(page).remove(selection(app, page)).run;
            match run(app) {
                Ok(preview) => app.modal = Some(preview),
                Err(error) => app.say(Kind::Error, error),
            }
        }
        _ => {}
    }
}
