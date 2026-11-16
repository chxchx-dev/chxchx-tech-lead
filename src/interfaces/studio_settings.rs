//! Settings: theme, accent color, interface size and keyboard shortcuts.
//!
//! The choices live in the Studio preferences (`appearance`, `shortcuts`) and take effect as
//! soon as they are picked: `apply` loads them into the global palette and key map.
use super::design::themes;
use super::keymap::{self, Action};
use super::*;
use iced::widget::{column, text_input};

#[derive(Default)]
pub(super) struct State {
    /// The action whose new shortcut is being recorded.
    capturing: Option<Action>,
    accent_input: String,
    notice: String,
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Theme(String),
    /// `None` goes back to the accent of the theme.
    Accent(Option<String>),
    AccentInput(String),
    AccentApply,
    Scale(f32),
    Rebind(Action),
    Captured(String),
    Cancel,
    ResetKey(Action),
    ResetKeys,
}

pub(super) const SCALES: [(&str, f32); 5] = [
    ("90 %", 0.9),
    ("100 %", 1.0),
    ("110 %", 1.1),
    ("125 %", 1.25),
    ("150 %", 1.5),
];

/// Loads the saved theme and shortcuts into the live palette and key map.
pub(super) fn apply(state: &Dashboard) {
    let preferences = state.session.preferences();
    let look = &preferences.appearance;
    themes::set_palette(themes::build(&look.theme, look.accent.as_deref()));
    keymap::apply(&preferences.shortcuts);
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Theme(id) => {
            state.session.edit(|p| p.appearance.theme = id);
        }
        Event::Accent(accent) => {
            state.settings.accent_input = accent.clone().unwrap_or_default();
            state.session.edit(|p| p.appearance.accent = accent);
        }
        Event::AccentInput(value) => state.settings.accent_input = value,
        Event::AccentApply => {
            let value = state.settings.accent_input.trim().to_owned();
            let value = if value.starts_with('#') {
                value
            } else {
                format!("#{value}")
            };
            if themes::parse_hex(&value).is_some() {
                state.settings.notice.clear();
                state.session.edit(|p| p.appearance.accent = Some(value));
            } else {
                state.settings.notice = "Escribe un color como #22D3FF.".to_owned();
            }
        }
        Event::Scale(scale) => state.session.edit(|p| p.appearance.scale = scale),
        Event::Rebind(action) => {
            state.settings.capturing = Some(action);
            state.settings.notice.clear();
            keymap::set_capturing(true);
        }
        Event::Cancel => stop_capture(state),
        Event::Captured(chord) => capture(state, chord),
        Event::ResetKey(action) => {
            state.session.edit(|p| {
                p.shortcuts.remove(action.id());
            });
        }
        Event::ResetKeys => state.session.edit(|p| p.shortcuts.clear()),
    }
    apply(state);
    Task::none()
}

fn stop_capture(state: &mut Dashboard) {
    state.settings.capturing = None;
    keymap::set_capturing(false);
}

fn capture(state: &mut Dashboard, chord: String) {
    let Some(action) = state.settings.capturing else {
        return;
    };
    if !keymap::is_bindable(&chord) {
        state.settings.notice =
            "Usa Ctrl (o Alt) con otra tecla, o una tecla de función (F1…F12).".to_owned();
        return;
    }
    // A shortcut belongs to one action: the previous owner is left without one.
    let taken = keymap::owner(&chord, action);
    state.session.edit(|p| {
        if let Some(other) = taken {
            p.shortcuts.insert(other.id().to_owned(), String::new());
        }
        p.shortcuts.insert(action.id().to_owned(), chord.clone());
    });
    state.settings.notice = match taken {
        Some(other) => format!(
            "{} ya no tiene atajo: ahora lo usa «{}».",
            other.label(),
            action.label()
        ),
        None => String::new(),
    };
    stop_capture(state);
}

/// While a shortcut is being recorded, the next key press is the shortcut.
pub(super) fn subscription(state: &Dashboard) -> Option<iced::Subscription<Message>> {
    state.settings.capturing?;
    Some(iced::event::listen_with(|event, _, _| {
        let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) = event
        else {
            return None;
        };
        if matches!(
            key.as_ref(),
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
        ) {
            return Some(Message::Settings(Event::Cancel));
        }
        keymap::chord(&key, modifiers).map(|chord| Message::Settings(Event::Captured(chord)))
    }))
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let preferences = state.session.preferences();
    let look = &preferences.appearance;
    let mut body = column![
        section_title(
            "palette",
            "Apariencia",
            "Elige el tema y el color de resalte."
        ),
        themes_card(&look.theme),
        accent_card(state, look.accent.as_deref()),
        scale_card(look.scale),
        section_title(
            "keyboard",
            "Atajos de teclado",
            "Pulsa «Cambiar» y luego la combinación que quieras. Esc cancela."
        ),
    ]
    .spacing(14)
    .max_width(820);
    body = body.push(shortcuts_card(state));
    if !state.settings.notice.is_empty() {
        body = body.push(
            container(text(&state.settings.notice).size(12))
                .padding([8, 12])
                .width(Fill)
                .style(styles::tinted(design::warning())),
        );
    }
    container(styles::vscroll(
        container(body).center_x(Fill).padding([22, 28]),
    ))
    .width(Fill)
    .height(Fill)
    .style(styles::surface(design::bg_base()))
    .into()
}

fn section_title<'a>(icon: &'static str, title: &'a str, hint: &'a str) -> Element<'a, Message> {
    row![
        icons::icon(icon, design::accent(), 20.0),
        column![
            text(title).size(20).font(design::UI_BOLD),
            text(hint).size(12).color(design::muted()),
        ]
        .spacing(2),
    ]
    .spacing(12)
    .align_y(iced::Center)
    .into()
}

fn swatch<'a>(color: iced::Color, side: f32) -> Element<'a, Message> {
    container(Space::new())
        .width(side)
        .height(side)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                color: design::border_strong(),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn themes_card(current: &str) -> Element<'_, Message> {
    let mut grid = row![].spacing(10);
    for (id, name, palette) in themes::THEMES {
        let selected = *id == current;
        let preview = row![
            swatch(palette.bg_base, 22.0),
            swatch(palette.bg_raised, 22.0),
            swatch(palette.accent, 22.0),
            swatch(palette.fg, 22.0),
        ]
        .spacing(4);
        grid = grid.push(
            button(
                column![
                    preview,
                    row![
                        text(*name).size(12).font(design::UI_BOLD),
                        Space::new().width(Fill),
                        icons::icon(
                            if selected { "check" } else { "dot" },
                            if selected {
                                design::accent()
                            } else {
                                design::faint()
                            },
                            12.0
                        ),
                    ]
                    .align_y(iced::Center),
                ]
                .spacing(8),
            )
            .padding(10)
            .width(Fill)
            .style(move |theme, status| {
                if selected {
                    styles::ghost(true)(theme, status)
                } else {
                    styles::secondary(theme, status)
                }
            })
            .on_press(Message::Settings(Event::Theme((*id).to_owned()))),
        );
    }
    container(
        iced::widget::scrollable(grid).direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new()
                .width(3)
                .scroller_width(3),
        )),
    )
    .padding(14)
    .width(Fill)
    .style(styles::card)
    .into()
}

fn accent_card<'a>(state: &'a Dashboard, current: Option<&'a str>) -> Element<'a, Message> {
    let mut colors = row![
        button(text("Del tema").size(11))
            .padding([5, 10])
            .style(move |theme, status| if current.is_none() {
                styles::ghost(true)(theme, status)
            } else {
                styles::secondary(theme, status)
            })
            .on_press(Message::Settings(Event::Accent(None))),
    ]
    .spacing(8)
    .align_y(iced::Center);
    for (name, value) in themes::ACCENTS {
        let hex = format!("#{value:06X}");
        let selected = current.is_some_and(|c| c.eq_ignore_ascii_case(&hex));
        colors = colors.push(styles::tip(
            button(swatch(
                themes::hex(*value),
                if selected { 22.0 } else { 18.0 },
            ))
            .padding(3)
            .style(styles::ghost(selected))
            .on_press(Message::Settings(Event::Accent(Some(hex)))),
            name,
            iced::widget::tooltip::Position::Top,
        ));
    }
    let custom = row![
        text_input("#22D3FF", &state.settings.accent_input)
            .on_input(|value| Message::Settings(Event::AccentInput(value)))
            .on_submit(Message::Settings(Event::AccentApply))
            .size(12)
            .padding([6, 10])
            .width(110)
            .style(styles::input),
        button(text("Aplicar").size(12))
            .padding([6, 12])
            .style(styles::secondary)
            .on_press(Message::Settings(Event::AccentApply)),
    ]
    .spacing(8)
    .align_y(iced::Center);
    container(
        column![
            text("Color de resalte").size(13).font(design::UI_BOLD),
            colors,
            custom
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Fill)
    .style(styles::card)
    .into()
}

fn scale_card<'a>(current: f32) -> Element<'a, Message> {
    let mut sizes = row![].spacing(8);
    for (label, value) in SCALES {
        let selected = (current - value).abs() < 0.01;
        sizes = sizes.push(
            button(text(label).size(12))
                .padding([6, 12])
                .style(move |theme, status| {
                    if selected {
                        styles::ghost(true)(theme, status)
                    } else {
                        styles::secondary(theme, status)
                    }
                })
                .on_press(Message::Settings(Event::Scale(value))),
        );
    }
    container(
        column![
            text("Tamaño de la interfaz").size(13).font(design::UI_BOLD),
            sizes
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Fill)
    .style(styles::card)
    .into()
}

fn shortcuts_card(state: &Dashboard) -> Element<'_, Message> {
    let changed = &state.session.preferences().shortcuts;
    let mut list = column![].spacing(6);
    for action in Action::ALL {
        let recording = state.settings.capturing == Some(action);
        let binding = keymap::binding(action);
        let chip: Element<'_, Message> = if recording {
            text("Pulsa las teclas…")
                .size(12)
                .color(design::accent())
                .into()
        } else {
            container(text(keymap::display(&binding)).size(12).font(design::MONO))
                .padding([4, 10])
                .style(styles::chip_surface(false))
                .into()
        };
        let mut line = row![
            text(action.label()).size(13).width(Fill),
            chip,
            button(text(if recording { "Cancelar" } else { "Cambiar" }).size(11))
                .padding([5, 10])
                .style(styles::secondary)
                .on_press(Message::Settings(if recording {
                    Event::Cancel
                } else {
                    Event::Rebind(action)
                })),
        ]
        .spacing(10)
        .align_y(iced::Center);
        if changed.contains_key(action.id()) {
            line = line.push(styles::tip(
                button(icons::icon("refresh", design::muted(), 12.0))
                    .padding(5)
                    .style(styles::ghost(false))
                    .on_press(Message::Settings(Event::ResetKey(action))),
                "Volver al atajo original",
                iced::widget::tooltip::Position::Left,
            ));
        }
        list = list.push(line);
    }
    list = list.push(
        button(text("Restaurar todos los atajos").size(12))
            .padding([6, 12])
            .style(styles::secondary)
            .on_press(Message::Settings(Event::ResetKeys)),
    );
    container(list)
        .padding(14)
        .width(Fill)
        .style(styles::card)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picking_a_theme_and_an_accent_repaints_and_is_remembered() {
        let mut state = super::super::editor::fixture();
        let _ = update(&mut state, Event::Theme("light".into()));
        let _ = update(&mut state, Event::Accent(Some("#FF0000".into())));
        let saved = &state.session.preferences().appearance;
        assert_eq!(saved.theme, "light");
        assert_eq!(saved.accent.as_deref(), Some("#FF0000"));
        assert_eq!(design::accent(), themes::hex(0xFF0000));
        assert_eq!(design::bg_base(), themes::build("light", None).bg_base);
        state.settings.accent_input = "nonsense".into();
        let _ = update(&mut state, Event::AccentApply);
        assert!(!state.settings.notice.is_empty());
        let _ = update(&mut state, Event::Theme("chxchx".into()));
        let _ = update(&mut state, Event::Accent(None));
    }
}
