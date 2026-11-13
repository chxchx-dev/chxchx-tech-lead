//! Design tokens for ChxChx Studio: brand palette, surfaces, type and theme.
use iced::font::Weight;
use iced::{Color, Font, Theme};

const fn hex(value: u32) -> Color {
    Color::from_rgb(
        ((value >> 16) & 0xff) as f32 / 255.0,
        ((value >> 8) & 0xff) as f32 / 255.0,
        (value & 0xff) as f32 / 255.0,
    )
}

pub(super) const fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

// Brand blues sampled from the ChxChx mark.
pub(super) const BRAND_CYAN: Color = hex(0x22D3FF);
pub(super) const BRAND_BLUE: Color = hex(0x1E8BFF);
pub(super) const BRAND_DEEP: Color = hex(0x1557F0);
pub(super) const ACCENT: Color = BRAND_CYAN;

// Surfaces, from the window chrome up to raised cards.
pub(super) const BG_DEEP: Color = hex(0x070B14);
pub(super) const BG_BASE: Color = hex(0x0B111D);
pub(super) const BG_PANEL: Color = hex(0x0D1524);
pub(super) const BG_RAISED: Color = hex(0x121C2E);
pub(super) const BG_HOVER: Color = hex(0x18243A);
pub(super) const BG_ACTIVE: Color = hex(0x1A2C4A);
pub(super) const BORDER: Color = hex(0x1C2940);
pub(super) const BORDER_STRONG: Color = hex(0x2A3B59);

pub(super) const TEXT: Color = hex(0xE6EDF7);
pub(super) const MUTED: Color = hex(0x8FA1BC);
pub(super) const FAINT: Color = hex(0x5C6F8C);

pub(super) const SUCCESS: Color = hex(0x3DD68C);
pub(super) const WARNING: Color = hex(0xF5B84B);
pub(super) const DANGER: Color = hex(0xF2707A);

// File-type tints used by the explorer and editor tabs.
pub(super) const TINT_ORANGE: Color = hex(0xF59E6B);
pub(super) const TINT_YELLOW: Color = hex(0xE8C55A);
pub(super) const TINT_GREEN: Color = hex(0x6FCF97);
pub(super) const TINT_PURPLE: Color = hex(0xB08CFF);

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
    Theme::custom(
        "ChxChx Studio".to_owned(),
        iced::theme::Palette {
            background: BG_BASE,
            text: TEXT,
            primary: BRAND_BLUE,
            success: SUCCESS,
            danger: DANGER,
            warning: WARNING,
        },
    )
}
