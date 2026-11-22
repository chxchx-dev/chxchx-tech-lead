//! A person's own identity: the name in block letters, a short line under it, a picture and the
//! colors (ADR 0017). Studio, the node and the web client all draw from this one definition, so
//! changing it in one place changes it everywhere.
//!
//! Only pure rules live here (what a name may be, how it is drawn, which colors a theme has), so the
//! same code builds for the desktop and for WebAssembly. Reading and writing files is the product's.

use serde::{Deserialize, Serialize};

pub const DEFAULT_NAME: &str = "CHXCHX";
pub const DEFAULT_TAGLINE: &str = "control plane local para proyectos con agentes de IA";
/// The accent of the ChxChx brand, when the person has not picked another.
pub const DEFAULT_ACCENT: &str = "#00d9ff";
/// Longest name that still reads on a phone once drawn in block letters.
pub const MAX_NAME: usize = 12;
pub const MAX_TAGLINE: usize = 64;
/// Rows of every block-letter glyph.
pub const GLYPH_ROWS: usize = 6;

/// What the person chose. The picture is a file next to this; `avatar_rev` says it exists and
/// changes whenever it is replaced, so a client never shows a stale one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Identity {
    pub name: String,
    pub tagline: String,
    /// `"auto"`: the web client follows the device (light or dark). `"studio"`: it uses the theme
    /// chosen in Studio.
    pub web_theme: String,
    pub avatar_rev: Option<u64>,
}

impl Default for Identity {
    fn default() -> Self {
        Self {
            name: DEFAULT_NAME.to_owned(),
            tagline: DEFAULT_TAGLINE.to_owned(),
            web_theme: "auto".to_owned(),
            avatar_rev: None,
        }
    }
}

/// What the node tells the web client, ready to draw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityView {
    pub name: String,
    pub tagline: String,
    /// The name in block letters, one string per row.
    pub wordmark: Vec<String>,
    /// The accent, `#rrggbb`.
    pub accent: String,
    pub avatar_rev: Option<u64>,
}

impl Identity {
    /// The same identity with anything invalid put back to its default, and the name upper case.
    pub fn sanitized(&self) -> Self {
        Self {
            name: normalize_name(&self.name).unwrap_or_else(|_| DEFAULT_NAME.to_owned()),
            tagline: normalize_tagline(&self.tagline)
                .unwrap_or_else(|_| DEFAULT_TAGLINE.to_owned()),
            web_theme: if self.web_theme == "studio" {
                "studio".to_owned()
            } else {
                "auto".to_owned()
            },
            avatar_rev: self.avatar_rev,
        }
    }

    pub fn view(&self, accent: Option<&str>) -> IdentityView {
        let clean = self.sanitized();
        IdentityView {
            wordmark: wordmark(&clean.name),
            name: clean.name,
            tagline: clean.tagline,
            accent: accent
                .and_then(normalize_accent)
                .unwrap_or_else(|| DEFAULT_ACCENT.to_owned()),
            avatar_rev: clean.avatar_rev,
        }
    }
}

/// Upper case, letters, digits, spaces, `-` and `.`; between 1 and [`MAX_NAME`] characters.
pub fn normalize_name(input: &str) -> Result<String, String> {
    let name: String = input
        .trim()
        .chars()
        .flat_map(char::to_uppercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        return Err("El nombre no puede estar vacío".to_owned());
    }
    if name.chars().count() > MAX_NAME {
        return Err(format!("El nombre admite hasta {MAX_NAME} caracteres"));
    }
    if let Some(bad) = name.chars().find(|c| glyph(*c).is_none()) {
        return Err(format!(
            "«{bad}» no se puede dibujar: usa letras sin tilde, números, espacio, «-» o «.»"
        ));
    }
    Ok(name)
}

/// A short line, no control characters. Empty means "the default one".
pub fn normalize_tagline(input: &str) -> Result<String, String> {
    let line = input.trim();
    if line.is_empty() {
        return Ok(DEFAULT_TAGLINE.to_owned());
    }
    if line.chars().count() > MAX_TAGLINE {
        return Err(format!("El lema admite hasta {MAX_TAGLINE} caracteres"));
    }
    if line.chars().any(char::is_control) {
        return Err("El lema no puede llevar saltos de línea ni caracteres de control".to_owned());
    }
    Ok(line.to_owned())
}

/// `#RRGGBB` (any case, with or without the `#`) as lower-case `#rrggbb`.
pub fn normalize_accent(input: &str) -> Option<String> {
    let hex = input.trim().trim_start_matches('#');
    (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| format!("#{}", hex.to_ascii_lowercase()))
}

// ---- block letters ---------------------------------------------------------------------------

const SPACE: [&str; GLYPH_ROWS] = ["   ", "   ", "   ", "   ", "   ", "   "];

/// The font: six rows per character, drawn with block and double-line characters.
const GLYPHS: &[(char, [&str; GLYPH_ROWS])] = &[
    (
        'A',
        [
            " █████╗ ",
            "██╔══██╗",
            "███████║",
            "██╔══██║",
            "██║  ██║",
            "╚═╝  ╚═╝",
        ],
    ),
    (
        'B',
        [
            "██████╗ ",
            "██╔══██╗",
            "██████╔╝",
            "██╔══██╗",
            "██████╔╝",
            "╚═════╝ ",
        ],
    ),
    (
        'C',
        [
            " ██████╗",
            "██╔════╝",
            "██║     ",
            "██║     ",
            "╚██████╗",
            " ╚═════╝",
        ],
    ),
    (
        'D',
        [
            "██████╗ ",
            "██╔══██╗",
            "██║  ██║",
            "██║  ██║",
            "██████╔╝",
            "╚═════╝ ",
        ],
    ),
    (
        'E',
        [
            "███████╗",
            "██╔════╝",
            "█████╗  ",
            "██╔══╝  ",
            "███████╗",
            "╚══════╝",
        ],
    ),
    (
        'F',
        [
            "███████╗",
            "██╔════╝",
            "█████╗  ",
            "██╔══╝  ",
            "██║     ",
            "╚═╝     ",
        ],
    ),
    (
        'G',
        [
            " ██████╗ ",
            "██╔════╝ ",
            "██║  ███╗",
            "██║   ██║",
            "╚██████╔╝",
            " ╚═════╝ ",
        ],
    ),
    (
        'H',
        [
            "██╗  ██╗",
            "██║  ██║",
            "███████║",
            "██╔══██║",
            "██║  ██║",
            "╚═╝  ╚═╝",
        ],
    ),
    ('I', ["██╗", "██║", "██║", "██║", "██║", "╚═╝"]),
    (
        'J',
        [
            "     ██╗",
            "     ██║",
            "     ██║",
            "██   ██║",
            "╚█████╔╝",
            " ╚════╝ ",
        ],
    ),
    (
        'K',
        [
            "██╗  ██╗",
            "██║ ██╔╝",
            "█████╔╝ ",
            "██╔═██╗ ",
            "██║  ██╗",
            "╚═╝  ╚═╝",
        ],
    ),
    (
        'L',
        [
            "██╗     ",
            "██║     ",
            "██║     ",
            "██║     ",
            "███████╗",
            "╚══════╝",
        ],
    ),
    (
        'M',
        [
            "███╗   ███╗",
            "████╗ ████║",
            "██╔████╔██║",
            "██║╚██╔╝██║",
            "██║ ╚═╝ ██║",
            "╚═╝     ╚═╝",
        ],
    ),
    (
        'N',
        [
            "███╗   ██╗",
            "████╗  ██║",
            "██╔██╗ ██║",
            "██║╚██╗██║",
            "██║ ╚████║",
            "╚═╝  ╚═══╝",
        ],
    ),
    (
        'O',
        [
            " ██████╗ ",
            "██╔═══██╗",
            "██║   ██║",
            "██║   ██║",
            "╚██████╔╝",
            " ╚═════╝ ",
        ],
    ),
    (
        'P',
        [
            "██████╗ ",
            "██╔══██╗",
            "██████╔╝",
            "██╔═══╝ ",
            "██║     ",
            "╚═╝     ",
        ],
    ),
    (
        'Q',
        [
            " ██████╗ ",
            "██╔═══██╗",
            "██║   ██║",
            "██║▄▄ ██║",
            "╚██████╔╝",
            " ╚══▀▀═╝ ",
        ],
    ),
    (
        'R',
        [
            "██████╗ ",
            "██╔══██╗",
            "██████╔╝",
            "██╔══██╗",
            "██║  ██║",
            "╚═╝  ╚═╝",
        ],
    ),
    (
        'S',
        [
            "███████╗",
            "██╔════╝",
            "███████╗",
            "╚════██║",
            "███████║",
            "╚══════╝",
        ],
    ),
    (
        'T',
        [
            "████████╗",
            "╚══██╔══╝",
            "   ██║   ",
            "   ██║   ",
            "   ██║   ",
            "   ╚═╝   ",
        ],
    ),
    (
        'U',
        [
            "██╗   ██╗",
            "██║   ██║",
            "██║   ██║",
            "██║   ██║",
            "╚██████╔╝",
            " ╚═════╝ ",
        ],
    ),
    (
        'V',
        [
            "██╗   ██╗",
            "██║   ██║",
            "██║   ██║",
            "╚██╗ ██╔╝",
            " ╚████╔╝ ",
            "  ╚═══╝  ",
        ],
    ),
    (
        'W',
        [
            "██╗    ██╗",
            "██║    ██║",
            "██║ █╗ ██║",
            "██║███╗██║",
            "╚███╔███╔╝",
            " ╚══╝╚══╝ ",
        ],
    ),
    (
        'X',
        [
            "██╗  ██╗",
            "╚██╗██╔╝",
            " ╚███╔╝ ",
            " ██╔██╗ ",
            "██╔╝ ██╗",
            "╚═╝  ╚═╝",
        ],
    ),
    (
        'Y',
        [
            "██╗   ██╗",
            "╚██╗ ██╔╝",
            " ╚████╔╝ ",
            "  ╚██╔╝  ",
            "   ██║   ",
            "   ╚═╝   ",
        ],
    ),
    (
        'Z',
        [
            "███████╗",
            "╚══███╔╝",
            "  ███╔╝ ",
            " ███╔╝  ",
            "███████╗",
            "╚══════╝",
        ],
    ),
    (
        '0',
        [
            " ██████╗ ",
            "██╔═████╗",
            "██║██╔██║",
            "████╔╝██║",
            "╚██████╔╝",
            " ╚═════╝ ",
        ],
    ),
    ('1', [" ██╗", "███║", "╚██║", " ██║", " ██║", " ╚═╝"]),
    (
        '2',
        [
            "██████╗ ",
            "╚════██╗",
            " █████╔╝",
            "██╔═══╝ ",
            "███████╗",
            "╚══════╝",
        ],
    ),
    (
        '3',
        [
            "██████╗ ",
            "╚════██╗",
            " █████╔╝",
            " ╚═══██╗",
            "██████╔╝",
            "╚═════╝ ",
        ],
    ),
    (
        '4',
        [
            "██╗  ██╗",
            "██║  ██║",
            "███████║",
            "╚════██║",
            "     ██║",
            "     ╚═╝",
        ],
    ),
    (
        '5',
        [
            "███████╗",
            "██╔════╝",
            "███████╗",
            "╚════██║",
            "███████║",
            "╚══════╝",
        ],
    ),
    (
        '6',
        [
            " ██████╗ ",
            "██╔════╝ ",
            "███████╗ ",
            "██╔═══██╗",
            "╚██████╔╝",
            " ╚═════╝ ",
        ],
    ),
    (
        '7',
        [
            "███████╗",
            "╚════██║",
            "    ██╔╝",
            "   ██╔╝ ",
            "   ██║  ",
            "   ╚═╝  ",
        ],
    ),
    (
        '8',
        [
            " █████╗ ",
            "██╔══██╗",
            "╚█████╔╝",
            "██╔══██╗",
            "╚█████╔╝",
            " ╚════╝ ",
        ],
    ),
    (
        '9',
        [
            " █████╗ ",
            "██╔══██╗",
            "╚██████║",
            " ╚═══██║",
            " █████╔╝",
            " ╚════╝ ",
        ],
    ),
    (
        '-',
        [
            "        ",
            "        ",
            "█████╗  ",
            "╚════╝  ",
            "        ",
            "        ",
        ],
    ),
    ('.', ["   ", "   ", "   ", "   ", "██╗", "╚═╝"]),
];

fn glyph(c: char) -> Option<&'static [&'static str; GLYPH_ROWS]> {
    if c == ' ' {
        return Some(&SPACE);
    }
    GLYPHS
        .iter()
        .find(|(wanted, _)| *wanted == c)
        .map(|(_, rows)| rows)
}

/// The name drawn in block letters: [`GLYPH_ROWS`] strings of the same width. A character that
/// cannot be drawn is left out (names are validated before this).
pub fn wordmark(name: &str) -> Vec<String> {
    let mut rows = vec![String::new(); GLYPH_ROWS];
    for c in name.chars().flat_map(char::to_uppercase) {
        let Some(glyph) = glyph(c) else { continue };
        let width = glyph
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0);
        for (row, text) in rows.iter_mut().zip(glyph) {
            row.push_str(text);
            for _ in text.chars().count()..width {
                row.push(' ');
            }
        }
    }
    rows
}

// ---- colors ------------------------------------------------------------------------------------

/// `#rrggbb` as three bytes.
pub fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = normalize_accent(hex)?;
    let value = u32::from_str_radix(&hex[1..], 16).ok()?;
    Some(((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

fn to_hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn mix(a: (u8, u8, u8), b: (u8, u8, u8), toward_b: f32) -> (u8, u8, u8) {
    let blend =
        |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * toward_b).round() as u8;
    (blend(a.0, b.0), blend(a.1, b.1), blend(a.2, b.2))
}

/// The two colors of the brand gradient for an accent: the accent itself on top and, a little
/// toward blue, at the bottom. With the default cyan this is the cyan-to-blue of the terminal banner.
pub fn gradient(accent: &str) -> (String, String) {
    let top = rgb(accent)
        .or_else(|| rgb(DEFAULT_ACCENT))
        .unwrap_or((0, 217, 255));
    (to_hex(top), to_hex(mix(top, (0x2a, 0x4b, 0xff), 0.55)))
}

/// A theme of the web client, with the colors of the Studio theme of the same id.
#[derive(Clone, Copy, Debug)]
pub struct WebTheme {
    pub id: &'static str,
    pub name: &'static str,
    pub dark: bool,
    pub bg: u32,
    pub panel: u32,
    pub panel2: u32,
    pub line: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub ok: u32,
    pub warn: u32,
    pub bad: u32,
}

const fn theme(
    id: &'static str,
    name: &'static str,
    dark: bool,
    [bg, panel, panel2, line, text, muted, accent, ok, warn, bad]: [u32; 10],
) -> WebTheme {
    WebTheme {
        id,
        name,
        dark,
        bg,
        panel,
        panel2,
        line,
        text,
        muted,
        accent,
        ok,
        warn,
        bad,
    }
}

/// Same ids as the themes of Studio; a test in the product keeps the shared colors equal.
pub const WEB_THEMES: &[WebTheme] = &[
    theme(
        "chxchx",
        "ChxChx Oscuro",
        true,
        [
            0x070B14, 0x0D1524, 0x121C2E, 0x1C2940, 0xE6EDF7, 0x8FA1BC, 0x22D3FF, 0x3DD68C,
            0xF5B84B, 0xF2707A,
        ],
    ),
    theme(
        "midnight",
        "Medianoche",
        true,
        [
            0x000000, 0x0A0C12, 0x10131B, 0x161A24, 0xE8ECF4, 0x8A93A6, 0x22D3FF, 0x3DD68C,
            0xF5B84B, 0xF2707A,
        ],
    ),
    theme(
        "graphite",
        "Grafito",
        true,
        [
            0x181818, 0x252526, 0x2D2D30, 0x333333, 0xD8D8D8, 0x9A9A9A, 0x4FC1FF, 0x3DD68C,
            0xF5B84B, 0xF2707A,
        ],
    ),
    theme(
        "forest",
        "Bosque",
        true,
        [
            0x08110E, 0x101C18, 0x16261F, 0x1B2E26, 0xE4F2EC, 0x8DAB9E, 0x5EEAD4, 0x3DD68C,
            0xF5B84B, 0xF2707A,
        ],
    ),
    theme(
        "light",
        "Claro",
        false,
        [
            0xF7F9FC, 0xFFFFFF, 0xEEF2F7, 0xDCE3EE, 0x16202F, 0x4F5F78, 0x0A7EA4, 0x168A54,
            0xB26B00, 0xC93642,
        ],
    ),
    theme(
        "contrast",
        "Alto contraste",
        true,
        [
            0x000000, 0x050505, 0x0F0F0F, 0x6B6B6B, 0xFFFFFF, 0xE0E0E0, 0xFFD400, 0x4DFF9A,
            0xFFD400, 0xFF6B6B,
        ],
    ),
];

pub fn web_theme(id: &str) -> Option<&'static WebTheme> {
    WEB_THEMES.iter().find(|theme| theme.id == id)
}

fn css_hex(value: u32) -> String {
    format!("#{value:06x}")
}

/// The CSS that makes the web client look like the person's choice, served before the app starts so
/// nothing flashes. `studio_theme` is `Some(id)` when the client must use a Studio theme (otherwise
/// it follows the device); `accent` replaces the highlight in both cases.
pub fn theme_css(studio_theme: Option<&str>, accent: Option<&str>) -> String {
    let theme = studio_theme.and_then(web_theme);
    let accent = accent
        .and_then(normalize_accent)
        .or_else(|| theme.map(|theme| css_hex(theme.accent)));
    let mut css = String::new();
    if let Some(theme) = theme {
        css.push_str(&format!(
            ":root {{ color-scheme: {scheme}; --bg: {}; --panel: {}; --panel2: {}; --line: {}; --text: {}; --muted: {}; --ok: {}; --warn: {}; --bad: {}; }}\n",
            css_hex(theme.bg),
            css_hex(theme.panel),
            css_hex(theme.panel2),
            css_hex(theme.line),
            css_hex(theme.text),
            css_hex(theme.muted),
            css_hex(theme.ok),
            css_hex(theme.warn),
            css_hex(theme.bad),
            scheme = if theme.dark { "dark" } else { "light" },
        ));
    }
    if let Some(accent) = accent {
        let (top, bottom) = gradient(&accent);
        let (r, g, b) = rgb(&accent).unwrap_or((0, 217, 255));
        // Dark text on a bright accent, light text on a dark one.
        let luminance = 0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b);
        let ink = if luminance > 150.0 {
            "#00131a"
        } else {
            "#ffffff"
        };
        css.push_str(&format!(
            ":root {{ --accent: {accent}; --accent-ink: {ink}; --brand: linear-gradient(135deg, {top} 0%, {bottom} 100%); --glow: 0 0 22px rgba({r}, {g}, {b}, 0.18); }}\n"
        ));
    }
    css
}

// ---- the banner as a picture ------------------------------------------------------------------------

/// One character cell of the drawing and the thickness of its strokes.
const CELL_W: i32 = 20;
const CELL_H: i32 = 40;
const STROKE: i32 = 5;
const OFFSET: i32 = 6;

/// Rectangles, collected as SVG.
#[derive(Default)]
struct Canvas {
    shapes: String,
}

impl Canvas {
    fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.shapes.push_str(&format!(
            "<rect x=\"{x0}\" y=\"{y0}\" width=\"{}\" height=\"{}\"/>",
            x1 - x0,
            y1 - y0
        ));
    }

    /// A stroke along the row `y`, from `x0` to `x1`.
    fn across(&mut self, x0: i32, x1: i32, y: i32) {
        self.rect(x0, y - STROKE / 2, x1, y + STROKE / 2 + 1);
    }

    /// A stroke down the column `x`, from `y0` to `y1`.
    fn down(&mut self, x: i32, y0: i32, y1: i32) {
        self.rect(x - STROKE / 2, y0, x + STROKE / 2 + 1, y1);
    }
}

/// The block letters as an SVG: no font needed, so it looks the same on every device. Only box
/// characters are ever drawn (the text of the name never reaches the SVG), so it holds no markup of
/// the person's.
pub fn banner_svg(rows: &[String], accent: &str, label: &str) -> String {
    // The label is the name, already validated; anything else is dropped so it can never close the attribute.
    let label: String = label
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '.'))
        .collect();
    let (top, bottom) = gradient(accent);
    let cols = rows
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0) as i32;
    let (width, height) = (cols * CELL_W, rows.len() as i32 * CELL_H);
    let mut canvas = Canvas::default();
    let half = STROKE / 2;
    let d = OFFSET;
    for (row, line) in rows.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let (l, t) = (col as i32 * CELL_W, row as i32 * CELL_H);
            let (r, b) = (l + CELL_W, t + CELL_H);
            let (cx, cy) = (l + CELL_W / 2, t + CELL_H / 2);
            match ch {
                '█' => canvas.rect(l, t, r, b),
                '▄' => canvas.rect(l, cy, r, b),
                '▀' => canvas.rect(l, t, r, cy),
                '═' => {
                    canvas.across(l, r, cy - d);
                    canvas.across(l, r, cy + d);
                }
                '║' => {
                    canvas.down(cx - d, t, b);
                    canvas.down(cx + d, t, b);
                }
                '╔' => {
                    canvas.down(cx - d, cy - d - half, b);
                    canvas.across(cx - d - half, r, cy - d);
                    canvas.down(cx + d, cy + d - half, b);
                    canvas.across(cx + d - half, r, cy + d);
                }
                '╗' => {
                    canvas.down(cx + d, cy - d - half, b);
                    canvas.across(l, cx + d + half + 1, cy - d);
                    canvas.down(cx - d, cy + d - half, b);
                    canvas.across(l, cx - d + half + 1, cy + d);
                }
                '╚' => {
                    canvas.down(cx - d, t, cy + d + half + 1);
                    canvas.across(cx - d - half, r, cy + d);
                    canvas.down(cx + d, t, cy - d + half + 1);
                    canvas.across(cx + d - half, r, cy - d);
                }
                '╝' => {
                    canvas.down(cx + d, t, cy + d + half + 1);
                    canvas.across(l, cx + d + half + 1, cy + d);
                    canvas.down(cx - d, t, cy - d + half + 1);
                    canvas.across(l, cx - d + half + 1, cy - d);
                }
                _ => {}
            }
        }
    }
    let shapes = canvas.shapes;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-12 -12 {} {}\" role=\"img\" aria-label=\"{label}\">\
<defs><linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"{height}\" gradientUnits=\"userSpaceOnUse\">\
<stop offset=\"0\" stop-color=\"{top}\"/><stop offset=\"1\" stop-color=\"{bottom}\"/></linearGradient>\
<filter id=\"b\" x=\"-10%\" y=\"-20%\" width=\"120%\" height=\"140%\"><feGaussianBlur stdDeviation=\"7\"/></filter></defs>\
<g fill=\"url(#g)\" filter=\"url(#b)\" opacity=\"0.55\">{shapes}</g><g fill=\"url(#g)\" shape-rendering=\"crispEdges\">{shapes}</g></svg>",
        width + 24,
        height + 24,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_is_six_rows_of_one_width() {
        for (c, rows) in GLYPHS {
            let width = rows[0].chars().count();
            assert!(width >= 3, "{c}");
            for row in rows {
                assert_eq!(row.chars().count(), width, "{c}: {row:?}");
            }
        }
    }

    /// To look at the font: `cargo test -p chxchx-protocol show_every_glyph -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn show_every_glyph() {
        for group in ["ABCDEFGHIJKLM", "NOPQRSTUVWXYZ", "0123456789-."] {
            for row in wordmark(group) {
                println!("{row}");
            }
            println!();
        }
    }

    #[test]
    fn every_letter_and_digit_has_a_glyph() {
        for c in ('A'..='Z').chain('0'..='9') {
            assert!(glyph(c).is_some(), "{c}");
        }
        assert!(glyph('-').is_some() && glyph('.').is_some() && glyph(' ').is_some());
    }

    #[test]
    fn the_default_name_draws_the_banner_of_the_terminal() {
        let rows = wordmark(DEFAULT_NAME);
        assert_eq!(rows.len(), GLYPH_ROWS);
        assert_eq!(rows[0], " ██████╗██╗  ██╗██╗  ██╗ ██████╗██╗  ██╗██╗  ██╗");
        assert_eq!(rows[5], " ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝");
        let width = rows[0].chars().count();
        assert!(
            rows.iter().all(|row| row.chars().count() == width),
            "a clean rectangle"
        );
    }

    #[test]
    fn a_name_is_upper_cased_trimmed_and_limited() {
        assert_eq!(
            normalize_name("  ana  pérez ".replace('é', "e").as_str()).unwrap(),
            "ANA PEREZ"
        );
        assert_eq!(normalize_name("dev-01").unwrap(), "DEV-01");
        assert!(normalize_name("").is_err());
        assert!(normalize_name("   ").is_err());
        assert!(normalize_name("DEMASIADOS CARACTERES").is_err());
        assert!(
            normalize_name("ñandú")
                .unwrap_err()
                .contains("no se puede dibujar")
        );
        assert!(normalize_name("<b>").is_err(), "markup never gets through");
    }

    #[test]
    fn a_wide_name_still_draws_every_character() {
        let rows = wordmark(&normalize_name("MMMMMMMMMMMM").unwrap());
        assert!(rows[0].chars().count() >= 12 * 11);
    }

    #[test]
    fn the_tagline_falls_back_and_refuses_control_characters() {
        assert_eq!(normalize_tagline("   ").unwrap(), DEFAULT_TAGLINE);
        assert_eq!(normalize_tagline(" hola ").unwrap(), "hola");
        assert!(normalize_tagline("uno\ndos").is_err());
        assert!(normalize_tagline(&"x".repeat(MAX_TAGLINE + 1)).is_err());
    }

    #[test]
    fn an_accent_is_read_in_any_case_with_or_without_the_hash() {
        assert_eq!(normalize_accent("#00D9FF").as_deref(), Some("#00d9ff"));
        assert_eq!(normalize_accent("00d9ff").as_deref(), Some("#00d9ff"));
        assert_eq!(normalize_accent("#00d9f"), None);
        assert_eq!(normalize_accent("#zzzzzz"), None);
        assert_eq!(rgb("#ff8000"), Some((255, 128, 0)));
    }

    #[test]
    fn the_default_gradient_is_the_cyan_to_blue_of_the_brand() {
        let (top, bottom) = gradient(DEFAULT_ACCENT);
        assert_eq!(top, "#00d9ff");
        assert_eq!(bottom, "#178bff");
        let (_, pink_bottom) = gradient("#f472b6");
        assert_ne!(pink_bottom, bottom, "another accent, another gradient");
    }

    #[test]
    fn invalid_things_in_a_saved_identity_become_defaults_instead_of_breaking_the_screen() {
        let broken = Identity {
            name: "ñ<script>".to_owned(),
            tagline: "a\u{7}b".to_owned(),
            web_theme: "rojo".to_owned(),
            avatar_rev: Some(3),
        };
        let clean = broken.sanitized();
        assert_eq!(clean.name, DEFAULT_NAME);
        assert_eq!(clean.tagline, DEFAULT_TAGLINE);
        assert_eq!(clean.web_theme, "auto");
        assert_eq!(clean.avatar_rev, Some(3));
        let view = broken.view(Some("rojo"));
        assert_eq!(view.accent, DEFAULT_ACCENT);
        assert_eq!(view.wordmark.len(), GLYPH_ROWS);
    }

    #[test]
    fn the_css_follows_the_device_unless_a_studio_theme_is_asked_for() {
        assert_eq!(
            theme_css(None, None),
            "",
            "nothing chosen: the app's own look"
        );
        let accent_only = theme_css(None, Some("#f472b6"));
        assert!(accent_only.contains("--accent: #f472b6") && !accent_only.contains("--bg"));
        let themed = theme_css(Some("light"), None);
        assert!(themed.contains("color-scheme: light") && themed.contains("--bg: #f7f9fc"));
        assert!(
            themed.contains("--accent: #0a7ea4"),
            "the theme's own highlight"
        );
        let both = theme_css(Some("graphite"), Some("#ff0000"));
        assert!(
            both.contains("--accent: #ff0000"),
            "the person's accent wins: {both}"
        );
        assert_eq!(theme_css(Some("nope"), None), "");
    }

    #[test]
    fn a_bright_accent_gets_dark_ink_and_a_dark_one_light_ink() {
        assert!(theme_css(None, Some("#ffd400")).contains("--accent-ink: #00131a"));
        assert!(theme_css(None, Some("#1a2a8c")).contains("--accent-ink: #ffffff"));
    }

    #[test]
    fn the_svg_draws_only_shapes_and_never_the_text_of_the_name() {
        let svg = banner_svg(&wordmark("ana"), DEFAULT_ACCENT, "ANA");
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(
            svg.contains("aria-label=\"ANA\""),
            "the name is its accessible text"
        );
        let hostile = banner_svg(&wordmark("ana"), DEFAULT_ACCENT, "\"><script>x</script>");
        assert!(!hostile.contains("<script"), "no tag survives");
        assert!(
            hostile.contains("aria-label=\"scriptxscript\""),
            "only harmless characters remain"
        );
        assert!(svg.contains("<rect") && svg.contains("linearGradient"));
        assert!(
            !svg.contains("<text"),
            "no text in the picture: only shapes"
        );
        assert!(!svg.contains("script"));
    }

    #[test]
    fn every_web_theme_has_a_distinct_id_and_readable_text() {
        let mut ids: Vec<_> = WEB_THEMES.iter().map(|theme| theme.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), WEB_THEMES.len());
        for theme in WEB_THEMES {
            assert_ne!(theme.bg, theme.text, "{}", theme.id);
            assert_eq!(web_theme(theme.id).map(|t| t.name), Some(theme.name));
        }
    }
}
