//! A person's own identity on disk (ADR 0017): the name in block letters, the line under it and the
//! picture, kept in the application's folder so Studio, the command line and the node (and through
//! it every phone) read the same thing. The theme and the accent are not repeated here: they are the
//! ones of Studio's appearance preferences.
//!
//! What may be written, and how the name is drawn, is in `chxchx_protocol::identity`.

use crate::features::project_profiles::application_home;
use crate::features::studio_preferences;
use chxchx_protocol::{Identity, IdentityView};
use std::fs;
use std::path::{Path, PathBuf};

const FOLDER: &str = "identity";
const FILE: &str = "identity.json";
const AVATAR: &str = "avatar.png";
/// Side of the stored picture, in pixels.
pub const AVATAR_SIDE: u32 = 256;
/// Largest file accepted as a picture.
pub const MAX_AVATAR_BYTES: usize = 12 * 1024 * 1024;

fn folder(home: &Path) -> PathBuf {
    home.join(FOLDER)
}

fn plain_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file() && !meta.file_type().is_symlink())
}

/// What is saved, or the default identity. Anything invalid in it is put back to its default.
pub fn load_in(home: &Path) -> Identity {
    let path = folder(home).join(FILE);
    if !plain_file(&path) {
        return Identity::default();
    }
    fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Identity>(&bytes).ok())
        .map(|identity| identity.sanitized())
        .unwrap_or_default()
}

/// Writes `contents` at `path` through a temporary file, keeping the previous one as `.bak`.
fn write_keeping_backup(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Ruta sin carpeta")?;
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    if plain_file(path) {
        let mut backup = path.as_os_str().to_owned();
        backup.push(".bak");
        let _ = fs::copy(path, PathBuf::from(backup));
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(".{}.tmp", std::process::id()));
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, contents).map_err(|error| format!("{}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("{}: {error}", path.display())
    })
}

pub fn save_in(home: &Path, identity: &Identity) -> Result<(), String> {
    let clean = identity.sanitized();
    let bytes = serde_json::to_vec_pretty(&clean).map_err(|error| error.to_string())?;
    write_keeping_backup(&folder(home).join(FILE), &bytes)
}

/// The stored picture, if there is one.
pub fn avatar_in(home: &Path) -> Option<Vec<u8>> {
    let path = folder(home).join(AVATAR);
    plain_file(&path).then(|| fs::read(path).ok()).flatten()
}

/// What the node tells its clients: the identity ready to draw, with the accent of the person's
/// appearance preferences.
pub fn view_in(home: &Path) -> IdentityView {
    let accent = studio_preferences::load_from(home)
        .preferences
        .appearance
        .accent;
    let mut identity = load_in(home);
    if avatar_in(home).is_none() {
        identity.avatar_rev = None;
    }
    identity.view(accent.as_deref())
}

/// The CSS for the web client: the Studio theme when the person asked for it, plus the accent.
pub fn css_in(home: &Path) -> String {
    let look = studio_preferences::load_from(home).preferences.appearance;
    let identity = load_in(home);
    let theme = (identity.web_theme == "studio").then_some(look.theme.as_str());
    chxchx_protocol::theme_css(theme, look.accent.as_deref())
}

/// Turns any picture the person has into the square one the product stores: the largest centred
/// square, [`AVATAR_SIDE`] pixels wide, as PNG. Refuses what is not a picture or is absurdly big.
#[cfg(any(feature = "studio", feature = "node"))]
pub fn prepare_avatar(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use image::{ImageFormat, ImageReader, Limits, imageops::FilterType};
    use std::io::Cursor;
    if bytes.is_empty() || bytes.len() > MAX_AVATAR_BYTES {
        return Err(format!(
            "La imagen debe pesar entre 1 byte y {} MB",
            MAX_AVATAR_BYTES / (1024 * 1024)
        ));
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(8_000);
    limits.max_image_height = Some(8_000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let picture = reader
        .decode()
        .map_err(|_| "No pude leer la imagen: usa un PNG, JPEG, WebP o GIF válido".to_owned())?;
    let side = picture.width().min(picture.height());
    let square = picture.crop_imm(
        (picture.width() - side) / 2,
        (picture.height() - side) / 2,
        side,
        side,
    );
    let resized = square.resize_exact(AVATAR_SIDE, AVATAR_SIDE, FilterType::Lanczos3);
    let mut out = Cursor::new(Vec::new());
    resized
        .write_to(&mut out, ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok(out.into_inner())
}

/// The console-only build carries no image decoder.
#[cfg(not(any(feature = "studio", feature = "node")))]
pub fn prepare_avatar(_: &[u8]) -> Result<Vec<u8>, String> {
    Err("Esta compilación no procesa imágenes (usa la versión con Studio o con el nodo)".to_owned())
}

/// Stores a picture as the person's and marks the identity as having one.
pub fn set_avatar_in(home: &Path, bytes: &[u8]) -> Result<(), String> {
    let png = prepare_avatar(bytes)?;
    write_keeping_backup(&folder(home).join(AVATAR), &png)?;
    let mut identity = load_in(home);
    identity.avatar_rev = Some(crate::features::jobs::store::now_ms());
    save_in(home, &identity)
}

/// Back to the picture of the product. The previous one stays as `avatar.png.bak`.
pub fn clear_avatar_in(home: &Path) -> Result<(), String> {
    let path = folder(home).join(AVATAR);
    if plain_file(&path) {
        let mut backup = path.as_os_str().to_owned();
        backup.push(".bak");
        fs::rename(&path, PathBuf::from(backup))
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    let mut identity = load_in(home);
    identity.avatar_rev = None;
    save_in(home, &identity)
}

/// Back to the identity of the product (name, line, picture and web theme). Nothing is deleted:
/// the previous files stay as `.bak`.
pub fn reset_in(home: &Path) -> Result<(), String> {
    clear_avatar_in(home)?;
    save_in(home, &Identity::default())
}

fn here() -> Result<PathBuf, String> {
    application_home().ok_or_else(|| "No se pudo resolver CHXCHX_TECH_HOME".to_owned())
}

pub fn load() -> Identity {
    here().map(|home| load_in(&home)).unwrap_or_default()
}

pub fn save(identity: &Identity) -> Result<(), String> {
    save_in(&here()?, identity)
}

pub fn avatar() -> Option<Vec<u8>> {
    avatar_in(&here().ok()?)
}

pub fn view() -> IdentityView {
    match here() {
        Ok(home) => view_in(&home),
        Err(_) => Identity::default().view(None),
    }
}

pub fn css() -> String {
    here().map(|home| css_in(&home)).unwrap_or_default()
}

pub fn set_avatar(bytes: &[u8]) -> Result<(), String> {
    set_avatar_in(&here()?, bytes)
}

pub fn clear_avatar() -> Result<(), String> {
    clear_avatar_in(&here()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chxchx_protocol::{DEFAULT_NAME, DEFAULT_TAGLINE};

    fn home(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chxchx-identity-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(any(feature = "studio", feature = "node"))]
    fn picture(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_fn(width, height, |x, _| {
            image::Rgba([(x % 256) as u8, 80, 200, 255])
        });
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn with_nothing_saved_the_identity_is_the_products() {
        let home = home("default");
        let identity = load_in(&home);
        assert_eq!(
            (identity.name.as_str(), identity.tagline.as_str()),
            (DEFAULT_NAME, DEFAULT_TAGLINE)
        );
        assert!(avatar_in(&home).is_none());
        assert_eq!(view_in(&home).name, DEFAULT_NAME);
        assert_eq!(
            css_in(&home),
            "",
            "no theme or accent chosen: the app's own look"
        );
    }

    #[test]
    fn a_saved_identity_comes_back_and_the_previous_one_is_kept() {
        let home = home("save");
        let mut mine = Identity {
            name: "ana".into(),
            tagline: "mi taller".into(),
            ..Identity::default()
        };
        save_in(&home, &mine).unwrap();
        assert_eq!(load_in(&home).name, "ANA", "the name is upper case");
        mine.name = "BETO".into();
        save_in(&home, &mine).unwrap();
        assert_eq!(load_in(&home).name, "BETO");
        let backup = fs::read_to_string(folder(&home).join("identity.json.bak")).unwrap();
        assert!(
            backup.contains("ANA"),
            "the previous one stays as .bak: {backup}"
        );
        assert_eq!(view_in(&home).wordmark.len(), chxchx_protocol::GLYPH_ROWS);
    }

    #[test]
    fn a_damaged_or_hostile_file_never_breaks_the_screens() {
        let home = home("hostile");
        fs::create_dir_all(folder(&home)).unwrap();
        fs::write(folder(&home).join("identity.json"), b"{ not json").unwrap();
        assert_eq!(load_in(&home), Identity::default());
        fs::write(
            folder(&home).join("identity.json"),
            br#"{"name":"<script>alert(1)</script>","tagline":"x\u0007","web_theme":"rojo"}"#,
        )
        .unwrap();
        let identity = load_in(&home);
        assert_eq!(
            (identity.name.as_str(), identity.web_theme.as_str()),
            (DEFAULT_NAME, "auto")
        );
    }

    #[test]
    fn the_theme_and_the_accent_come_from_the_appearance_of_studio() {
        let home = home("theme");
        let mut loaded = studio_preferences::load_from(&home);
        loaded.preferences.appearance.theme = "forest".into();
        loaded.preferences.appearance.accent = Some("#f472b6".into());
        studio_preferences::save_to(&home, &loaded.preferences).unwrap();
        // The web client follows the device unless asked to use the theme: the accent applies anyway.
        let css = css_in(&home);
        assert!(
            css.contains("--accent: #f472b6") && !css.contains("--bg:"),
            "{css}"
        );
        assert_eq!(view_in(&home).accent, "#f472b6");
        let mut identity = load_in(&home);
        identity.web_theme = "studio".into();
        save_in(&home, &identity).unwrap();
        let css = css_in(&home);
        assert!(
            css.contains("--bg: #08110e") && css.contains("--accent: #f472b6"),
            "{css}"
        );
    }

    #[cfg(any(feature = "studio", feature = "node"))]
    #[test]
    fn any_picture_becomes_a_centred_square_png() {
        let home = home("avatar");
        set_avatar_in(&home, &picture(600, 300)).unwrap();
        let stored = avatar_in(&home).expect("it was stored");
        let decoded =
            image::load_from_memory_with_format(&stored, image::ImageFormat::Png).unwrap();
        assert_eq!(
            (decoded.width(), decoded.height()),
            (AVATAR_SIDE, AVATAR_SIDE)
        );
        assert!(
            view_in(&home).avatar_rev.is_some(),
            "the screens learn there is a picture"
        );
        // Replacing it keeps the old one, and removing it goes back to the product's.
        set_avatar_in(&home, &picture(100, 100)).unwrap();
        assert!(folder(&home).join("avatar.png.bak").exists());
        clear_avatar_in(&home).unwrap();
        assert!(avatar_in(&home).is_none() && view_in(&home).avatar_rev.is_none());
    }

    #[cfg(any(feature = "studio", feature = "node"))]
    #[test]
    fn what_is_not_a_picture_or_is_absurd_is_refused_and_changes_nothing() {
        let home = home("refuse");
        assert!(prepare_avatar(b"").is_err());
        assert!(prepare_avatar(b"GIF89a-no-es-una-imagen").is_err());
        assert!(
            prepare_avatar(b"<svg xmlns='http://www.w3.org/2000/svg'><script>1</script></svg>")
                .is_err(),
            "no SVG: it could carry a script"
        );
        assert!(prepare_avatar(&vec![0u8; MAX_AVATAR_BYTES + 1]).is_err());
        assert!(set_avatar_in(&home, b"basura").is_err());
        assert!(avatar_in(&home).is_none());
        assert!(
            !folder(&home).join("identity.json").exists(),
            "a refusal writes nothing"
        );
    }

    #[test]
    fn resetting_goes_back_to_the_default_and_keeps_what_was_there() {
        let home = home("reset");
        let mine = Identity {
            name: "ana".into(),
            ..Identity::default()
        };
        save_in(&home, &mine).unwrap();
        reset_in(&home).unwrap();
        assert_eq!(load_in(&home).name, DEFAULT_NAME);
        let backup = fs::read_to_string(folder(&home).join("identity.json.bak")).unwrap();
        assert!(backup.contains("ANA"), "{backup}");
    }
}
