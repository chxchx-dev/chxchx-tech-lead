//! Settings → Conexión: how this installation reaches the world, chosen by the person: the node's
//! port, the HTTPS port of Tailscale, where updates come from (and the public key they must carry)
//! and where each tool lives. Empty means the product's own value. Nothing is applied without
//! pressing «Aplicar» and the old file stays as `.bak`; «Valores de fábrica» goes back.
use super::*;
use crate::features::connection::{self, Connection};
use iced::widget::{column, text_input};

#[derive(Default)]
pub(super) struct State {
    loaded: bool,
    /// What the person is typing, not saved yet. Empty means the product's value.
    node_port: String,
    https_port: String,
    owner: String,
    repository: String,
    key: String,
    tools: Vec<String>,
    saved: Connection,
    notice: String,
    failed: bool,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::interfaces::studio) enum Field {
    NodePort,
    HttpsPort,
    Owner,
    Repository,
    Key,
    Tool(usize),
}

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) enum Event {
    Load,
    Type(Field, String),
    Apply,
    Reset,
}

fn texts(saved: &Connection) -> State {
    State {
        loaded: true,
        node_port: saved.node_port.map(|v| v.to_string()).unwrap_or_default(),
        https_port: saved.https_port.map(|v| v.to_string()).unwrap_or_default(),
        owner: saved.update_owner.clone().unwrap_or_default(),
        repository: saved.update_repository.clone().unwrap_or_default(),
        key: saved.update_key.clone().unwrap_or_default(),
        tools: connection::TOOLS
            .iter()
            .map(|name| saved.tools.get(*name).cloned().unwrap_or_default())
            .collect(),
        saved: saved.clone(),
        notice: String::new(),
        failed: false,
    }
}

fn reload(state: &mut Dashboard) {
    let Some(home) = super::identity::home() else {
        return;
    };
    state.settings.connection = texts(&connection::load_in(&home));
}

fn port(text: &str, what: &str) -> Result<Option<u16>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<u16>()
        .map(Some)
        .map_err(|_| format!("{what}: un número entre 1 y 65535 (vacío es el de fábrica)."))
}

fn some(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// What the boxes say, or why it cannot be saved yet.
fn draft(own: &State) -> Result<Connection, String> {
    let mut mine = Connection {
        node_port: port(&own.node_port, "Puerto del nodo")?,
        https_port: port(&own.https_port, "Puerto HTTPS de Tailscale")?,
        update_owner: some(&own.owner),
        update_repository: some(&own.repository),
        update_key: some(&own.key),
        ..Default::default()
    };
    for (name, path) in connection::TOOLS.iter().zip(&own.tools) {
        if let Some(path) = some(path) {
            mine.tools.insert((*name).to_owned(), path);
        }
    }
    connection::check(&mine)?;
    Ok(mine)
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    let say = |state: &mut Dashboard, text: String, failed: bool| {
        state.settings.connection.notice = text;
        state.settings.connection.failed = failed;
    };
    match event {
        Event::Load => {
            if !state.settings.connection.loaded {
                reload(state);
            }
        }
        Event::Type(field, value) => {
            let own = &mut state.settings.connection;
            let value: String = value.chars().take(300).collect();
            match field {
                Field::NodePort => own.node_port = value,
                Field::HttpsPort => own.https_port = value,
                Field::Owner => own.owner = value,
                Field::Repository => own.repository = value,
                Field::Key => own.key = value,
                Field::Tool(index) => {
                    if let Some(slot) = own.tools.get_mut(index) {
                        *slot = value;
                    }
                }
            }
            own.notice.clear();
        }
        Event::Apply => {
            let Some(home) = super::identity::home() else {
                say(
                    state,
                    "No se pudo resolver la carpeta de ChxChx".into(),
                    true,
                );
                return Task::none();
            };
            match draft(&state.settings.connection)
                .and_then(|mine| connection::save_in(&home, &mine))
            {
                Ok(()) => {
                    reload(state);
                    say(
                        state,
                        "Guardado. El puerto del nodo se aplica la próxima vez que se inicie; lo demás, ya.".into(),
                        false,
                    );
                }
                Err(error) => say(state, error, true),
            }
        }
        Event::Reset => {
            let result = super::identity::home()
                .ok_or("No se pudo resolver la carpeta de ChxChx".to_owned())
                .and_then(|home| connection::reset_in(&home));
            match result {
                Ok(()) => {
                    reload(state);
                    say(
                        state,
                        "Valores de fábrica (lo tuyo quedó como .bak).".into(),
                        false,
                    );
                }
                Err(error) => say(state, error, true),
            }
        }
    }
    Task::none()
}

fn send(event: Event) -> Message {
    Message::Settings(settings::Event::Connection(event))
}

fn hint<'a>(content: impl Into<String>, warning: bool) -> Element<'a, Message> {
    text(content.into())
        .size(11)
        .color(if warning {
            design::warning()
        } else {
            design::muted()
        })
        .into()
}

fn row_of<'a>(
    label: &'a str,
    placeholder: &str,
    value: &str,
    field: Field,
    width: impl Into<iced::Length>,
) -> Element<'a, Message> {
    row![
        text(label).size(12).font(design::UI_BOLD).width(150),
        text_input(placeholder, value)
            .on_input(move |value| send(Event::Type(field, value)))
            .on_submit(send(Event::Apply))
            .size(12)
            .padding([6, 10])
            .width(width)
            .style(styles::input),
    ]
    .spacing(10)
    .align_y(iced::Center)
    .into()
}

/// Tailscale and the node, as they are now (read only: changed with the boxes below or with
/// `chxchx-tech node remote enable`).
fn status() -> String {
    #[cfg(feature = "node")]
    {
        use crate::features::node_state;
        let cli = crate::integrations::tailscale::locate();
        let remote = node_state::dir().and_then(|dir| node_state::read_remote(&dir));
        let tailscale = cli.map_or_else(
            || "Tailscale: no se encontró (pon su ruta abajo si no está en el PATH).".to_owned(),
            |path| format!("Tailscale: encontrado en {}.", path.display()),
        );
        let access = remote.map_or_else(
            || "Acceso desde fuera: apagado (`chxchx-tech node remote enable`).".to_owned(),
            |config| format!("Acceso desde fuera: {}", config.url()),
        );
        format!("{tailscale}\n{access}")
    }
    #[cfg(not(feature = "node"))]
    {
        "El nodo no viene en esta versión.".to_owned()
    }
}

pub(super) fn card(state: &Dashboard) -> Element<'_, Message> {
    let own = &state.settings.connection;
    let drafted = draft(own);
    let changed = drafted.as_ref().is_ok_and(|mine| *mine != own.saved);
    let defaults = connection::load_in(std::path::Path::new("/nonexistent"));

    let mut body = column![
        text(status()).size(11).color(design::muted()),
        row_of(
            "Puerto del nodo",
            &defaults.node_port().to_string(),
            &own.node_port,
            Field::NodePort,
            120
        ),
        row_of(
            "Puerto HTTPS",
            &defaults.https_port().to_string(),
            &own.https_port,
            Field::HttpsPort,
            120
        ),
        hint(
            "El puerto HTTPS es el que usa Tailscale al activar el acceso desde fuera (443 por defecto).",
            false
        ),
        row_of(
            "Versiones de (dueño)",
            defaults.owner(),
            &own.owner,
            Field::Owner,
            Fill
        ),
        row_of(
            "Versiones de (repo)",
            defaults.repository(),
            &own.repository,
            Field::Repository,
            Fill
        ),
        row_of(
            "Clave pública de firma",
            "la incluida en esta versión (64 dígitos hexadecimales)",
            &own.key,
            Field::Key,
            Fill
        ),
        hint(
            "Solo si distribuyes tu propia versión: se aceptan actualizaciones firmadas con esa clave. Es la parte pública; nunca pegues aquí la privada.",
            false
        ),
    ]
    .spacing(8);
    for (index, name) in connection::TOOLS.iter().enumerate() {
        let found = crate::integrations::executable_search::find_executable(name).map_or_else(
            || "no encontrado".to_owned(),
            |path| path.display().to_string(),
        );
        body = body.push(row_of(
            name,
            &found,
            own.tools.get(index).map_or("", String::as_str),
            Field::Tool(index),
            Fill,
        ));
    }
    body = body.push(hint(
        "Las rutas solo hacen falta si la herramienta no está en el PATH; deben ser absolutas y existir.",
        false,
    ));
    match (&drafted, own.notice.is_empty()) {
        (Err(error), _) => body = body.push(hint(error.clone(), true)),
        (Ok(_), false) => body = body.push(hint(own.notice.clone(), own.failed)),
        (Ok(_), true) => {}
    }
    let apply = {
        let button = button(text("Aplicar").size(12))
            .padding([6, 14])
            .style(styles::primary);
        if changed {
            button.on_press(send(Event::Apply))
        } else {
            button
        }
    };
    body = body.push(
        row![
            apply,
            button(text("Valores de fábrica").size(12))
                .padding([6, 12])
                .style(styles::secondary)
                .on_press(send(Event::Reset)),
        ]
        .spacing(10),
    );
    container(body)
        .padding(20)
        .width(Fill)
        .style(styles::card)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_a_home(name: &str, test: impl FnOnce(&mut Dashboard)) {
        let _guard = super::super::identity::TEST_SERIAL
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let dir = std::env::temp_dir().join(format!(
            "chxchx-studio-connection-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        *super::super::identity::TEST_HOME
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(dir);
        let mut state = super::super::super::editor::fixture();
        test(&mut state);
        *super::super::identity::TEST_HOME
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = None;
    }

    fn event(state: &mut Dashboard, event: Event) {
        let _ = update(state, event);
    }

    #[test]
    fn a_new_install_shows_empty_boxes_and_applying_needs_a_change() {
        in_a_home("empty", |state| {
            event(state, Event::Load);
            let own = &state.settings.connection;
            assert!(own.node_port.is_empty() && own.owner.is_empty() && own.key.is_empty());
            assert_eq!(own.tools.len(), connection::TOOLS.len());
            assert_eq!(
                draft(own).unwrap(),
                own.saved,
                "sin cambios no hay nada que aplicar"
            );
        });
    }

    #[test]
    fn typing_valid_values_saves_them_and_a_wrong_one_says_why() {
        in_a_home("apply", |state| {
            event(state, Event::Load);
            event(state, Event::Type(Field::NodePort, "50000".into()));
            event(state, Event::Type(Field::Owner, "mi-org".into()));
            event(state, Event::Apply);
            assert!(
                !state.settings.connection.failed,
                "{}",
                state.settings.connection.notice
            );
            let home = super::super::identity::home().unwrap();
            let saved = connection::load_in(&home);
            assert_eq!(saved.node_port(), 50_000);
            assert_eq!(saved.owner(), "mi-org");

            event(state, Event::Type(Field::NodePort, "80".into()));
            assert!(
                draft(&state.settings.connection)
                    .unwrap_err()
                    .contains("1024")
            );
            event(state, Event::Apply);
            assert!(state.settings.connection.failed);
            assert_eq!(
                connection::load_in(&home).node_port(),
                50_000,
                "lo malo no se guarda"
            );

            event(state, Event::Type(Field::NodePort, "abc".into()));
            assert!(
                draft(&state.settings.connection)
                    .unwrap_err()
                    .contains("número")
            );
        });
    }

    #[test]
    fn a_tool_path_must_exist_and_factory_values_go_back() {
        in_a_home("tools", |state| {
            event(state, Event::Load);
            event(
                state,
                Event::Type(Field::Tool(1), "/no/existe/codex".into()),
            );
            assert!(
                draft(&state.settings.connection)
                    .unwrap_err()
                    .contains("no existe")
            );
            event(state, Event::Type(Field::Tool(1), String::new()));
            event(state, Event::Type(Field::HttpsPort, "8443".into()));
            event(state, Event::Apply);
            let home = super::super::identity::home().unwrap();
            assert_eq!(connection::load_in(&home).https_port(), 8443);
            event(state, Event::Reset);
            assert_eq!(connection::load_in(&home), Connection::default());
            assert!(state.settings.connection.https_port.is_empty());
        });
    }
}
