//! ChxChx brand assets, pre-scaled from `assets/` at the repository root.
use iced::widget::image::{self, Handle};
use std::sync::LazyLock;

const MARK_SMALL: &[u8] = include_bytes!("../../assets/brand/mark-40.png");
const MARK_LARGE: &[u8] = include_bytes!("../../assets/brand/mark-256.png");
/// The character that goes with the name (the hooded figure and its smiling mask), already round.
const MASCOT: &[u8] = include_bytes!("../../assets/brand/mascot-256.png");

// Handles are created once so the renderer decodes and caches each image once.
static MARK_SMALL_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(MARK_SMALL));
static MARK_LARGE_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(MARK_LARGE));
static MASCOT_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(MASCOT));

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

/// The character, square, for the welcome screen, empty states and avatars.
pub(super) fn mascot(size: f32) -> image::Image<Handle> {
    image::Image::new(MASCOT_HANDLE.clone())
        .width(size)
        .height(size)
}

/// The CHXCHX block letters of the terminal banner, one per row of the art.
const WORDMARK: [&str; 6] = [
    " ██████╗██╗  ██╗██╗  ██╗ ██████╗██╗  ██╗██╗  ██╗",
    "██╔════╝██║  ██║╚██╗██╔╝██╔════╝██║  ██║╚██╗██╔╝",
    "██║     ███████║ ╚███╔╝ ██║     ███████║ ╚███╔╝ ",
    "██║     ██╔══██║ ██╔██╗ ██║     ██╔══██║ ██╔██╗ ",
    "╚██████╗██║  ██║██╔╝ ██╗╚██████╗██║  ██║██╔╝ ██╗",
    " ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝",
];

fn mix(from: iced::Color, to: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: 1.0,
    }
}

/// The name in block letters, fading from the brand cyan at the top to the brand blue at the
/// bottom, like the banner of the command line. Drawn as text, so it stays sharp at any scale.
pub(super) fn wordmark<'a, M: 'a>(size: f32) -> iced::Element<'a, M> {
    let mut lines = iced::widget::column![].spacing(0);
    for (index, line) in WORDMARK.iter().enumerate() {
        let color = mix(
            super::design::brand_cyan(),
            super::design::brand_blue(),
            index as f32 / (WORDMARK.len() - 1) as f32,
        );
        lines = lines.push(
            iced::widget::text(*line)
                .size(size)
                .font(super::design::MONO)
                .line_height(1.0)
                .color(color)
                .wrapping(iced::widget::text::Wrapping::None),
        );
    }
    lines.into()
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
    fn the_wordmark_is_a_clean_rectangle_of_monospace_cells() {
        let width = WORDMARK[0].chars().count();
        assert!(width > 40);
        for line in WORDMARK {
            assert_eq!(line.chars().count(), width, "{line:?}");
        }
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
    fn both_images_are_valid_pngs() {
        for bytes in [MASCOT, MARK_LARGE] {
            let image = ::image::load_from_memory_with_format(bytes, ::image::ImageFormat::Png)
                .expect("an embedded brand image decodes");
            assert!(image.width() >= 128 && image.width() == image.height());
        }
    }
}
