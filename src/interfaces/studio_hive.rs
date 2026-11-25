//! Colmena in the sidebar (ADR 0018). Only reachable while the extension is on.
//!
//! C1 puts the extension in place — switch, database, icon. The board, decisions and roadmap views
//! arrive in the next steps of the plan; this panel says so instead of pretending.
use super::*;
use crate::features::extensions;
use iced::widget::column;

/// Whether the rail shows Colmena: the extension must be compiled in and turned on.
pub(super) fn visible(state: &Dashboard) -> bool {
    state
        .session
        .preferences()
        .extension_enabled(extensions::HIVE)
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let body = column![
        row![
            icons::icon("hive", design::accent(), 18.0),
            text("Colmena").size(15).font(design::UI_BOLD),
        ]
        .spacing(8)
        .align_y(iced::Center),
        text("Tu tablero vivo: lo que planean los agentes, lo que va quedando hecho y las decisiones que esperan por ti.")
            .size(12)
            .color(design::muted()),
        container(
            column![
                text("Lista para usarse").size(12).font(design::UI_BOLD),
                text("La base de este proyecto ya existe. El tablero, las decisiones y el roadmap llegan en los próximos pasos.")
                    .size(11)
                    .color(design::muted()),
            ]
            .spacing(4),
        )
        .padding(12)
        .width(Fill)
        .style(styles::card),
        button(text("Extensiones en Ajustes").size(11))
            .padding([5, 10])
            .style(styles::secondary)
            .on_press(Message::Navigate(Page::Settings)),
    ]
    .spacing(10);
    container(
        column![
            row![
                text("COLMENA")
                    .size(10)
                    .font(design::UI_BOLD)
                    .color(design::faint()),
            ]
            .padding([0, 12]),
            styles::vscroll(container(body).padding([0, 12]).width(Fill)).height(Fill),
        ]
        .spacing(8),
    )
    .padding([10, 0])
    .width(state.shell.explorer_width)
    .height(Fill)
    .style(styles::surface(design::bg_panel()))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::studio_preferences::ExtensionPrefs;

    #[test]
    fn it_is_hidden_until_the_extension_is_on() {
        let mut state = super::super::editor::fixture();
        assert!(!visible(&state));
        state.session.edit(|preferences| {
            preferences.extensions.insert(
                extensions::HIVE.into(),
                ExtensionPrefs {
                    enabled: true,
                    accepted_at: None,
                },
            );
        });
        assert_eq!(visible(&state), cfg!(feature = "hive"));
        let _ = view(&state);
    }
}
