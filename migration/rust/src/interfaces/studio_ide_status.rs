//! Status bar: brand segment, project trust, file and cursor details.
use super::*;

const HEIGHT: f32 = 24.0;

fn segment<'a>(icon: &'static str, tint: iced::Color, label: String) -> Element<'a, Message> {
    row![
        icons::icon(icon, tint, 12.0),
        text(label).size(11).color(design::MUTED)
    ]
    .spacing(5)
    .align_y(iced::Center)
    .into()
}

fn label<'a>(value: impl text::IntoFragment<'a>) -> Element<'a, Message> {
    text(value).size(11).color(design::MUTED).into()
}

pub(super) fn status_bar(state: &Dashboard) -> Element<'_, Message> {
    let overview = state.overview.as_ref().ok();
    let trusted = overview.is_some_and(|project| project.workspace.trusted);
    let brand = container(
        row![
            brand::mark(13.0),
            text("ChxChx").size(11).font(design::UI_BOLD)
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .padding([0, 10])
    .height(HEIGHT)
    .align_y(iced::Center)
    .style(styles::brand_fill);
    let mut bar = row![brand].spacing(14).align_y(iced::Center);
    if overview.is_some_and(|project| project.project.git_repository) {
        let label = match &state.git.status {
            Some(status) => {
                let mut label = status.head.clone();
                if status.ahead > 0 {
                    label.push_str(&format!(" ↑{}", status.ahead));
                }
                if status.behind > 0 {
                    label.push_str(&format!(" ↓{}", status.behind));
                }
                match state.git.change_count() {
                    0 => label,
                    count => format!("{label} · {count} cambios"),
                }
            }
            None => "Git".to_owned(),
        };
        bar = bar.push(segment("branch", design::MUTED, label));
    }
    if state.pending > 0 {
        bar = bar.push(segment("refresh", design::ACCENT, "Procesando…".into()));
    }
    bar = bar.push(if trusted {
        segment("shield", design::SUCCESS, "Proyecto de confianza".into())
    } else {
        segment("warning", design::WARNING, "Permisos por revisar".into())
    });
    let mut details = row![].spacing(14).align_y(iced::Center);
    if let Some(tab) = tabs::selected_tab(state) {
        let cursor = tab.content.cursor().position;
        if tab.dirty {
            details = details.push(segment("dot", design::WARNING, "Sin guardar".into()));
        }
        details = details
            .push(label(format!(
                "Ln {}, Col {}",
                cursor.line + 1,
                cursor.column + 1
            )))
            .push(label(format!("{} líneas", tab.content.line_count())))
            .push(label("UTF-8"))
            .push(label(file_types::of(&tab.document.relative).language));
    } else {
        details = details.push(label("Espacio de trabajo"));
    }
    details = details.push(label(concat!("v", env!("CARGO_PKG_VERSION"))));
    let bar = bar.push(container(details).align_right(Fill).padding([0, 12]));
    container(bar)
        .width(Fill)
        .height(HEIGHT)
        .style(styles::surface(design::BG_DEEP))
        .into()
}
