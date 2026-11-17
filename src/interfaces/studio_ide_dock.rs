//! The activity log: the latest operation message.
use super::*;

pub(super) fn activity(state: &Dashboard) -> Element<'_, Message> {
    let content: Element<'_, Message> = if state.message.is_empty() {
        text("Sin operaciones recientes.")
            .size(12)
            .color(design::faint())
            .into()
    } else {
        row![
            icons::icon("info", design::accent(), 14.0),
            text(&state.message).size(12)
        ]
        .spacing(8)
        .into()
    };
    container(styles::vscroll(container(content).padding(12).width(Fill)).height(Fill))
        .padding([0, 4])
        .width(Fill)
        .height(Fill)
        .into()
}
