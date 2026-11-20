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
                text(label.to_string()).size(12),
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
                Entry::Item { label, message: None, .. } if label == wanted)
            })
        };
        assert!(disabled(Menu::File, "Guardar"));
        assert!(disabled(Menu::Edit, "Deshacer"));
    }

    fn labels(menu: Menu, state: &Dashboard) -> Vec<String> {
        entries(menu, state)
            .iter()
            .filter_map(|entry| match entry {
                Entry::Item { label, .. } => Some(label.to_string()),
                Entry::Submenu { label, .. } => Some((*label).to_owned()),
                Entry::Separator => None,
            })
            .collect()
    }

    fn message_of(menu: Menu, state: &Dashboard, wanted: &str) -> Option<Message> {
        entries(menu, state)
            .into_iter()
            .find_map(|entry| match entry {
                Entry::Item { label, message, .. } if label == wanted => message,
                _ => None,
            })
    }

    #[test]
    fn the_file_menu_opens_projects_and_creates_and_saves_like_an_editor() {
        let state = editor::fixture();
        let file = labels(Menu::File, &state);
        for wanted in [
            "Nuevo archivo",
            "Nueva carpeta",
            "Abrir proyecto…",
            "Abrir proyecto en ventana nueva…",
            "Abrir reciente",
            "Guardar",
            "Guardar todo",
            "Cerrar todas las pestañas",
            "Copiar la ruta del archivo",
            "Mostrar en el explorador de archivos",
        ] {
            assert!(
                file.iter().any(|label| label == wanted),
                "falta {wanted}: {file:?}"
            );
        }
        assert!(matches!(
            message_of(Menu::File, &state, "Abrir proyecto…"),
            Some(Message::Session(session::Event::PickProject {
                new_window: false
            }))
        ));
        assert!(matches!(
            message_of(Menu::File, &state, "Abrir proyecto en ventana nueva…"),
            Some(Message::Session(session::Event::PickProject {
                new_window: true
            }))
        ));
    }

    #[test]
    fn what_needs_an_open_file_is_disabled_without_one_and_enabled_with_one() {
        let mut state = editor::fixture();
        for wanted in [
            "Cortar",
            "Copiar",
            "Pegar",
            "Seleccionar todo",
            "Copiar la ruta del archivo",
            "Guardar todo",
            "Cerrar todas las pestañas",
        ] {
            let menu = if wanted.contains("ruta")
                || wanted.contains("Guardar")
                || wanted.contains("Cerrar")
            {
                Menu::File
            } else {
                Menu::Edit
            };
            assert!(
                message_of(menu, &state, wanted).is_none(),
                "{wanted} should be disabled"
            );
        }
        let document = crate::features::project_files::Document {
            relative: "src/a.rs".into(),
            text: "x".into(),
            fingerprint: vec![],
        };
        let _ = editor::update(&mut state, editor::Event::Loaded(Ok(document)));
        for wanted in ["Cortar", "Copiar", "Pegar", "Seleccionar todo"] {
            assert!(message_of(Menu::Edit, &state, wanted).is_some(), "{wanted}");
        }
        assert!(message_of(Menu::File, &state, "Copiar la ruta del archivo").is_some());
        assert!(message_of(Menu::File, &state, "Cerrar todas las pestañas").is_some());
        assert!(
            message_of(Menu::File, &state, "Guardar todo").is_none(),
            "nothing unsaved yet"
        );
        state.editor.tabs[0].dirty = true;
        assert!(message_of(Menu::File, &state, "Guardar todo").is_some());
    }

    #[test]
    fn recent_projects_list_the_others_that_still_exist_and_open_in_place_of_this_one() {
        let base = std::env::temp_dir().join(format!("chxchx-recent-{}", std::process::id()));
        let (here, other) = (base.join("aqui"), base.join("otro"));
        std::fs::create_dir_all(&here).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        let mut state = editor::fixture();
        state.root = here.clone();
        state.session.edit(|p| {
            p.recent_projects = vec![
                here.display().to_string(),
                other.display().to_string(),
                base.join("borrado").display().to_string(),
            ];
        });

        let recent = entries(Menu::File, &state)
            .into_iter()
            .find_map(|entry| match entry {
                Entry::Submenu {
                    label: "Abrir reciente",
                    entries,
                } => Some(entries),
                _ => None,
            })
            .expect("a recent submenu");
        assert_eq!(
            recent.len(),
            1,
            "this project and the vanished one are left out"
        );
        assert!(matches!(
            &recent[0],
            Entry::Item { message: Some(Message::Session(session::Event::OpenFolder { path, new_window: false })), .. }
                if *path == other
        ));
    }

    #[test]
    fn without_recent_projects_the_submenu_says_so() {
        let state = editor::fixture();
        let recent = entries(Menu::File, &state)
            .into_iter()
            .find_map(|entry| match entry {
                Entry::Submenu { entries, .. } => Some(entries),
                _ => None,
            })
            .unwrap();
        assert!(recent.iter().any(|e| matches!(e, Entry::Item { label, message: None, .. } if label == "Ningún proyecto reciente")));
    }

    #[test]
    fn the_view_menu_zooms_and_goes_full_screen() {
        let state = editor::fixture();
        for wanted in ["Acercar", "Alejar", "Tamaño normal", "Pantalla completa"] {
            assert!(message_of(Menu::View, &state, wanted).is_some(), "{wanted}");
        }
    }
}
