//! Design tokens for ChxChx Studio: brand palette, surfaces, type and theme.
use iced::font::Weight;
use iced::{Color, Font, Theme};

pub(super) const fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

// The colors in use come from the active theme (see `studio_themes.rs`): every token is a
// function so a change in Settings repaints the whole window.
#[path = "studio_themes.rs"]
pub(super) mod themes;
#[allow(unused_imports)]
pub(super) use themes::{Palette, set_palette};

macro_rules! tokens {
    ($($name:ident),* $(,)?) => {
        $(pub(super) fn $name() -> Color { themes::active().$name })*
    };
}
tokens!(
    brand_cyan,
    brand_blue,
    brand_deep,
    accent,
    bg_deep,
    bg_base,
    bg_panel,
    bg_raised,
    bg_hover,
    bg_active,
    border,
    border_strong,
    fg,
    muted,
    faint,
    success,
    warning,
    danger,
    tint_orange,
    tint_yellow,
    tint_green,
    tint_purple,
);

// System fonts keep the binary small and the text rendering native.
#[cfg(target_os = "windows")]
pub(super) const UI: Font = Font::with_name("Segoe UI");
#[cfg(target_os = "macos")]
pub(super) const UI: Font = Font::with_name("Helvetica Neue");
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(super) const UI: Font = Font::DEFAULT;

#[cfg(target_os = "windows")]
pub(super) const MONO: Font = Font::with_name("Consolas");
#[cfg(target_os = "macos")]
pub(super) const MONO: Font = Font::with_name("Menlo");
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(super) const MONO: Font = Font::MONOSPACE;

pub(super) const UI_BOLD: Font = Font {
    weight: Weight::Semibold,
    ..UI
};

pub(super) fn theme() -> Theme {
    let p = themes::active();
    Theme::custom(
        "ChxChx Studio".to_owned(),
        iced::theme::Palette {
            background: p.bg_base,
            text: p.fg,
            primary: p.brand_blue,
            success: p.success,
            danger: p.danger,
            warning: p.warning,
        },
    )
}
