//! Settings → Identidad: the name in block letters, the line under it, the picture and whether the
//! web client uses Studio's theme. What is saved here is what Studio draws on its welcome screen
//! and what every phone linked to the node shows (the theme and the accent are the ones of the
//! «Apariencia» section above). Nothing is deleted: what is replaced stays next to it as `.bak`.
use super::*;
use crate::features::identity;
use chxchx_protocol::{Identity, normalize_name, normalize_tagline, wordmark};
use iced::widget::{column, text_input};
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct State {
    loaded: bool,
    /// What the person is typing, not saved yet.
    name: String,
    tagline: String,
    /// What is saved.
    saved: Identity,
    has_avatar: bool,
    notice: String,
    failed: bool,
}

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) enum Event {
    Load,
    Name(String),
    Tagline(String),
    Apply,
    PickAvatar,
    Picked(Option<PathBuf>),
    Stored(Result<(), String>),
    ClearAvatar,
    WebTheme(bool),
    Reset,
}

#[cfg(test)]
pub(super) static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[cfg(test)]
pub(super) static TEST_HOME: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Where the identity lives. Tests point it at a folder of their own, never at the person's.
pub(super) fn home() -> Option<PathBuf> {
    #[cfg(test)]
    {
        TEST_HOME
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
    #[cfg(not(test))]
    {
        crate::features::project_profiles::application_home()
    }
}

/// Reads what is saved into the state and puts it to use.
fn reload(state: &mut Dashboard) {
    let Some(home) = home() else { return };
    let saved = identity::load_in(&home);
    let avatar = identity::avatar_in(&home);
    let own = &mut state.settings.identity;
    own.has_avatar = avatar.is_some();
    own.name = saved.name.clone();
    own.tagline = saved.tagline.clone();
    brand::apply(&saved, avatar);
    own.saved = saved;
    own.loaded = true;
}

fn say(state: &mut Dashboard, text: impl Into<String>, failed: bool) {
    state.settings.identity.notice = text.into();
    state.settings.identity.failed = failed;
}

fn save(state: &mut Dashboard, identity: &Identity) -> Result<(), String> {
    let home = home().ok_or("No se pudo resolver la carpeta de ChxChx")?;
    identity::save_in(&home, identity)?;
    reload(state);
    Ok(())
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Load => {
            if !state.settings.identity.loaded {
                reload(state);
            }
        }
        Event::Name(value) => {
            state.settings.identity.name = value.chars().take(40).collect();
            state.settings.identity.notice.clear();
        }
        Event::Tagline(value) => {
            state.settings.identity.tagline = value.chars().take(120).collect();
            state.settings.identity.notice.clear();
        }
        Event::Apply => {
            let own = &state.settings.identity;
            let result = normalize_name(&own.name).and_then(|name| {
                normalize_tagline(&own.tagline).map(|tagline| Identity {
                    name,
                    tagline,
                    ..own.saved.clone()
                })
            });
            match result.and_then(|identity| save(state, &identity)) {
                Ok(()) => say(state, "Guardado. La app web lo muestra al recargar.", false),
                Err(error) => say(state, error, true),
            }
        }
        Event::PickAvatar => {
            return Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Elegir tu imagen")
                        .add_filter("Imágenes", &["png", "jpg", "jpeg", "webp", "gif"])
                        .pick_file()
                        .await
                        .map(|file| file.path().to_path_buf())
                },
                |path| Message::Settings(settings::Event::Identity(Event::Picked(path))),
            );
        }
        Event::Picked(None) => {}
        Event::Picked(Some(path)) => {
            say(state, "Preparando la imagen…", false);
            let Some(home) = home() else {
                say(state, "No se pudo resolver la carpeta de ChxChx", true);
                return Task::none();
            };
            return jobs::quiet(
                move || {
                    let bytes = std::fs::read(&path)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    identity::set_avatar_in(&home, &bytes)
                },
                |state, result| {
                    settings::update(state, settings::Event::Identity(Event::Stored(result)))
                },
            );
        }
        Event::Stored(result) => match result {
            Ok(()) => {
                reload(state);
                say(
                    state,
                    "Imagen guardada. La app web la muestra al recargar.",
                    false,
                );
            }
            Err(error) => say(state, error, true),
        },
        Event::ClearAvatar => match home()
            .ok_or("No se pudo resolver la carpeta de ChxChx".to_owned())
            .and_then(|home| identity::clear_avatar_in(&home))
        {
            Ok(()) => {
                reload(state);
                say(
                    state,
                    "Vuelves a la imagen del producto (la tuya quedó como avatar.png.bak).",
                    false,
                );
            }
            Err(error) => say(state, error, true),
        },
        Event::WebTheme(use_studio) => {
            let mut mine = state.settings.identity.saved.clone();
            mine.web_theme = if use_studio { "studio" } else { "auto" }.to_owned();
            match save(state, &mine) {
                Ok(()) => say(state, "Listo. La app web lo aplica al recargar.", false),
                Err(error) => say(state, error, true),
            }
        }
        Event::Reset => {
            let result = home()
                .ok_or("No se pudo resolver la carpeta de ChxChx".to_owned())
                .and_then(|home| identity::reset_in(&home));
            match result {
                Ok(()) => {
                    state.settings.identity.loaded = false;
                    reload(state);
                    say(
                        state,
                        "Vuelves a la identidad del producto (lo tuyo quedó como .bak).",
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
    Message::Settings(settings::Event::Identity(event))
}

fn field<'a>(
    label: &'a str,
    input: Element<'a, Message>,
    hint: Element<'a, Message>,
) -> Element<'a, Message> {
    column![text(label).size(12).font(design::UI_BOLD), input, hint]
        .spacing(5)
        .into()
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

pub(super) fn card(state: &Dashboard) -> Element<'_, Message> {
    let own = &state.settings.identity;
    let typed_name = normalize_name(&own.name);
    let typed_line = normalize_tagline(&own.tagline);
    // The preview follows what is typed while it is valid, and keeps the last good one meanwhile.
    let preview_name = typed_name
        .clone()
        .unwrap_or_else(|_| own.saved.name.clone());
    let preview_line = typed_line
        .clone()
        .unwrap_or_else(|_| own.saved.tagline.clone());
    let changed = typed_name.as_deref().ok() != Some(own.saved.name.as_str())
        || typed_line.as_deref().ok() != Some(own.saved.tagline.as_str());
    let valid = typed_name.is_ok() && typed_line.is_ok();

    let preview = row![
        brand::mascot(72.0),
        column![
            brand::wordmark_of(&wordmark(&preview_name), 8.0),
            text(preview_line)
                .size(11)
                .font(design::MONO)
                .color(design::muted()),
        ]
        .spacing(8),
    ]
    .spacing(18)
    .align_y(iced::Center);

    let name_input = text_input("CHXCHX", &own.name)
        .on_input(|value| send(Event::Name(value)))
        .on_submit(send(Event::Apply))
        .size(12)
        .padding([6, 10])
        .width(260)
        .style(styles::input);
    let line_input = text_input(
        "control plane local para proyectos con agentes de IA",
        &own.tagline,
    )
    .on_input(|value| send(Event::Tagline(value)))
    .on_submit(send(Event::Apply))
    .size(12)
    .padding([6, 10])
    .width(Fill)
    .style(styles::input);
    let name_hint = match &typed_name {
        Err(error) => hint(error.clone(), true),
        Ok(_) => hint(
            "Hasta 12 caracteres: letras sin tilde, números, espacio, «-» y «.».",
            false,
        ),
    };
    let line_hint = match &typed_line {
        Err(error) => hint(error.clone(), true),
        Ok(_) => hint(
            "Una línea corta bajo el nombre; vacía vuelve a la de siempre.",
            false,
        ),
    };
    let apply = {
        let button = button(text("Aplicar").size(12))
            .padding([6, 14])
            .style(styles::primary);
        if valid && changed {
            button.on_press(send(Event::Apply))
        } else {
            button
        }
    };

    let picture = row![
        text("Imagen").size(12).font(design::UI_BOLD).width(60),
        button(text("Elegir imagen…").size(12))
            .padding([6, 12])
            .style(styles::secondary)
            .on_press(send(Event::PickAvatar)),
        {
            let quit = button(text("Quitar").size(12))
                .padding([6, 12])
                .style(styles::ghost(false));
            if own.has_avatar {
                quit.on_press(send(Event::ClearAvatar))
            } else {
                quit
            }
        },
        text(if own.has_avatar {
            "La tuya (recortada a cuadrado)"
        } else {
            "La del producto"
        })
        .size(11)
        .color(design::muted()),
    ]
    .spacing(10)
    .align_y(iced::Center);

    let uses_studio = own.saved.web_theme == "studio";
    let web = row![
        text("App web").size(12).font(design::UI_BOLD).width(60),
        button(text("Seguir al dispositivo").size(12))
            .padding([6, 12])
            .style(styles::ghost(!uses_studio))
            .on_press(send(Event::WebTheme(false))),
        button(text("Usar el tema de Studio").size(12))
            .padding([6, 12])
            .style(styles::ghost(uses_studio))
            .on_press(send(Event::WebTheme(true))),
    ]
    .spacing(10)
    .align_y(iced::Center);

    let mut body = column![
        preview,
        field("Nombre", name_input.into(), name_hint),
        field("Lema", line_input.into(), line_hint),
        row![apply, Space::new().width(Fill)].align_y(iced::Center),
        picture,
        web,
        hint(
            "El color de resalte de «Apariencia» también se usa en la app web, con tu rótulo.",
            false
        ),
    ]
    .spacing(12);
    if !own.notice.is_empty() {
        body = body.push(text(own.notice.clone()).size(12).color(if own.failed {
            design::warning()
        } else {
            design::success()
        }));
    }
    body = body.push(row![
        Space::new().width(Fill),
        button(text("Restablecer a la identidad del producto").size(11))
            .padding([5, 10])
            .style(styles::ghost(false))
            .on_press(send(Event::Reset)),
    ]);
    container(body)
        .padding(18)
        .width(Fill)
        .style(styles::card)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `test` with the identity pointed at a folder of its own, one test at a time.
    fn in_a_home(name: &str, test: impl FnOnce(&mut Dashboard)) {
        let _guard = TEST_SERIAL
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let dir = std::env::temp_dir().join(format!(
            "chxchx-studio-identity-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        *TEST_HOME
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(dir);
        let mut state = super::super::super::editor::fixture();
        test(&mut state);
        *TEST_HOME
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = None;
    }

    fn event(state: &mut Dashboard, event: Event) {
        let _ = update(state, event);
    }

    #[test]
    fn opening_the_section_shows_what_is_saved_and_keeps_what_is_being_typed() {
        in_a_home("load", |state| {
            event(state, Event::Load);
            assert_eq!(state.settings.identity.name, "CHXCHX");
            event(state, Event::Name("ana".into()));
            event(state, Event::Load);
            assert_eq!(
                state.settings.identity.name, "ana",
                "loading again does not erase the typing"
            );
        });
    }

    #[test]
    fn applying_saves_the_name_upper_case_and_the_screens_use_it() {
        in_a_home("apply", |state| {
            event(state, Event::Load);
            event(state, Event::Name("  ana dev ".into()));
            event(state, Event::Tagline("mi taller".into()));
            event(state, Event::Apply);
            assert!(
                !state.settings.identity.failed,
                "{}",
                state.settings.identity.notice
            );
            let saved = identity::load_in(&home().unwrap());
            assert_eq!(
                (saved.name.as_str(), saved.tagline.as_str()),
                ("ANA DEV", "mi taller")
            );
            assert_eq!(
                state.settings.identity.name, "ANA DEV",
                "the box shows what was saved"
            );
            assert_eq!(super::brand::tagline(), "mi taller", "Studio draws it");
        });
    }

    #[test]
    fn a_name_that_cannot_be_drawn_is_refused_with_the_reason_and_nothing_is_saved() {
        in_a_home("refuse", |state| {
            event(state, Event::Load);
            event(state, Event::Name("ñandú".into()));
            event(state, Event::Apply);
            assert!(state.settings.identity.failed);
            assert!(
                state
                    .settings
                    .identity
                    .notice
                    .contains("no se puede dibujar")
            );
            assert_eq!(identity::load_in(&home().unwrap()).name, "CHXCHX");
            event(state, Event::Name("DEMASIADOS CARACTERES".into()));
            event(state, Event::Apply);
            assert!(state.settings.identity.failed);
        });
    }

    #[test]
    fn the_web_theme_choice_is_saved_and_reset_goes_back_keeping_a_copy() {
        in_a_home("web", |state| {
            event(state, Event::Load);
            assert_eq!(
                identity::load_in(&home().unwrap()).web_theme,
                "studio",
                "Studio's theme by default"
            );
            event(state, Event::WebTheme(false));
            assert_eq!(identity::load_in(&home().unwrap()).web_theme, "auto");
            event(state, Event::Name("beto".into()));
            event(state, Event::Apply);
            event(state, Event::Reset);
            let mine = identity::load_in(&home().unwrap());
            assert_eq!(
                (mine.name.as_str(), mine.web_theme.as_str()),
                ("CHXCHX", "studio"),
                "reset goes back to the default, which follows Studio"
            );
            assert_eq!(state.settings.identity.name, "CHXCHX", "the box follows");
            assert!(
                std::fs::read_to_string(home().unwrap().join("identity/identity.json.bak"))
                    .unwrap()
                    .contains("BETO")
            );
        });
    }

    #[test]
    fn a_stored_picture_is_noticed_and_removing_it_goes_back_to_the_products() {
        in_a_home("avatar", |state| {
            event(state, Event::Load);
            let image = image::RgbaImage::from_pixel(64, 32, image::Rgba([9, 99, 199, 255]));
            let mut png = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(image)
                .write_to(&mut png, image::ImageFormat::Png)
                .unwrap();
            identity::set_avatar_in(&home().unwrap(), &png.into_inner()).unwrap();
            event(state, Event::Stored(Ok(())));
            assert!(state.settings.identity.has_avatar);
            event(state, Event::Stored(Err("No pude leer la imagen".into())));
            assert!(state.settings.identity.failed);
            event(state, Event::ClearAvatar);
            assert!(!state.settings.identity.has_avatar);
        });
    }

    #[test]
    fn the_card_draws_in_every_state_without_panicking() {
        in_a_home("view", |state| {
            event(state, Event::Load);
            let _ = card(state);
            event(state, Event::Name("ñ".into()));
            let _ = card(state);
            event(state, Event::Name("ana".into()));
            event(state, Event::Apply);
            let _ = card(state);
        });
    }
}
