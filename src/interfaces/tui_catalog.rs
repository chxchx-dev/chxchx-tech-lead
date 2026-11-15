//! Catálogo: skills, Tech Packs and Mods that guide the agents, recommended for the project's
//! stack. Enabling or removing one opens a preview like every other change.
use super::app::{App, CatalogTab, Kind};
use super::preview::Preview;
use super::theme::*;
use crate::features::{
    catalog_view::CatalogView,
    project_mods::{ModsChange, plan_mods},
    project_skills::{SkillsChange, plan_skills},
};
use ratatui::crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

/// `(marker, color, name, description)` of every row of the open tab.
fn rows(view: &CatalogView, tab: CatalogTab) -> Vec<(char, Color, String, String)> {
    let mark = |on: bool, recommended: bool| match (on, recommended) {
        (true, _) => ('✓', SUCCESS),
        (false, true) => ('★', WARNING),
        (false, false) => ('·', FAINT),
    };
    match tab {
        CatalogTab::Skills => view
            .skills
            .iter()
            .map(|s| {
                let (m, c) = mark(s.enabled, s.recommended);
                (m, c, s.name.clone(), s.description.clone())
            })
            .collect(),
        CatalogTab::Packs => view
            .packs
            .iter()
            .map(|p| {
                let (m, c) = mark(
                    p.enabled_skills == p.skills.len() && !p.skills.is_empty(),
                    p.recommended,
                );
                (m, c, p.name.clone(), p.description.clone())
            })
            .collect(),
        CatalogTab::Mods => view
            .mods
            .iter()
            .map(|m| {
                let (mk, c) = mark(m.applied, m.recommended);
                (
                    mk,
                    if m.conflict { DANGER } else { c },
                    m.name.clone(),
                    m.description.clone(),
                )
            })
            .collect(),
    }
}

fn detail(view: &CatalogView, tab: CatalogTab, index: usize, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let push = |lines: &mut Vec<Line<'static>>, text_: &str, style: Style| {
        for l in wrap(text_, width) {
            lines.push(Line::from(Span::styled(l, style)));
        }
    };
    let reasons = |lines: &mut Vec<Line<'static>>, reasons: &[String]| {
        if !reasons.is_empty() {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "★ Recomendado porque",
                Style::new().fg(WARNING).bold(),
            )));
            for reason in reasons {
                lines.push(Line::from(Span::styled(format!("  · {reason}"), text())));
            }
        }
    };
    match tab {
        CatalogTab::Skills => {
            let Some(s) = view.skills.get(index) else {
                return lines;
            };
            lines.push(Line::from(Span::styled(
                s.name.clone(),
                Style::new().fg(CYAN).bold(),
            )));
            lines.push(Line::from(if s.enabled {
                badge("habilitada", SUCCESS)
            } else {
                badge("sin habilitar", FAINT)
            }));
            lines.push(Line::default());
            push(&mut lines, &s.description, text());
            if !s.tags.is_empty() {
                lines.push(Line::default());
                lines.push(Line::from(
                    s.tags
                        .iter()
                        .flat_map(|t| [badge(t.clone(), PURPLE), Span::raw(" ")])
                        .collect::<Vec<_>>(),
                ));
            }
            reasons(&mut lines, &s.reasons);
        }
        CatalogTab::Packs => {
            let Some(p) = view.packs.get(index) else {
                return lines;
            };
            lines.push(Line::from(Span::styled(
                p.name.clone(),
                Style::new().fg(CYAN).bold(),
            )));
            lines.push(Line::from(badge(
                format!("{} de {} skills", p.enabled_skills, p.skills.len()),
                if p.enabled_skills == p.skills.len() {
                    SUCCESS
                } else {
                    MUTED
                },
            )));
            lines.push(Line::default());
            push(&mut lines, &p.description, text());
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "Incluye",
                Style::new().fg(MUTED).bold(),
            )));
            for skill in &p.skills {
                lines.push(Line::from(Span::styled(format!("  · {skill}"), text())));
            }
            reasons(&mut lines, &p.reasons);
        }
        CatalogTab::Mods => {
            let Some(m) = view.mods.get(index) else {
                return lines;
            };
            lines.push(Line::from(Span::styled(
                m.name.clone(),
                Style::new().fg(CYAN).bold(),
            )));
            lines.push(Line::from(if m.conflict {
                badge("conflicto: hay un archivo tuyo", DANGER)
            } else if m.applied {
                badge("instalado", SUCCESS)
            } else {
                badge("sin instalar", FAINT)
            }));
            lines.push(Line::default());
            push(&mut lines, &m.description, text());
            reasons(&mut lines, &m.reasons);
        }
    }
    lines
}

fn title(view: &CatalogView, tab: CatalogTab) -> Line<'static> {
    let label = |t: CatalogTab, name: &str, n: usize| {
        let on = t == tab;
        Span::styled(
            format!(" {name} {n} "),
            if on {
                Style::new().fg(Color::Black).bg(CYAN).bold()
            } else {
                muted()
            },
        )
    };
    Line::from(vec![
        label(CatalogTab::Skills, "Skills", view.skills.len()),
        label(CatalogTab::Packs, "Packs", view.packs.len()),
        label(CatalogTab::Mods, "Mods", view.mods.len()),
    ])
}

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let view = match &app.catalog {
        Some(Ok(view)) => view,
        Some(Err(error)) => {
            frame.render_widget(
                Paragraph::new(error.clone())
                    .style(Style::new().fg(DANGER))
                    .block(panel(" Catálogo ")),
                area,
            );
            return;
        }
        None => return,
    };
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(46), Constraint::Percentage(54)]).areas(area);
    let list = rows(view, app.catalog_tab);
    let selected = app.catalog_rows.at(list.len());
    let items: Vec<ListItem> = list
        .iter()
        .map(|(m, c, name, _)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{m} "), Style::new().fg(*c).bold()),
                Span::styled(name.clone(), text()),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(
                focused_panel(title(view, app.catalog_tab)).title_bottom(Line::from(vec![
                    Span::styled(" ✓ habilitado ", Style::new().fg(SUCCESS)),
                    Span::styled(" ★ recomendado ", Style::new().fg(WARNING)),
                ])),
            )
            .highlight_style(Style::new().bg(ACTIVE).bold())
            .highlight_symbol("▸ "),
        left,
        &mut state,
    );
    let width = right.width.saturating_sub(4) as usize;
    let mut lines = detail(view, app.catalog_tab, selected, width);
    if let Some(error) = &view.selection_error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("! {error}"),
            Style::new().fg(WARNING),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" Detalle ").padding(ratatui::widgets::Padding::horizontal(1))),
        right,
    );
}

fn open(app: &mut App, plan: Result<Preview, String>) {
    match plan {
        Ok(preview) => app.modal = Some(preview),
        Err(error) => app.say(Kind::Error, error),
    }
}

pub fn key(app: &mut App, code: KeyCode) {
    let Some(Ok(view)) = &app.catalog else { return };
    let tabs = [CatalogTab::Skills, CatalogTab::Packs, CatalogTab::Mods];
    let current = tabs.iter().position(|t| *t == app.catalog_tab).unwrap_or(0);
    let len = rows(view, app.catalog_tab).len();
    let index = app.catalog_rows.at(len);
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.catalog_rows.go(-1, len),
        KeyCode::Down | KeyCode::Char('j') => app.catalog_rows.go(1, len),
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
            let delta = if matches!(code, KeyCode::Left | KeyCode::Char('h')) {
                2
            } else {
                1
            };
            app.catalog_tab = tabs[(current + delta) % 3];
            app.catalog_rows.0 = 0;
        }
        KeyCode::Enter => {
            let path = app.path.clone();
            let plan = match app.catalog_tab {
                CatalogTab::Skills => view.skills.get(index).map(|s| {
                    let (change, title) = if s.enabled {
                        (
                            SkillsChange::Disable(vec![s.name.clone()]),
                            format!("Quitar la skill {}", s.name),
                        )
                    } else {
                        (
                            SkillsChange::Enable(vec![s.name.clone()]),
                            format!("Habilitar la skill {}", s.name),
                        )
                    };
                    plan_skills(&path, &change).map(|p| Preview::skills(&title, p))
                }),
                CatalogTab::Packs => view.packs.get(index).map(|p| {
                    plan_skills(&path, &SkillsChange::Pack(p.name.clone()))
                        .map(|plan| Preview::skills(&format!("Habilitar el pack {}", p.name), plan))
                }),
                CatalogTab::Mods => view.mods.get(index).map(|m| {
                    let (change, title) = if m.applied {
                        (
                            ModsChange::Remove(vec![m.name.clone()]),
                            format!("Quitar el mod {}", m.name),
                        )
                    } else {
                        (
                            ModsChange::Apply(Some(vec![m.name.clone()])),
                            format!("Instalar el mod {}", m.name),
                        )
                    };
                    plan_mods(&path, &change).map(|p| Preview::mods(&title, p))
                }),
            };
            if let Some(plan) = plan {
                open(app, plan);
            }
        }
        KeyCode::Char('a') => {
            let path = app.path.clone();
            let plan = match app.catalog_tab {
                CatalogTab::Skills => {
                    let names: Vec<String> = view
                        .skills
                        .iter()
                        .filter(|s| s.recommended && !s.enabled)
                        .map(|s| s.name.clone())
                        .collect();
                    if names.is_empty() {
                        app.say(
                            Kind::Info,
                            "Ya tienes habilitadas todas las skills recomendadas.",
                        );
                        return;
                    }
                    plan_skills(&path, &SkillsChange::Enable(names))
                        .map(|p| Preview::skills("Habilitar las skills recomendadas", p))
                }
                CatalogTab::Mods => plan_mods(&path, &ModsChange::Apply(None))
                    .map(|p| Preview::mods("Instalar los mods recomendados", p)),
                CatalogTab::Packs => return,
            };
            open(app, plan);
        }
        _ => {}
    }
}
