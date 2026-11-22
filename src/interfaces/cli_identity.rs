//! `chxchx-tech identity`: the name in block letters, the line under it, the picture and the colors
//! that make this installation the person's own. What is set here shows in Studio and on every
//! phone linked to the node. Nothing is deleted: what is replaced stays next to it as `.bak`.
use crate::features::{identity, studio_preferences};
use chxchx_protocol::{self as protocol, Identity};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(super) enum IdentityCommand {
    /// Muestra tu identidad y cómo se dibuja el nombre.
    Show {
        #[arg(long)]
        json: bool,
    },
    /// Cambia lo que quieras; lo que no pongas se queda como está.
    Set {
        /// El nombre que se dibuja en letras de bloques (hasta 12: letras sin tilde, números, espacio, - y .).
        #[arg(long)]
        name: Option<String>,
        /// La línea bajo el nombre; vacía vuelve a la de siempre.
        #[arg(long)]
        tagline: Option<String>,
        /// Una imagen tuya (PNG, JPEG, WebP o GIF); se recorta a cuadrado.
        #[arg(long)]
        avatar: Option<PathBuf>,
        /// Tema de Studio y, si lo pides, de la app web: chxchx, midnight, graphite, forest, light, contrast.
        #[arg(long)]
        theme: Option<String>,
        /// Color de resalte `#rrggbb`, o `none` para el de tu tema.
        #[arg(long)]
        accent: Option<String>,
        /// `auto` (la app web sigue al dispositivo) o `studio` (usa el tema de Studio).
        #[arg(long)]
        web_theme: Option<String>,
        /// Muestra lo que haría sin cambiar nada.
        #[arg(long)]
        dry_run: bool,
    },
    /// Quita tu imagen y vuelve a la del producto.
    ClearAvatar {
        #[arg(long)]
        dry_run: bool,
    },
    /// Vuelve a la identidad del producto (la tuya queda como `.bak`).
    Reset {
        #[arg(long)]
        dry_run: bool,
    },
    /// Lista los temas.
    Themes,
}

pub(super) fn run(command: IdentityCommand) -> Result<(), String> {
    match command {
        IdentityCommand::Show { json } => show(json),
        IdentityCommand::Set {
            name,
            tagline,
            avatar,
            theme,
            accent,
            web_theme,
            dry_run,
        } => set(name, tagline, avatar, theme, accent, web_theme, dry_run),
        IdentityCommand::ClearAvatar { dry_run } => {
            if dry_run {
                println!(
                    "Quitaría tu imagen y volvería a la del producto (la actual quedaría como avatar.png.bak). No se cambió nada."
                );
                return Ok(());
            }
            identity::clear_avatar()?;
            println!("Imagen quitada; la anterior quedó como avatar.png.bak.");
            Ok(())
        }
        IdentityCommand::Reset { dry_run } => {
            if dry_run {
                println!(
                    "Volvería al nombre, la línea y la imagen del producto (lo tuyo quedaría como .bak). No se cambió nada."
                );
                return Ok(());
            }
            let home = crate::features::project_profiles::application_home()
                .ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
            identity::reset_in(&home)?;
            println!("Identidad restablecida; lo anterior quedó como .bak.");
            Ok(())
        }
        IdentityCommand::Themes => {
            for theme in protocol::WEB_THEMES {
                println!("{:<10} {}", theme.id, theme.name);
            }
            Ok(())
        }
    }
}

fn print_wordmark(name: &str) {
    for row in protocol::wordmark(name) {
        println!("{row}");
    }
}

fn show(json: bool) -> Result<(), String> {
    let view = identity::view();
    let loaded = studio_preferences::load().preferences;
    let mine = identity::load();
    if json {
        let value = serde_json::json!({
            "schema": "chxchx.identity",
            "name": view.name,
            "tagline": view.tagline,
            "accent": view.accent,
            "theme": loaded.appearance.theme,
            "web_theme": mine.web_theme,
            "avatar": view.avatar_rev.is_some(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    print_wordmark(&view.name);
    println!("\n{}", view.tagline);
    println!(
        "\nNombre: {} · Tema: {} · Resalte: {} · App web: {} · Imagen: {}",
        view.name,
        loaded.appearance.theme,
        view.accent,
        if mine.web_theme == "studio" {
            "usa el tema de Studio"
        } else {
            "sigue al dispositivo"
        },
        if view.avatar_rev.is_some() {
            "la tuya"
        } else {
            "la del producto"
        },
    );
    Ok(())
}

fn set(
    name: Option<String>,
    tagline: Option<String>,
    avatar: Option<PathBuf>,
    theme: Option<String>,
    accent: Option<String>,
    web_theme: Option<String>,
    dry_run: bool,
) -> Result<(), String> {
    let mut mine = identity::load();
    if let Some(name) = &name {
        mine.name = protocol::normalize_name(name)?;
    }
    if let Some(tagline) = &tagline {
        mine.tagline = protocol::normalize_tagline(tagline)?;
    }
    if let Some(mode) = &web_theme {
        if !matches!(mode.as_str(), "auto" | "studio") {
            return Err("--web-theme admite `auto` o `studio`".to_owned());
        }
        mine.web_theme = mode.clone();
    }
    let mut loaded = studio_preferences::load();
    if let Some(id) = &theme {
        if protocol::web_theme(id).is_none() {
            return Err(format!(
                "Tema desconocido «{id}». Mira `chxchx-tech identity themes`."
            ));
        }
        loaded.preferences.appearance.theme = id.clone();
    }
    if let Some(accent) = &accent {
        loaded.preferences.appearance.accent = if accent.eq_ignore_ascii_case("none") {
            None
        } else {
            Some(protocol::normalize_accent(accent).ok_or("--accent debe ser `#rrggbb` o `none`")?)
        };
    }
    let picture = match &avatar {
        Some(path) => {
            let bytes =
                std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
            Some(identity::prepare_avatar(&bytes)?)
        }
        None => None,
    };
    println!("Así quedaría:\n");
    print_wordmark(&mine.name);
    println!("\n{}", mine.tagline);
    if picture.is_some() {
        println!(
            "\nImagen: la que indicaste, recortada a cuadrado de {0}×{0}.",
            identity::AVATAR_SIDE
        );
    }
    if dry_run {
        println!("\nVista previa: no se cambió nada.");
        return Ok(());
    }
    if let Some(png) = picture {
        identity::set_avatar(&png)?;
    }
    identity::save(&Identity {
        avatar_rev: identity::load().avatar_rev,
        ..mine
    })?;
    if theme.is_some() || accent.is_some() {
        if !loaded.writable {
            return Err(
                "Las preferencias de Studio no se pueden escribir (¿son de una versión más nueva?)"
                    .to_owned(),
            );
        }
        studio_preferences::save(&loaded.preferences)?;
    }
    println!("\nListo. Studio lo verá al abrirse y la app web al recargar.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line_has_no_conflicting_flags() {
        use clap::CommandFactory;
        #[derive(clap::Parser)]
        struct Probe {
            #[command(subcommand)]
            command: IdentityCommand,
        }
        Probe::command().debug_assert();
    }
}
