//! ChxChx brand assets, pre-scaled from `assets/` at the repository root, and the person's own
//! identity (their name in block letters, their line and their picture) when they set one.
use chxchx_protocol::{DEFAULT_NAME, DEFAULT_TAGLINE, Identity};
use iced::widget::image::{self, Handle};
use std::sync::{LazyLock, RwLock};

const MARK_SMALL: &[u8] = include_bytes!("../../assets/brand/mark-40.png");
const MARK_LARGE: &[u8] = include_bytes!("../../assets/brand/mark-256.png");
/// The character that goes with the name (the hooded figure and its smiling mask), already round.
const MASCOT: &[u8] = include_bytes!("../../assets/brand/mascot-256.png");

// Handles are created once so the renderer decodes and caches each image once.
static MARK_SMALL_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(MARK_SMALL));
static MARK_LARGE_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(MARK_LARGE));
static MASCOT_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(MASCOT));

/// What the person made their own. `None` while nothing is set: the product's own identity.
struct Own {
    lines: Vec<String>,
    tagline: String,
    avatar: Option<Handle>,
}

static OWN: RwLock<Option<Own>> = RwLock::new(None);
/// The second color of the gradient the person chose (`#rrggbb`), if any.
static SECOND: RwLock<Option<String>> = RwLock::new(None);

/// Tells the brand which second color the person chose (from the appearance preferences).
pub(super) fn set_second_color(color: Option<&str>) {
    if let Ok(mut slot) = SECOND.write() {
        *slot = color.map(str::to_owned);
    }
}

/// Puts the person's identity to use: from now on the welcome screen, the empty editor and new
/// chats draw it. `avatar` is the stored picture, if there is one.
pub(super) fn apply(identity: &Identity, avatar: Option<Vec<u8>>) {
    let clean = identity.sanitized();
    let own = Own {
        lines: chxchx_protocol::wordmark(&clean.name),
        tagline: clean.tagline,
        // `from_bytes` decodes on first use, so a new handle is a new picture: it never shows the old one.
        avatar: avatar.map(Handle::from_bytes),
    };
    if let Ok(mut slot) = OWN.write() {
        *slot = Some(own);
    }
}

/// The ChxChx mark, square, for chrome and empty states.
pub(super) fn mark(size: f32) -> image::Image<Handle> {
    let handle = if size <= 40.0 {
        &MARK_SMALL_HANDLE
    } else {
        &MARK_LARGE_HANDLE
    };
    image::Image::new((*handle).clone())
        .width(size)
        .height(size)
}

/// The picture of the person, square: their own when they set one, else the product's character.
pub(super) fn mascot(size: f32) -> image::Image<Handle> {
    let handle = OWN
        .read()
        .ok()
        .and_then(|own| own.as_ref().and_then(|own| own.avatar.clone()))
        .unwrap_or_else(|| MASCOT_HANDLE.clone());
    image::Image::new(handle).width(size).height(size)
}

/// The line under the name.
pub(super) fn tagline() -> String {
    OWN.read()
        .ok()
        .and_then(|own| own.as_ref().map(|own| own.tagline.clone()))
        .unwrap_or_else(|| DEFAULT_TAGLINE.to_owned())
}

fn lines() -> Vec<String> {
    OWN.read()
        .ok()
        .and_then(|own| own.as_ref().map(|own| own.lines.clone()))
        .unwrap_or_else(|| chxchx_protocol::wordmark(DEFAULT_NAME))
}

fn color((r, g, b): (u8, u8, u8)) -> iced::Color {
    iced::Color::from_rgb8(r, g, b)
}

fn hex_of(color: iced::Color) -> String {
    let byte = |value: f32| (value * 255.0).round().clamp(0.0, 255.0) as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(color.r),
        byte(color.g),
        byte(color.b)
    )
}

/// The colors a name fades between: the accent in use at the top and a little toward blue at the
/// bottom, the same two the web client draws with.
fn fade() -> (iced::Color, iced::Color) {
    let second = SECOND.read().ok().and_then(|second| second.clone());
    let (top, bottom) =
        chxchx_protocol::gradient(&hex_of(super::design::accent()), second.as_deref());
    (
        color(chxchx_protocol::rgb(&top).unwrap_or((0, 217, 255))),
        color(chxchx_protocol::rgb(&bottom).unwrap_or((42, 139, 255))),
    )
}

fn mix(from: iced::Color, to: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: 1.0,
    }
}

/// The person's name in block letters, fading from the accent at the top to blue at the bottom, like
/// the banner of the command line. Drawn as text, so it stays sharp at any scale.
pub(super) fn wordmark<'a, M: 'a>(size: f32) -> iced::Element<'a, M> {
    wordmark_of(&lines(), size)
}

/// Any rows of block letters in the same colors (for the preview while the name is being typed).
pub(super) fn wordmark_of<'a, M: 'a>(rows: &[String], size: f32) -> iced::Element<'a, M> {
    let (top, bottom) = fade();
    let last = rows.len().saturating_sub(1).max(1) as f32;
    let mut column = iced::widget::column![].spacing(0);
    for (index, line) in rows.iter().enumerate() {
        column = column.push(
            iced::widget::text(line.clone())
                .size(size)
                .font(super::design::MONO)
                .line_height(1.0)
                .color(mix(top, bottom, index as f32 / last))
                .wrapping(iced::widget::text::Wrapping::None),
        );
    }
    column.into()
}

/// Window and taskbar icon. Returns `None` if the embedded PNG cannot be read.
pub(super) fn window_icon() -> Option<iced::window::Icon> {
    let pixels = ::image::load_from_memory_with_format(MARK_LARGE, ::image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    let (width, height) = pixels.dimensions();
    iced::window::icon::from_rgba(pixels.into_raw(), width, height).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_name_is_a_clean_rectangle_of_monospace_cells() {
        let rows = chxchx_protocol::wordmark(DEFAULT_NAME);
        let width = rows[0].chars().count();
        assert!(width > 40);
        assert!(rows.iter().all(|row| row.chars().count() == width));
    }

    #[test]
    fn the_fade_runs_from_the_first_color_to_the_second() {
        let (a, b) = (
            iced::Color::from_rgb(0.0, 0.8, 1.0),
            iced::Color::from_rgb(0.2, 0.4, 1.0),
        );
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }

    #[test]
    fn a_color_goes_to_hex_and_back_without_changing() {
        let accent = iced::Color::from_rgb8(0x22, 0xd3, 0xff);
        assert_eq!(hex_of(accent), "#22d3ff");
        assert_eq!(
            chxchx_protocol::rgb(&hex_of(accent)),
            Some((0x22, 0xd3, 0xff))
        );
    }

    #[test]
    fn both_images_are_valid_pngs() {
        for bytes in [MASCOT, MARK_LARGE] {
            let image = ::image::load_from_memory_with_format(bytes, ::image::ImageFormat::Png)
                .expect("an embedded brand image decodes");
            assert!(image.width() >= 128 && image.width() == image.height());
        }
    }
}
