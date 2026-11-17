//! Menu bar: dropdown menus that mirror the commands of the Studio.
use super::*;
#[path = "studio_menu_items.rs"]
mod items;
use iced::widget::{column, mouse_area, opaque, pin, stack};
use items::{Entry, entries};

/// Fixed layout so a dropdown can sit exactly under its menu button.
pub(super) const BAR_HEIGHT: f32 = 34.0;
pub(super) const BRAND_WIDTH: f32 = 112.0;
const BAR_PADDING: f32 = 12.0;
const GAP: f32 = 2.0;
const DROPDOWN_WIDTH: f32 = 290.0;
const SUBMENU_WIDTH: f32 = 200.0;
/// Height of one row of a dropdown, to line a submenu up with its entry.
const ROW_HEIGHT: f32 = 28.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Menu {
    File,
    Edit,
    View,
    Go,
    Project,
    Git,
    Help,
}

const MENUS: [Menu; 7] = [
    Menu::File,
    Menu::Edit,
    Menu::View,
    Menu::Go,
    Menu::Project,
    Menu::Git,
    Menu::Help,
];

impl Menu {
    fn label(self) -> &'static str {
        match self {
            Menu::File => "Archivo",
            Menu::Edit => "Editar",
            Menu::View => "Ver",
            Menu::Go => "Ir",
            Menu::Project => "Proyecto",
            Menu::Git => "Git",
            Menu::Help => "Ayuda",
        }
    }

    fn width(self) -> f32 {
        self.label().chars().count() as f32 * 6.6 + 18.0
    }

    fn offset(self) -> f32 {
        let before: f32 = MENUS
            .iter()
            .take_while(|menu| **menu != self)
            .map(|menu| menu.width() + GAP)
            .sum();
        BAR_PADDING + BRAND_WIDTH + 6.0 + before
    }
}

#[derive(Default)]
pub(super) struct State {
    pub open: Option<Menu>,
    /// The submenu unfolded next to the dropdown.
    pub sub: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Toggle(Menu),
    /// Moving over another menu while one is open switches to it.
    Hover(Menu),
    Close,
    Sub(&'static str),
    Pick(Box<Message>),
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Toggle(menu) => {
            state.menu.open = (state.menu.open != Some(menu)).then_some(menu);
            state.menu.sub = None;
        }
        Event::Sub(label) => {
            state.menu.sub = (state.menu.sub != Some(label)).then_some(label);
        }
        Event::Hover(menu) => {
            if state.menu.open.is_some() {
                if state.menu.open != Some(menu) {
                    state.menu.sub = None;
                }
                state.menu.open = Some(menu);
            }
        }
        Event::Close => {
            state.menu.open = None;
            state.menu.sub = None;
        }
        Event::Pick(message) => {
            state.menu.open = None;
            state.menu.sub = None;
            return Task::done(*message);
        }
    }
    Task::none()
}

pub(super) fn bar(state: &Dashboard) -> Element<'_, Message> {
    let mut bar = row![].spacing(GAP).align_y(iced::Center);
    for menu in MENUS {
        let open = state.menu.open == Some(menu);
        bar = bar.push(
            mouse_area(
                button(container(text(menu.label()).size(12)).center_x(Fill))
                    .width(menu.width())
                    .padding([4, 0])
                    .style(styles::tool(open))
                    .on_press(Message::Menu(Event::Toggle(menu))),
            )
            .on_enter(Message::Menu(Event::Hover(menu))),
        );
    }
    bar.into()
}

/// The open dropdown plus a click-catcher that closes it; `None` when no menu is open.
pub(super) fn overlay(state: &Dashboard) -> Option<Element<'_, Message>> {
    let menu = state.menu.open?;
    let items = entries(menu, state);
    let mut flyout = None;
    let mut list = column![].spacing(1);
    for (index, entry) in items.iter().enumerate() {
        if let Entry::Submenu { label, entries } = entry
            && state.menu.sub == Some(*label)
        {
            flyout = Some((index, entries));
        }
        list = list.push(entry_view(entry, state.menu.sub));
    }
    let dropdown = opaque(
        container(list.padding(4))
            .width(DROPDOWN_WIDTH)
            .style(styles::menu_panel),
    );
    let catcher = mouse_area(container(Space::new()).width(Fill).height(Fill))
        .on_press(Message::Menu(Event::Close))
        .on_right_press(Message::Menu(Event::Close));
    let mut layers = stack![catcher, pin(dropdown).x(menu.offset()).y(BAR_HEIGHT - 2.0)];
    if let Some((index, entries)) = flyout {
        let mut sub = column![].spacing(1);
        for entry in entries {
            sub = sub.push(entry_view(entry, None));
        }
        layers = layers.push(
            pin(opaque(
                container(sub.padding(4))
                    .width(SUBMENU_WIDTH)
                    .style(styles::menu_panel),
            ))
            .x(menu.offset() + DROPDOWN_WIDTH - 4.0)
            .y(BAR_HEIGHT - 2.0 + index as f32 * ROW_HEIGHT),
        );
    }
    Some(layers.into())
}

fn entry_view(entry: &Entry, open_sub: Option<&'static str>) -> Element<'static, Message> {
    match entry {
        Entry::Separator => container(Space::new())
            .width(Fill)
            .height(1)
            .style(styles::surface(design::border()))
            .into(),
        Entry::Item {
            label,
            hint,
            message,
        } => {
            let content = row![
                text(*label).size(12),
                Space::new().width(Fill),
                text(hint.clone()).size(11).color(design::faint())
            ]
            .align_y(iced::Center);
            let mut item = button(content)
                .width(Fill)
                .padding([6, 10])
                .style(styles::menu_item);
            if let Some(message) = message {
                item = item.on_press(Message::Menu(Event::Pick(Box::new(message.clone()))));
            }
            item.into()
        }
        Entry::Submenu { label, .. } => mouse_area(
            button(
                row![
                    text(*label).size(12),
                    Space::new().width(Fill),
                    icons::icon(
                        "chevron-right",
                        if open_sub == Some(*label) {
                            design::accent()
                        } else {
                            design::faint()
                        },
                        11.0
                    ),
                ]
                .align_y(iced::Center),
            )
            .width(Fill)
            .padding([6, 10])
            .style(styles::menu_item)
            .on_press(Message::Menu(Event::Sub(label))),
        )
        .on_enter(Message::Menu(Event::Sub(label)))
        .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menus_open_switch_on_hover_and_close() {
        let mut state = editor::fixture();
        let _ = update(&mut state, Event::Hover(Menu::Edit));
        assert_eq!(state.menu.open, None, "hovering alone must not open a menu");
        let _ = update(&mut state, Event::Toggle(Menu::File));
        assert_eq!(state.menu.open, Some(Menu::File));
        let _ = update(&mut state, Event::Hover(Menu::View));
        assert_eq!(state.menu.open, Some(Menu::View));
        let _ = update(&mut state, Event::Toggle(Menu::View));
        assert_eq!(state.menu.open, None, "clicking the open menu closes it");
        let _ = update(&mut state, Event::Toggle(Menu::Go));
        let _ = update(&mut state, Event::Close);
        assert_eq!(state.menu.open, None);
    }

    #[test]
    fn dropdowns_sit_under_their_buttons_and_disable_commands_that_do_not_apply() {
        let state = editor::fixture();
        let offsets: Vec<f32> = MENUS.iter().map(|menu| menu.offset()).collect();
        assert!(offsets.windows(2).all(|pair| pair[1] > pair[0]));
        for menu in MENUS {
            assert!(!entries(menu, &state).is_empty(), "{menu:?} has no entries");
        }
        // Without an open file, Save and Undo are shown but disabled.
        let disabled = |menu: Menu, wanted: &str| {
            entries(menu, &state).iter().any(|entry| {
                matches!(entry,
                Entry::Item { label, message: None, .. } if *label == wanted)
            })
        };
        assert!(disabled(Menu::File, "Guardar"));
        assert!(disabled(Menu::Edit, "Deshacer"));
    }
}
