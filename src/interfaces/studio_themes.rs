//! The color themes of Studio and the palette in use.
//!
//! A theme is a full set of colors. The person may also pick an accent color that replaces the
//! theme's own highlight. The active palette is global because every widget style reads it.
use iced::Color;
use std::sync::RwLock;

pub(in crate::interfaces::studio) const fn hex(value: u32) -> Color {
    Color::from_rgb(
        ((value >> 16) & 0xff) as f32 / 255.0,
        ((value >> 8) & 0xff) as f32 / 255.0,
        (value & 0xff) as f32 / 255.0,
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::interfaces::studio) struct Palette {
    pub brand_cyan: Color,
    pub brand_blue: Color,
    pub brand_deep: Color,
    pub accent: Color,
    pub bg_deep: Color,
    pub bg_base: Color,
    pub bg_panel: Color,
    pub bg_raised: Color,
    pub bg_hover: Color,
    pub bg_active: Color,
    pub border: Color,
    pub border_strong: Color,
    pub fg: Color,
    pub muted: Color,
    pub faint: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub tint_orange: Color,
    pub tint_yellow: Color,
    pub tint_green: Color,
    pub tint_purple: Color,
}

const DARK: Palette = Palette {
    brand_cyan: hex(0x22D3FF),
    brand_blue: hex(0x1E8BFF),
    brand_deep: hex(0x1557F0),
    accent: hex(0x22D3FF),
    bg_deep: hex(0x070B14),
    bg_base: hex(0x0B111D),
    bg_panel: hex(0x0D1524),
    bg_raised: hex(0x121C2E),
    bg_hover: hex(0x18243A),
    bg_active: hex(0x1A2C4A),
    border: hex(0x1C2940),
    border_strong: hex(0x2A3B59),
    fg: hex(0xE6EDF7),
    muted: hex(0x8FA1BC),
    faint: hex(0x5C6F8C),
    success: hex(0x3DD68C),
    warning: hex(0xF5B84B),
    danger: hex(0xF2707A),
    tint_orange: hex(0xF59E6B),
    tint_yellow: hex(0xE8C55A),
    tint_green: hex(0x6FCF97),
    tint_purple: hex(0xB08CFF),
};

const MIDNIGHT: Palette = Palette {
    bg_deep: hex(0x000000),
    bg_base: hex(0x05060A),
    bg_panel: hex(0x0A0C12),
    bg_raised: hex(0x10131B),
    bg_hover: hex(0x171B26),
    bg_active: hex(0x1D2333),
    border: hex(0x161A24),
    border_strong: hex(0x262C3B),
    fg: hex(0xE8ECF4),
    muted: hex(0x8A93A6),
    faint: hex(0x555D70),
    ..DARK
};

const GRAPHITE: Palette = Palette {
    brand_cyan: hex(0x4FC1FF),
    brand_blue: hex(0x2F80ED),
    brand_deep: hex(0x1F5FBF),
    accent: hex(0x4FC1FF),
    bg_deep: hex(0x181818),
    bg_base: hex(0x1F1F1F),
    bg_panel: hex(0x252526),
    bg_raised: hex(0x2D2D30),
    bg_hover: hex(0x37373D),
    bg_active: hex(0x094771),
    border: hex(0x333333),
    border_strong: hex(0x454545),
    fg: hex(0xD8D8D8),
    muted: hex(0x9A9A9A),
    faint: hex(0x6E6E6E),
    ..DARK
};

const FOREST: Palette = Palette {
    brand_cyan: hex(0x5EEAD4),
    brand_blue: hex(0x22A67A),
    brand_deep: hex(0x137A58),
    accent: hex(0x5EEAD4),
    bg_deep: hex(0x08110E),
    bg_base: hex(0x0C1613),
    bg_panel: hex(0x101C18),
    bg_raised: hex(0x16261F),
    bg_hover: hex(0x1D3329),
    bg_active: hex(0x1F4033),
    border: hex(0x1B2E26),
    border_strong: hex(0x2B4639),
    fg: hex(0xE4F2EC),
    muted: hex(0x8DAB9E),
    faint: hex(0x5B7A6C),
    ..DARK
};

const LIGHT: Palette = Palette {
    brand_cyan: hex(0x0891B2),
    brand_blue: hex(0x1D6FE0),
    brand_deep: hex(0x1557F0),
    accent: hex(0x0A7EA4),
    bg_deep: hex(0xE9EDF3),
    bg_base: hex(0xF7F9FC),
    bg_panel: hex(0xEEF2F7),
    bg_raised: hex(0xFFFFFF),
    bg_hover: hex(0xE3E9F2),
    bg_active: hex(0xD5E3F8),
    border: hex(0xDCE3EE),
    border_strong: hex(0xC3CEDE),
    fg: hex(0x16202F),
    muted: hex(0x4F5F78),
    faint: hex(0x7B8AA3),
    success: hex(0x168A54),
    warning: hex(0xB26B00),
    danger: hex(0xC93642),
    tint_orange: hex(0xC4631F),
    tint_yellow: hex(0x9A7B0A),
    tint_green: hex(0x2E8B57),
    tint_purple: hex(0x7A52D1),
};

const CONTRAST: Palette = Palette {
    brand_cyan: hex(0x00E5FF),
    brand_blue: hex(0x2E9BFF),
    brand_deep: hex(0x0A5FD0),
    accent: hex(0xFFD400),
    bg_deep: hex(0x000000),
    bg_base: hex(0x000000),
    bg_panel: hex(0x050505),
    bg_raised: hex(0x0F0F0F),
    bg_hover: hex(0x242424),
    bg_active: hex(0x1B3A66),
    border: hex(0x6B6B6B),
    border_strong: hex(0xBFBFBF),
    fg: hex(0xFFFFFF),
    muted: hex(0xE0E0E0),
    faint: hex(0xB5B5B5),
    success: hex(0x4DFF9A),
    warning: hex(0xFFD400),
    danger: hex(0xFF6B6B),
    tint_orange: hex(0xFFA060),
    tint_yellow: hex(0xFFE04D),
    tint_green: hex(0x7DFFB0),
    tint_purple: hex(0xC9A8FF),
};

/// Stable id, name shown and palette of every theme.
pub(in crate::interfaces::studio) const THEMES: &[(&str, &str, Palette)] = &[
    ("chxchx", "ChxChx Oscuro", DARK),
    ("midnight", "Medianoche", MIDNIGHT),
    ("graphite", "Grafito", GRAPHITE),
    ("forest", "Bosque", FOREST),
    ("light", "Claro", LIGHT),
    ("contrast", "Alto contraste", CONTRAST),
];

/// Highlight colors offered next to the theme's own.
pub(in crate::interfaces::studio) const ACCENTS: &[(&str, u32)] = &[
    ("Cian", 0x22D3FF),
    ("Azul", 0x4C9AFF),
    ("Violeta", 0xA78BFA),
    ("Rosa", 0xF472B6),
    ("Naranja", 0xFB923C),
    ("Verde", 0x34D399),
];

static ACTIVE: RwLock<Palette> = RwLock::new(DARK);

pub(in crate::interfaces::studio) fn active() -> Palette {
    ACTIVE.read().map_or(DARK, |palette| *palette)
}

pub(in crate::interfaces::studio) fn set_palette(palette: Palette) {
    if let Ok(mut slot) = ACTIVE.write() {
        *slot = palette;
    }
}

/// `#RRGGBB` (or `RRGGBB`) as a color.
pub(in crate::interfaces::studio) fn parse_hex(value: &str) -> Option<Color> {
    let digits = value.trim().trim_start_matches('#');
    (digits.len() == 6 && digits.is_ascii())
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
        .map(hex)
}

fn mix(from: Color, to: Color, amount: f32) -> Color {
    let lerp = |a: f32, b: f32| a + (b - a) * amount;
    Color::from_rgb(lerp(from.r, to.r), lerp(from.g, to.g), lerp(from.b, to.b))
}

/// The palette of a theme (the default one when the id is unknown) with the person's accent.
pub(in crate::interfaces::studio) fn build(theme: &str, accent: Option<&str>) -> Palette {
    let mut palette = THEMES
        .iter()
        .find(|(id, ..)| *id == theme)
        .map_or(DARK, |(.., palette)| *palette);
    if let Some(color) = accent.and_then(parse_hex) {
        palette.accent = color;
        palette.brand_cyan = color;
        palette.brand_blue = mix(color, Color::BLACK, 0.12);
        palette.brand_deep = mix(color, Color::BLACK, 0.38);
    }
    palette
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_themes_fall_back_and_an_accent_replaces_the_highlight() {
        assert_eq!(build("nope", None), DARK);
        let custom = build("light", Some("#ff0000"));
        assert_eq!(custom.accent, hex(0xFF0000));
        assert_eq!(custom.bg_base, LIGHT.bg_base);
        assert_eq!(build("light", Some("zzz")), LIGHT);
    }
}
