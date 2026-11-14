//! What the terminal looks like: the character grid of one session.
use super::terminal_surface::Surface;
use super::*;
use crate::integrations::terminal::{Color as TermColor, Run, Snapshot};
use iced::font::{Style as FontStyle, Weight};
use iced::widget::text::{LineHeight, Span, Wrapping};
use iced::widget::{column, rich_text, span};
use iced::{Color, Font, Pixels};

/// The 16 ANSI colors, tuned for the dark surface (index 0-7 normal, 8-15 bright).
fn ansi(index: u8) -> Color {
    const PALETTE: [u32; 16] = [
        0x1B2230, 0xF2707A, 0x3DD68C, 0xF5B84B, 0x5AA2FF, 0xB08CFF, 0x22D3FF, 0xC9D4E5, 0x4A5A75,
        0xFF8F98, 0x6EE7A8, 0xFFD079, 0x86BBFF, 0xC9ADFF, 0x67E0FF, 0xF1F5FB,
    ];
    let rgb = |value: u32| Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8);
    match index {
        0..=15 => rgb(PALETTE[usize::from(index)]),
        // The 6x6x6 color cube.
        16..=231 => {
            let n = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            Color::from_rgb8(level(n / 36), level(n / 6 % 6), level(n % 6))
        }
        // The grayscale ramp.
        _ => {
            let gray = 8 + 10 * (index - 232);
            Color::from_rgb8(gray, gray, gray)
        }
    }
}

pub(super) fn resolve(color: TermColor, default: Color) -> Color {
    match color {
        TermColor::Default => default,
        TermColor::Indexed(index) => ansi(index),
        TermColor::Rgb(r, g, b) => Color::from_rgb8(r, g, b),
    }
}

/// `(foreground, background)` of a run; `None` background means the terminal's own.
pub(super) fn colors(run: &Run, focused: bool) -> (Color, Option<Color>) {
    let fg = resolve(run.fg, design::TEXT);
    let bg = match run.bg {
        TermColor::Default => None,
        other => Some(resolve(other, design::BG_DEEP)),
    };
    // The cursor is a block while the terminal has the focus; the program's own reverse video flips it back.
    if run.inverse != (run.cursor && focused) {
        (bg.unwrap_or(design::BG_DEEP), Some(fg))
    } else {
        (fg, bg)
    }
}

fn spans(row: &[Run], focused: bool) -> Vec<Span<'static, (), Font>> {
    row.iter()
        .map(|run| {
            let (fg, bg) = colors(run, focused);
            let font = Font {
                weight: if run.bold {
                    Weight::Bold
                } else {
                    Weight::Normal
                },
                style: if run.italic {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                },
                ..design::MONO
            };
            let mut piece = span::<(), Font>(run.text.clone())
                .color(fg)
                .font(font)
                .underline(run.underline || (run.cursor && !focused));
            if let Some(bg) = bg {
                piece = piece.background(bg);
            }
            piece
        })
        .collect()
}

fn grid<'a>(snapshot: &Snapshot, focused: bool) -> Element<'a, Message> {
    let mut lines = column![];
    for row in &snapshot.rows {
        lines = lines.push(
            rich_text(spans(row, focused))
                .font(design::MONO)
                .size(FONT_SIZE)
                .line_height(LineHeight::Absolute(Pixels(LINE_HEIGHT)))
                .wrapping(Wrapping::None),
        );
    }
    lines.into()
}

pub(in crate::interfaces::studio) fn view(
    state: &Dashboard,
    id: Option<u64>,
) -> Element<'_, Message> {
    let terminal = &state.terminal;
    let Some((index, tab)) = id.and_then(|id| {
        terminal
            .tabs
            .iter()
            .enumerate()
            .find(|(_, tab)| tab.id == id)
    }) else {
        let hint = if terminal.opening {
            "Iniciando…"
        } else {
            "Abre un shell del proyecto o un agente. Se requiere confianza local."
        };
        let mut empty = column![
            icons::icon("terminal", design::FAINT, 28.0),
            text(hint).size(12).color(design::MUTED),
        ]
        .spacing(10)
        .align_x(iced::Center);
        if !terminal.opening {
            empty = empty.push(
                button(text("Abrir terminal").size(12))
                    .padding([7, 14])
                    .style(styles::primary)
                    .on_press(Message::Terminal(Event::Open)),
            );
        }
        return container(empty).center(Fill).into();
    };
    let focused = terminal.focused && terminal.active == index;
    let tab_id = tab.id;
    let surface = Surface::new(
        surface_id(tab_id),
        container(grid(&tab.snapshot, focused))
            .padding(PADDING)
            .width(Fill)
            .height(Fill)
            .clip(true)
            .style(styles::well),
        focused,
        move |focused| Message::Terminal(Event::Focus(tab_id, focused)),
        move |key| Message::Terminal(Event::Key(tab_id, key)),
        move |lines| Message::Terminal(Event::Scroll(tab_id, lines)),
        move |size| Message::Terminal(Event::Resized(tab_id, size)),
    );
    container(Element::from(surface))
        .padding(6)
        .width(Fill)
        .height(Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(fg: TermColor, bg: TermColor, inverse: bool, cursor: bool) -> Run {
        Run {
            text: "x".into(),
            fg,
            bg,
            bold: false,
            italic: false,
            underline: false,
            inverse,
            cursor,
        }
    }

    #[test]
    fn ansi_colors_cover_the_palette_the_cube_and_the_grays() {
        assert_eq!(ansi(1), Color::from_rgb8(0xF2, 0x70, 0x7A));
        assert_eq!(ansi(16), Color::from_rgb8(0, 0, 0));
        assert_eq!(ansi(231), Color::from_rgb8(255, 255, 255));
        assert_eq!(ansi(196), Color::from_rgb8(255, 0, 0));
        assert_eq!(ansi(232), Color::from_rgb8(8, 8, 8));
        assert_eq!(ansi(255), Color::from_rgb8(238, 238, 238));
        assert_eq!(
            resolve(TermColor::Rgb(1, 2, 3), design::TEXT),
            Color::from_rgb8(1, 2, 3)
        );
        assert_eq!(resolve(TermColor::Default, design::TEXT), design::TEXT);
    }

    #[test]
    fn plain_text_uses_the_terminal_colors_and_no_background() {
        assert_eq!(
            colors(
                &run(TermColor::Default, TermColor::Default, false, false),
                true
            ),
            (design::TEXT, None)
        );
    }

    #[test]
    fn the_cursor_is_a_block_only_while_the_terminal_has_the_focus() {
        let cursor = run(TermColor::Default, TermColor::Default, false, true);
        assert_eq!(colors(&cursor, true), (design::BG_DEEP, Some(design::TEXT)));
        assert_eq!(colors(&cursor, false), (design::TEXT, None));
    }

    #[test]
    fn reverse_video_swaps_and_the_cursor_on_it_swaps_back() {
        let reversed = run(TermColor::Indexed(2), TermColor::Indexed(4), true, false);
        assert_eq!(colors(&reversed, true), (ansi(4), Some(ansi(2))));
        let both = run(TermColor::Indexed(2), TermColor::Indexed(4), true, true);
        assert_eq!(colors(&both, true), (ansi(2), Some(ansi(4))));
    }
}
