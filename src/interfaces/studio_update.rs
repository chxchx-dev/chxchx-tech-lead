//! "Buscar actualizaciones": checks the channel this install follows and installs the newer
//! release when the person agrees. The installer keeps the running version until the next
//! update, so a failed update never leaves Studio unusable.
use super::*;
use crate::integrations::chat_bridge as bridge;
use crate::integrations::studio_update::{self as update, Info};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Phase {
    #[default]
    Closed,
    Checking,
    UpToDate(Info),
    Available(Info),
    Applying(Info),
    Done(Info),
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
    Apply,
    Applied(Result<Info, String>),
    Close,
}

/// A local build is not a managed install: say so instead of showing the CLI's internal error.
fn explain(error: String) -> String {
    if error.contains("instalación administrada") || error.contains("canal de la instalación") {
        "Esta copia de Studio no se instaló con el instalador (es una compilación local), así que no se actualiza desde aquí. Reinstálala con install.sh / install.ps1 o compila la versión nueva.".to_owned()
    } else {
        error
    }
}

fn run(
    state: &Dashboard,
    job: fn(&std::path::Path) -> Result<Info, String>,
    done: fn(Result<Info, String>) -> Event,
) -> Task<Message> {
    let root = state.root.clone();
    Task::perform(
        async move {
            jobs::offload(move || match bridge::find_cli(&root) {
                Some(cli) => job(&cli),
                None => Err(bridge::missing_cli_hint()),
            })
            .await
            .unwrap_or_else(|| Err("No se pudo consultar las actualizaciones".into()))
        },
        move |result| Message::Update(done(result)),
    )
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Open => {
            if matches!(state.update.phase, Phase::Checking | Phase::Applying(_)) {
                return Task::none();
            }
            state.update.phase = Phase::Checking;
            return run(state, update::check, Event::Checked);
        }
        Event::Checked(result) => {
            state.update.phase = match result {
                Ok(info) if info.available => Phase::Available(info),
                Ok(info) => Phase::UpToDate(info),
                Err(error) => Phase::Failed(explain(error)),
            };
        }
        Event::Apply => {
            if let Phase::Available(info) = state.update.phase.clone() {
                state.update.phase = Phase::Applying(info);
                return run(state, update::apply, Event::Applied);
            }
        }
        Event::Applied(result) => {
            state.update.phase = match result {
                Ok(info) => Phase::Done(info),
                Err(error) => Phase::Failed(explain(error)),
            };
        }
        Event::Close => {
            // An installation in progress cannot be dismissed: closing would hide it, not stop it.
            if !matches!(state.update.phase, Phase::Applying(_)) {
                state.update.phase = Phase::Closed;
            }
        }
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
                design::ACCENT,
                "Buscando actualizaciones…",
                "Consultando la versión publicada en tu canal.".into(),
                Space::new().into(),
            ),
            Phase::UpToDate(info) => (
                "check",
                design::SUCCESS,
                "Studio está al día",
                format!("Tienes la versión {}.", version(info)),
                close().into(),
            ),
            Phase::Available(info) => (
                "arrow-down",
                design::ACCENT,
                "Hay una versión nueva",
                format!(
                    "Tienes {} y está disponible {}. Se descarga, se verifica y se instala en lugar de la actual; tus proyectos y ajustes no cambian. Al terminar, reinicia Studio.",
                    version(info),
                    info.latest
                ),
                row![
                    close(),
                    button(text("Actualizar ahora").size(12))
                        .padding([7, 14])
                        .style(styles::primary)
                        .on_press(Message::Update(Event::Apply)),
                ]
                .spacing(10)
                .into(),
            ),
            Phase::Applying(info) => (
                "refresh",
                design::ACCENT,
                "Actualizando Studio…",
                format!(
                    "Instalando {}. No cierres Studio hasta que termine.",
                    info.latest
                ),
                Space::new().into(),
            ),
            Phase::Done(info) => (
                "check",
                design::SUCCESS,
                "Studio se actualizó",
                format!(
                    "Quedó instalada la versión {}. Cierra Studio y ábrelo de nuevo para usarla; la anterior se conserva hasta la próxima actualización.",
                    info.latest
                ),
                close().into(),
            ),
            Phase::Failed(error) => (
                "warning",
                design::WARNING,
                "No se pudo actualizar",
                format!("{error} La versión que tienes sigue intacta."),
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
            text(detail).size(13).color(design::MUTED),
            row![Space::new().width(Fill), buttons],
        ]
        .spacing(16),
    )
    .padding(22)
    .width(460)
    .style(styles::modal);
    Some(iced::widget::opaque(container(card).center(Fill).style(
        styles::surface(design::alpha(design::BG_DEEP, 0.72)),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_build_is_explained_not_blamed_on_the_cli() {
        let text = explain("No se detectó una instalación administrada de Studio.".into());
        assert!(text.contains("compilación local"));
        assert_eq!(explain("sin red".into()), "sin red");
    }

    #[test]
    fn an_update_in_progress_cannot_be_dismissed() {
        let mut state = super::super::editor::fixture();
        state.update.phase = Phase::Applying(Info::default());
        let _ = update(&mut state, Event::Close);
        assert!(matches!(state.update.phase, Phase::Applying(_)));
        state.update.phase = Phase::Failed("x".into());
        let _ = update(&mut state, Event::Close);
        assert_eq!(state.update.phase, Phase::Closed);
    }
}
