//! The terminal interface: the same product as Studio, in a terminal. Pages compose screens and
//! call use cases; nothing here runs processes or touches files by itself.
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::prelude::*;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

#[path = "tui_actions.rs"]
mod actions;
#[path = "tui_agents.rs"]
mod agents;
#[path = "tui_app.rs"]
mod app;
#[path = "tui_catalog.rs"]
mod catalog;
#[path = "tui_chrome.rs"]
mod chrome;
#[path = "tui_git.rs"]
mod git;
#[path = "tui_health.rs"]
mod health;
#[path = "tui_home.rs"]
mod home;
#[path = "tui_preview.rs"]
mod preview;
#[path = "tui_projects.rs"]
mod projects;
#[path = "tui_theme.rs"]
mod theme;

use app::{App, Kind, Page};

pub fn run(path: PathBuf) -> Result<(), String> {
    let mut app = App::new(path)?;
    // `CHXCHX_TUI_PAGE=1..8` opens on that page (handy for demos and screenshots).
    let first = std::env::var("CHXCHX_TUI_PAGE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .and_then(|n| Page::ALL.get(n.wrapping_sub(1)).copied())
        .unwrap_or(Page::Home);
    app.go(first);
    let mut terminal = ratatui::init();
    let _ = execute!(io::stdout(), EnableMouseCapture);
    let result = event_loop(&mut terminal, &mut app);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result.map_err(|error| format!("No se pudo usar la interfaz de terminal: {error}"))
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while !app.quit {
        app.poll();
        terminal.draw(|frame| draw(frame, app))?;
        if let Some(index) = app.launch.take() {
            launch(terminal, app, index);
            continue;
        }
        // Short enough to animate the spinner and notice finished work, long enough to stay idle.
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key),
            Event::Mouse(mouse) => handle_mouse(app, mouse),
            _ => {}
        }
    }
    Ok(())
}

/// Hands the terminal to an agent and takes it back when it exits.
fn launch(terminal: &mut DefaultTerminal, app: &mut App, index: usize) {
    let Some(agent) = app.agents.agents.get(index).cloned() else {
        return;
    };
    let root = PathBuf::from(&app.overview.project.root);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    println!(
        "\n  ChxChx · abriendo {} en {}\n  Al salir volverás al panel.\n",
        agent.label(),
        root.display()
    );
    let outcome = crate::features::agent_launch::run(&root, &agent);
    *terminal = ratatui::init();
    let _ = execute!(io::stdout(), EnableMouseCapture);
    match outcome {
        Ok(status) if status.success() => {
            app.say(Kind::Info, format!("{} terminó.", agent.label()))
        }
        Ok(status) => app.say(
            Kind::Info,
            format!(
                "{} terminó con código {}.",
                agent.label(),
                status.code().unwrap_or(-1)
            ),
        ),
        Err(error) => app.say(Kind::Error, error),
    }
    app.reload_all();
}

pub(super) fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    if area.width < chrome::MIN_WIDTH || area.height < chrome::MIN_HEIGHT {
        chrome::too_small(frame);
        return;
    }
    let body = chrome::frame(frame, app);
    match app.page {
        Page::Home => home::render(frame, app, body),
        Page::Setup | Page::Connect => actions::render(frame, app, body, app.page),
        Page::Catalog => catalog::render(frame, app, body),
        Page::Agents => agents::render(frame, app, body),
        Page::Git => git::render(frame, app, body),
        Page::Health => health::render(frame, app, body),
        Page::Projects => projects::render(frame, app, body),
    }
    if let Some(modal) = &app.modal {
        preview::render(frame, modal, app.busy.is_some());
    }
    if app.help {
        chrome::help(frame);
    }
}

pub(super) fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit = true;
        return;
    }
    if app.help {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?' | 'q')) {
            app.help = false;
        }
        return;
    }
    if app.modal.is_some() {
        modal_key(app, key.code);
        return;
    }
    match key.code {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Char('?') => app.help = true,
        KeyCode::Tab => app.go(app.page.step(1)),
        KeyCode::BackTab => app.go(app.page.step(-1)),
        KeyCode::Char(digit @ '1'..='8') => {
            app.go(Page::ALL[digit as usize - '1' as usize]);
        }
        KeyCode::Char('r') => {
            app.reload_all();
            app.say(Kind::Info, "Proyecto leído de nuevo.");
        }
        code => match app.page {
            Page::Home => home::key(app, code),
            Page::Setup | Page::Connect => actions::key(app, app.page, code),
            Page::Catalog => catalog::key(app, code),
            Page::Agents => agents::key(app, code),
            Page::Git => git::key(app, code),
            Page::Health => health::key(app, code),
            Page::Projects => projects::key(app, code),
        },
    }
}

fn modal_key(app: &mut App, code: KeyCode) {
    if app.busy.is_some() {
        return;
    }
    let Some(modal) = app.modal.as_mut() else {
        return;
    };
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.modal = None,
        KeyCode::Up | KeyCode::Char('k') => modal.scroll = modal.scroll.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => modal.scroll = modal.scroll.saturating_add(1),
        KeyCode::Char('y') | KeyCode::Enter => {
            if let Some(change) = modal.apply.take() {
                app.apply(change);
            }
        }
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    if app.modal.is_some() || app.help {
        return;
    }
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let hit = app.tab_hits.iter().find(|(rect, _)| {
                mouse.row == rect.y && mouse.column >= rect.x && mouse.column < rect.x + rect.width
            });
            if let Some((_, page)) = hit.copied() {
                app.go(page);
            }
        }
        MouseEventKind::ScrollDown => handle_key(app, KeyEvent::from(KeyCode::Down)),
        MouseEventKind::ScrollUp => handle_key(app, KeyEvent::from(KeyCode::Up)),
        _ => {}
    }
}

#[cfg(test)]
#[path = "tui_tests.rs"]
mod tests;
