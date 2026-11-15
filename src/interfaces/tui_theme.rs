//! Colors and small building blocks shared by every TUI page: the same palette as Studio.
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders};

const fn hex(value: u32) -> Color {
    Color::Rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

pub const CYAN: Color = hex(0x2FD3F0);
pub const BLUE: Color = hex(0x3B82F6);
pub const SUCCESS: Color = hex(0x3DD68C);
pub const WARNING: Color = hex(0xF5B84B);
pub const DANGER: Color = hex(0xF2707A);
pub const PURPLE: Color = hex(0xB08CFF);
pub const ORANGE: Color = hex(0xF59E6B);
pub const TEXT: Color = hex(0xE6EDF7);
pub const MUTED: Color = hex(0x8FA1BC);
pub const FAINT: Color = hex(0x5C6F8C);
pub const RAISED: Color = hex(0x121C2E);
pub const ACTIVE: Color = hex(0x1A2C4A);
pub const BORDER: Color = hex(0x2A3B59);

pub fn text() -> Style {
    Style::new().fg(TEXT)
}

pub fn muted() -> Style {
    Style::new().fg(MUTED)
}

pub fn faint() -> Style {
    Style::new().fg(FAINT)
}

/// A rounded panel; the title sits on the border.
pub fn panel<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(BORDER))
        .title(title.into())
        .title_style(Style::new().fg(CYAN).bold())
}

/// The same panel, lit up because it has the focus.
pub fn focused_panel<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    panel(title).border_style(Style::new().fg(CYAN))
}

/// ` label ` on a raised background, in the color of its meaning.
pub fn badge(label: impl Into<String>, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", label.into()),
        Style::new().fg(color).bg(RAISED).bold(),
    )
}

/// `✓ text` / `✗ text` / `· text` for a yes/no/unknown state.
pub fn check(state: Option<bool>, label: impl Into<String>) -> Line<'static> {
    let (mark, color) = match state {
        Some(true) => ("✓", SUCCESS),
        Some(false) => ("✗", DANGER),
        None => ("·", FAINT),
    };
    Line::from(vec![
        Span::styled(format!("{mark} "), Style::new().fg(color).bold()),
        Span::styled(label.into(), text()),
    ])
}

/// A key hint such as `[Tab] Cambiar`.
pub fn key_hint(key: &str, label: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!(" {key} "),
            Style::new().fg(Color::Black).bg(CYAN).bold(),
        ),
        Span::styled(format!(" {label}  "), muted()),
    ]
}

/// Blends two colors; only RGB colors blend, anything else keeps `from`.
pub fn mix(from: Color, to: Color, amount: f32) -> Color {
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount).round() as u8;
            Color::Rgb(lerp(r1, r2), lerp(g1, g2), lerp(b1, b2))
        }
        _ => from,
    }
}

/// Breaks a long string to `width` columns without splitting words when it can.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_blend_between_two_rgb_values() {
        assert_eq!(
            mix(hex(0x000000), hex(0xFFFFFF), 0.5),
            Color::Rgb(128, 128, 128)
        );
        assert_eq!(mix(Color::Red, hex(0xFFFFFF), 0.5), Color::Red);
    }

    #[test]
    fn text_wraps_at_word_boundaries() {
        assert_eq!(
            wrap("uno dos tres cuatro", 8),
            ["uno dos", "tres", "cuatro"]
        );
        assert!(wrap("", 10).is_empty());
    }
}
