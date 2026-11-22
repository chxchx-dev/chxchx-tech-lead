//! Settings: theme, accent color, interface size and keyboard shortcuts.
//!
//! The choices live in the Studio preferences (`appearance`, `shortcuts`) and take effect as
//! soon as they are picked: `apply` loads them into the global palette and key map.
use super::design::themes;
use super::keymap::{self, Action};
use super::*;
use iced::widget::{column, text_input};

#[path = "studio_settings_connection.rs"]
pub(super) mod connection;
#[path = "studio_settings_identity.rs"]
pub(super) mod identity;
#[path = "studio_settings_node.rs"]
pub(super) mod node;

#[derive(Default)]
pub(super) struct State {
    /// Settings → Nodo local.
    node: node::State,
    /// Settings → Identidad.
    identity: identity::State,
    connection: connection::State,
    /// The action whose new shortcut is being recorded.
    capturing: Option<Action>,
    accent_input: String,
    accent2_input: String,
    notice: String,
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Theme(String),
    /// `None` goes back to the accent of the theme.
    Accent(Option<String>),
    AccentInput(String),
    AccentApply,
    /// A two-color gradient: sets the accent and the second color together.
    Gradient(String, String),
    /// `None` goes back to the second color derived from the accent.
    Accent2(Option<String>),
    Accent2Input(String),
    Accent2Apply,
    Scale(f32),
    /// Zoom in (+1), out (-1) or back to 100 % (0).
    Zoom(i32),
    Rebind(Action),
    Captured(String),
    Cancel,
    ResetKey(Action),
    ResetKeys,
    Node(node::Event),
    Identity(identity::Event),
    Connection(connection::Event),
}

/// The next interface size up (`step` > 0) or down, within the sizes Settings offers.
pub(super) fn stepped_scale(current: f32, step: i32) -> f32 {
    let index = SCALES
        .iter()
        .position(|(_, value)| (current - value).abs() < 0.01)
        .unwrap_or(1);
    let moved = (index as i32 + step).clamp(0, SCALES.len() as i32 - 1) as usize;
    SCALES[moved].1
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
    themes::set_palette(themes::build(
        &look.theme,
        look.accent.as_deref(),
        look.accent2.as_deref(),
    ));
    brand::set_second_color(look.accent2.as_deref());
    keymap::apply(&preferences.shortcuts);
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Node(event) => return node::update(state, event),
        Event::Identity(event) => return identity::update(state, event),
        Event::Connection(event) => return connection::update(state, event),
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
        Event::Gradient(first, second) => {
            state.settings.accent_input = first.clone();
            state.settings.accent2_input = second.clone();
            state.settings.notice.clear();
            state.session.edit(|p| {
                p.appearance.accent = Some(first);
                p.appearance.accent2 = Some(second);
            });
        }
        Event::Accent2(second) => {
            state.settings.accent2_input = second.clone().unwrap_or_default();
            state.session.edit(|p| p.appearance.accent2 = second);
        }
        Event::Accent2Input(value) => state.settings.accent2_input = value,
        Event::Accent2Apply => {
            let value = state.settings.accent2_input.trim().to_owned();
            let value = if value.starts_with('#') {
                value
            } else {
                format!("#{value}")
            };
            if themes::parse_hex(&value).is_some() {
                state.settings.notice.clear();
                state.session.edit(|p| p.appearance.accent2 = Some(value));
            } else {
                state.settings.notice = "Escribe el segundo color como #2A6BFF.".to_owned();
            }
        }
        Event::Scale(scale) => state.session.edit(|p| p.appearance.scale = scale),
        Event::Zoom(step) => {
            let current = state.session.preferences().appearance.scale;
            let next = if step == 0 {
                1.0
            } else {
                stepped_scale(current, step)
            };
            state.session.edit(|p| p.appearance.scale = next);
        }
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
    let refresh = (state.page == Page::Settings).then(|| {
        iced::time::every(std::time::Duration::from_secs(4))
            .map(|_| Message::Settings(Event::Node(node::Event::Refresh)))
    });
    let keys = state.settings.capturing.map(|_| capture_keys());
    match (refresh, keys) {
        (Some(refresh), Some(keys)) => Some(iced::Subscription::batch([refresh, keys])),
        (one, other) => one.or(other),
    }
}

fn capture_keys() -> iced::Subscription<Message> {
    iced::event::listen_with(|event, _, _| {
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
    })
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
        gradient_card(state, look.accent.as_deref(), look.accent2.as_deref()),
        scale_card(look.scale),
        section_title(
            "sparkle",
            "Identidad",
            "Tu nombre, tu lema y tu imagen: se ven aquí y en la app web de tu teléfono."
        ),
        identity::card(state),
        section_title(
            "integrations",
            "Conexión",
            "Cómo llega esta instalación al mundo: puertos, origen de las versiones y rutas de herramientas."
        ),
        connection::card(state),
        section_title(
            "keyboard",
            "Atajos de teclado",
            "Pulsa «Cambiar» y luego la combinación que quieras. Esc cancela."
        ),
    ]
    .spacing(14)
    .max_width(820);
    body = body.push(shortcuts_card(state));
    body = body.push(section_title(
        "cpu",
        "Nodo local",
        "Corre tus agentes aunque cierres Studio y los sigue en tu teléfono.",
    ));
    body = body.push(node::card(state));
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
            *name,
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

/// Two-color gradients: the name, the buttons and the messages fade between the two colors, here and
/// on the phone.
fn gradient_card<'a>(
    state: &'a Dashboard,
    accent: Option<&'a str>,
    accent2: Option<&'a str>,
) -> Element<'a, Message> {
    let mut pairs = row![].spacing(10);
    for (_, name, first, second) in chxchx_protocol::COLOR_PAIRS {
        let selected = accent.is_some_and(|c| c.eq_ignore_ascii_case(first))
            && accent2.is_some_and(|c| c.eq_ignore_ascii_case(second));
        let color = |value: &str| themes::parse_hex(value).unwrap_or(iced::Color::WHITE);
        pairs = pairs.push(styles::tip(
            button(row![swatch(color(first), 16.0), swatch(color(second), 16.0)].spacing(2))
                .padding(4)
                .style(styles::ghost(selected))
                .on_press(Message::Settings(Event::Gradient(
                    (*first).to_owned(),
                    (*second).to_owned(),
                ))),
            *name,
            iced::widget::tooltip::Position::Top,
        ));
    }
    let custom = row![
        text("Segundo color").size(12).color(design::muted()),
        text_input("#2A6BFF", &state.settings.accent2_input)
            .on_input(|value| Message::Settings(Event::Accent2Input(value)))
            .on_submit(Message::Settings(Event::Accent2Apply))
            .size(12)
            .padding([6, 10])
            .width(110)
            .style(styles::input),
        button(text("Aplicar").size(12))
            .padding([6, 12])
            .style(styles::secondary)
            .on_press(Message::Settings(Event::Accent2Apply)),
        button(text("Quitar").size(12))
            .padding([6, 12])
            .style(styles::ghost(false))
            .on_press(Message::Settings(Event::Accent2(None))),
    ]
    .spacing(8)
    .align_y(iced::Center);
    container(
        column![
            text("Degradado de dos colores").size(13).font(design::UI_BOLD),
            text("El nombre, los botones y tus mensajes pasan de un color al otro, aquí y en el teléfono.")
                .size(11)
                .color(design::muted()),
            pairs,
            custom,
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
    #[test]
    fn a_gradient_sets_both_colors_and_the_second_one_can_be_typed_or_removed() {
        let mut state = super::super::editor::fixture();
        let _ = update(
            &mut state,
            Event::Gradient("#fb923c".into(), "#f472b6".into()),
        );
        let look = &state.session.preferences().appearance;
        assert_eq!(
            (look.accent.as_deref(), look.accent2.as_deref()),
            (Some("#fb923c"), Some("#f472b6"))
        );
        assert_eq!(
            design::accent(),
            themes::hex(0xFB923C),
            "the palette in use follows"
        );

        let _ = update(&mut state, Event::Accent2Input("2a6bff".into()));
        let _ = update(&mut state, Event::Accent2Apply);
        assert_eq!(
            state.session.preferences().appearance.accent2.as_deref(),
            Some("#2a6bff")
        );
        let _ = update(&mut state, Event::Accent2Input("rojo".into()));
        let _ = update(&mut state, Event::Accent2Apply);
        assert!(state.settings.notice.contains("segundo color"));
        assert_eq!(
            state.session.preferences().appearance.accent2.as_deref(),
            Some("#2a6bff"),
            "a bad one changes nothing"
        );

        let _ = update(&mut state, Event::Accent2(None));
        assert_eq!(state.session.preferences().appearance.accent2, None);
    }

    #[test]
    fn every_ready_made_gradient_is_drawn_in_the_card() {
        let state = super::super::editor::fixture();
        let _ = gradient_card(&state, None, None);
        for (_, _, first, second) in chxchx_protocol::COLOR_PAIRS {
            let _ = gradient_card(&state, Some(first), Some(second));
        }
    }

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
        assert_eq!(
            design::bg_base(),
            themes::build("light", None, None).bg_base
        );
        state.settings.accent_input = "nonsense".into();
        let _ = update(&mut state, Event::AccentApply);
        assert!(!state.settings.notice.is_empty());
        let _ = update(&mut state, Event::Theme("chxchx".into()));
        let _ = update(&mut state, Event::Accent(None));
    }

    #[test]
    fn zooming_steps_through_the_offered_sizes_and_stops_at_the_ends() {
        assert_eq!(stepped_scale(1.0, 1), 1.1);
        assert_eq!(stepped_scale(1.0, -1), 0.9);
        assert_eq!(stepped_scale(0.9, -1), 0.9);
        assert_eq!(stepped_scale(1.5, 1), 1.5);
        assert_eq!(stepped_scale(1.25, 1), 1.5);
        // A size that is not on the list counts as 100 %.
        assert_eq!(stepped_scale(1.07, 1), 1.1);
    }

    #[test]
    fn the_zoom_event_changes_the_saved_size_and_zero_restores_it() {
        let mut state = super::super::editor::fixture();
        let _ = update(&mut state, Event::Zoom(1));
        let _ = update(&mut state, Event::Zoom(1));
        assert_eq!(state.session.preferences().appearance.scale, 1.25);
        let _ = update(&mut state, Event::Zoom(-1));
        assert_eq!(state.session.preferences().appearance.scale, 1.1);
        let _ = update(&mut state, Event::Zoom(0));
        assert_eq!(state.session.preferences().appearance.scale, 1.0);
    }
}
