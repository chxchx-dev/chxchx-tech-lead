//! ChxChx brand assets, pre-scaled from `assets/` at the repository root.
use iced::widget::image::{self, Handle};
use std::sync::LazyLock;

const MARK_SMALL: &[u8] = include_bytes!("../../assets/brand/mark-40.png");
const MARK_LARGE: &[u8] = include_bytes!("../../assets/brand/mark-256.png");
const LOGO: &[u8] = include_bytes!("../../assets/brand/logo-420.png");
/// Width / height of the full logo (mark plus wordmark).
const LOGO_RATIO: f32 = 395.0 / 420.0;

// Handles are created once so the renderer decodes and caches each image once.
static MARK_SMALL_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(MARK_SMALL));
static MARK_LARGE_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(MARK_LARGE));
static LOGO_HANDLE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(LOGO));

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

/// The full logo with the ChxChx wordmark.
pub(super) fn logo(height: f32) -> image::Image<Handle> {
    image::Image::new(LOGO_HANDLE.clone())
        .width(height * LOGO_RATIO)
        .height(height)
}

/// Window and taskbar icon. Returns `None` if the embedded PNG cannot be read.
pub(super) fn window_icon() -> Option<iced::window::Icon> {
    let pixels = ::image::load_from_memory_with_format(MARK_LARGE, ::image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    let (width, height) = pixels.dimensions();
    iced::window::icon::from_rgba(pixels.into_raw(), width, height).ok()
}
