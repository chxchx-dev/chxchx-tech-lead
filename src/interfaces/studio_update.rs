//! "Buscar actualizaciones": tells the person when a newer release is published and opens its
//! page. Nothing is installed from here, so a failed check never touches the running Studio.
use super::*;
use crate::integrations::studio_update::{self as update, Info};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Phase {
    #[default]
    Closed,
    Checking,
    UpToDate(Info),
    Available(Info),
    Failed(String),
}

#[derive(Default)]
pub(super) struct State {
    pub phase: Phase,
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Open,
    Checked(Result<Info, String>),
    /// "Ver la descarga": opens the page of the new release.
    Download,
    Opened(Result<(), String>),
    Close,
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Open => {
            if matches!(state.update.phase, Phase::Checking) {
                return Task::none();
            }
            state.update.phase = Phase::Checking;
            return Task::perform(
                async {
                    jobs::offload(update::check)
                        .await
                        .unwrap_or_else(|| Err("No se pudo consultar las actualizaciones".into()))
                },
                |result| Message::Update(Event::Checked(result)),
            );
        }
        Event::Checked(result) => {
            state.update.phase = match result {
                Ok(info) if info.available => Phase::Available(info),
                Ok(info) => Phase::UpToDate(info),
                Err(error) => Phase::Failed(error),
            };
        }
        Event::Download => {
            if let Phase::Available(info) = state.update.phase.clone() {
                return Task::perform(
                    async move {
                        jobs::offload(move || update::open_download(&info))
                            .await
                            .unwrap_or_else(|| Err("No se pudo abrir la descarga".into()))
                    },
                    |result| Message::Update(Event::Opened(result)),
                );
            }
        }
        Event::Opened(result) => {
            state.update.phase = match result {
                Ok(()) => Phase::Closed,
                Err(error) => Phase::Failed(error),
            };
        }
        Event::Close => state.update.phase = Phase::Closed,
    }
    Task::none()
}

fn version(info: &Info) -> String {
    if info.current.is_empty() {
        "desconocida".to_owned()
    } else {
        info.current.clone()
    }
}

/// The dialog for the current phase, or nothing while closed.
pub(super) fn dialog(state: &Dashboard) -> Option<Element<'_, Message>> {
    let close = || {
        button(text("Cerrar").size(12))
            .padding([7, 14])
            .style(styles::secondary)
            .on_press(Message::Update(Event::Close))
    };
    let (icon, tint, title, detail, buttons): (_, _, &str, String, Element<'_, Message>) =
        match &state.update.phase {
            Phase::Closed => return None,
            Phase::Checking => (
                "refresh",
                design::accent(),
                "Buscando actualizaciones…",
                "Consultando el último release publicado.".into(),
                Space::new().into(),
            ),
            Phase::UpToDate(info) => (
                "check",
                design::success(),
                "Studio está al día",
                format!("Tienes la versión {}.", version(info)),
                close().into(),
            ),
            Phase::Available(info) => (
                "arrow-down",
                design::accent(),
                "Hay una versión nueva",
                format!(
                    "Tienes {} y está disponible {}. Descárgala de la página del release e instálala encima de la actual; tus proyectos y ajustes no cambian.",
                    version(info),
                    info.latest
                ),
                row![
                    close(),
                    button(text("Ver la descarga").size(12))
                        .padding([7, 14])
                        .style(styles::primary)
                        .on_press(Message::Update(Event::Download)),
                ]
                .spacing(10)
                .into(),
            ),
            Phase::Failed(error) => (
                "warning",
                design::warning(),
                "No se pudo comprobar",
                format!("{error} Tu versión sigue intacta."),
                close().into(),
            ),
        };
    let card = container(
        iced::widget::column![
            row![
                icons::icon(icon, tint, 20.0),
                text(title).size(16).font(design::UI_BOLD)
            ]
            .spacing(10)
            .align_y(iced::Center),
            text(detail).size(13).color(design::muted()),
            row![Space::new().width(Fill), buttons],
        ]
        .spacing(16),
    )
    .padding(22)
    .width(460)
    .style(styles::modal);
    Some(iced::widget::opaque(container(card).center(Fill).style(
        styles::surface(design::alpha(design::bg_deep(), 0.72)),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn newer() -> Info {
        Info {
            current: "0.6.3".into(),
            latest: "0.7.0".into(),
            available: true,
            url: "https://github.com/chxchx-dev-hp/x/releases/tag/v0.7.0".into(),
        }
    }

    #[test]
    fn the_download_is_only_requested_when_a_newer_release_was_found() {
        let mut state = super::super::editor::fixture();
        state.update.phase = Phase::UpToDate(newer());
        let _ = update(&mut state, Event::Download);
        assert!(
            matches!(state.update.phase, Phase::UpToDate(_)),
            "nothing to download"
        );
        state.update.phase = Phase::Available(newer());
        let _ = update(&mut state, Event::Download);
        assert!(
            matches!(state.update.phase, Phase::Available(_)),
            "the dialog stays until the page opened"
        );
    }

    #[test]
    fn a_page_that_could_not_open_is_reported_and_closing_always_works() {
        let mut state = super::super::editor::fixture();
        state.update.phase = Phase::Available(newer());
        let _ = update(&mut state, Event::Opened(Err("sin navegador".into())));
        assert_eq!(state.update.phase, Phase::Failed("sin navegador".into()));
        let _ = update(&mut state, Event::Close);
        assert_eq!(state.update.phase, Phase::Closed);
    }
}
