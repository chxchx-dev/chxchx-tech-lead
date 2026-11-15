//! Git: where the branch stands, what changed and the recent history.
use super::app::App;
use super::theme::*;
use ratatui::crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph};

fn badge_color(letter: char) -> Color {
    match letter {
        'N' => SUCCESS,
        'M' => WARNING,
        'D' | '!' => DANGER,
        'R' => PURPLE,
        _ => MUTED,
    }
}

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let snapshot = match &app.git {
        None => {
            frame.render_widget(
                Paragraph::new("Leyendo Git…")
                    .style(muted())
                    .block(panel(" Git ")),
                area,
            );
            return;
        }
        Some(Err(error)) => {
            frame.render_widget(
                Paragraph::new(error.clone())
                    .style(Style::new().fg(DANGER))
                    .block(panel(" Git ")),
                area,
            );
            return;
        }
        Some(Ok(None)) => {
            frame.render_widget(
                Paragraph::new("Esta carpeta no es un repositorio Git.\n\nCréalo desde ChxChx Studio (Git → Iniciar repositorio) o con `git init`.")
                    .style(muted())
                    .block(panel(" Git ")),
                area,
            );
            return;
        }
        Some(Ok(Some(snapshot))) => snapshot,
    };
    let status = &snapshot.status;
    let [info, lists] = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).areas(area);
    let mut spans = vec![
        Span::styled(" ⎇ ", Style::new().fg(CYAN).bold()),
        Span::styled(status.head.clone(), text().bold()),
        Span::raw("  "),
    ];
    if let Some(upstream) = &status.upstream {
        spans.push(Span::styled(format!("→ {upstream}  "), faint()));
        if status.ahead > 0 {
            spans.push(badge(format!("↑{} por subir", status.ahead), SUCCESS));
            spans.push(Span::raw(" "));
        }
        if status.behind > 0 {
            spans.push(badge(format!("↓{} por bajar", status.behind), WARNING));
            spans.push(Span::raw(" "));
        }
    } else {
        spans.push(badge("sin remoto", FAINT));
    }
    let staged = status.staged_entries().len();
    let changed = status.changed_entries().len();
    spans.push(badge(format!("{staged} listos"), SUCCESS));
    spans.push(Span::raw(" "));
    spans.push(badge(
        format!("{changed} sin preparar"),
        if changed > 0 { WARNING } else { FAINT },
    ));
    if !status.conflicts().is_empty() {
        spans.push(Span::raw(" "));
        spans.push(badge(
            format!("{} conflictos", status.conflicts().len()),
            DANGER,
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(panel(" Rama ")),
        info,
    );

    let [files, history] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(lists);
    let entries: Vec<ListItem> = status
        .entries
        .iter()
        .map(|e| {
            let letter = e.badge();
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {letter} "),
                    Style::new().fg(badge_color(letter)).bold(),
                ),
                Span::styled(e.path.clone(), text()),
                Span::styled(
                    if e.staged() { "  ●" } else { "" },
                    Style::new().fg(SUCCESS),
                ),
            ]))
        })
        .collect();
    let commits: Vec<ListItem> = snapshot
        .commits
        .iter()
        .map(|c| {
            let mut spans = vec![
                Span::styled(format!("{} ", c.short_hash()), Style::new().fg(ORANGE)),
                Span::styled(c.subject.clone(), text()),
            ];
            if let Some(reference) = c.refs.first() {
                spans.push(Span::styled(
                    format!("  [{reference}]"),
                    Style::new().fg(CYAN),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    let lists = [
        (entries, files, " Cambios ", !app.git_history),
        (commits, history, " Historial ", app.git_history),
    ];
    for (items, rect, title, focused) in lists {
        let empty = items.is_empty();
        let block = if focused {
            focused_panel(title)
        } else {
            panel(title)
        };
        let mut state = ListState::default();
        if focused && !empty {
            state.select(Some(app.git_rows.at(items.len())));
        }
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().bg(ACTIVE).bold())
            .highlight_symbol("▸ ");
        frame.render_stateful_widget(list, rect, &mut state);
        if empty {
            frame.render_widget(
                Paragraph::new("Nada por mostrar.").style(faint()),
                Rect::new(rect.x + 2, rect.y + 1, rect.width.saturating_sub(4), 1),
            );
        }
    }
}

pub fn key(app: &mut App, code: KeyCode) {
    let len = match &app.git {
        Some(Ok(Some(s))) => {
            if app.git_history {
                s.commits.len()
            } else {
                s.status.entries.len()
            }
        }
        _ => 0,
    };
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.git_rows.go(-1, len),
        KeyCode::Down | KeyCode::Char('j') => app.git_rows.go(1, len),
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
            app.git_history = !app.git_history;
            app.git_rows.0 = 0;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_of_change_has_its_own_color() {
        assert_ne!(badge_color('N'), badge_color('M'));
        assert_ne!(badge_color('M'), badge_color('D'));
        assert_eq!(badge_color('D'), badge_color('!'));
    }
}
