//! What the terminal looks like: the tab strip and the character grid.
use super::terminal_surface::Surface;
use super::*;
use crate::integrations::terminal::{Color as TermColor, Run, Snapshot};
use iced::font::{Style as FontStyle, Weight};
use iced::widget::text::{LineHeight, Span, Wrapping};
use iced::widget::{column, rich_text, span, tooltip};
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

pub(in crate::interfaces::studio) fn view(state: &Dashboard) -> Element<'_, Message> {
    let terminal = &state.terminal;
    let Some(tab) = terminal.active_tab() else {
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
    let surface = Surface::new(
        iced::widget::Id::new(SURFACE),
        container(grid(&tab.snapshot, terminal.focused))
            .padding(PADDING)
            .width(Fill)
            .height(Fill)
            .clip(true)
            .style(styles::well),
        terminal.focused,
        |focused| Message::Terminal(Event::Focus(focused)),
        |key| Message::Terminal(Event::Key(key)),
        |lines| Message::Terminal(Event::Scroll(lines)),
        |size| Message::Terminal(Event::Resized(size)),
    );
    container(column![tab_strip(state), Element::from(surface)].spacing(6))
        .padding(iced::Padding {
            top: 0.0,
            right: 10.0,
            bottom: 8.0,
            left: 10.0,
        })
        .width(Fill)
        .height(Fill)
        .into()
}

fn tab_strip(state: &Dashboard) -> Element<'_, Message> {
    let terminal = &state.terminal;
    let mut strip = row![].spacing(4).align_y(iced::Center);
    for (index, tab) in terminal.tabs.iter().enumerate() {
        let active = index == terminal.active;
        let (icon, tint) = match tab.kind {
            Kind::Shell => ("terminal", design::MUTED),
            Kind::Agent(_) => ("sparkle", design::ACCENT),
        };
        let mut label = row![
            icons::icon(icon, if active { tint } else { design::FAINT }, 12.0),
            text(&tab.title).size(11).font(design::UI_BOLD),
        ]
        .spacing(5)
        .align_y(iced::Center);
        if tab.exited {
            label = label.push(text("terminó").size(10).color(design::WARNING));
        }
        strip = strip.push(
            container(
                row![
                    button(label)
                        .padding([4, 8])
                        .style(styles::ghost(active))
                        .on_press(Message::Terminal(Event::Select(index))),
                    styles::tip(
                        button(icons::icon("close", design::FAINT, 10.0))
                            .padding(3)
                            .style(styles::ghost(false))
                            .on_press(Message::Terminal(Event::CloseTab(index))),
                        "Cerrar esta sesión",
                        tooltip::Position::Top,
                    ),
                ]
                .align_y(iced::Center),
            )
            .style(styles::chip_surface(active)),
        );
    }
    strip = strip.push(styles::tip(
        button(icons::icon("plus", design::MUTED, 12.0))
            .padding(5)
            .style(styles::ghost(false))
            .on_press(Message::Terminal(Event::Open)),
        "Nuevo shell",
        tooltip::Position::Top,
    ));
    strip.into()
}

/// Session controls shown in the dock header while the terminal tab is active.
pub(in crate::interfaces::studio) fn toolbar(state: &Dashboard) -> Element<'_, Message> {
    if !state.terminal.has_sessions() {
        return Space::new().into();
    }
    styles::tip(
        button(icons::icon("stop", design::MUTED, 13.0))
            .padding(5)
            .style(styles::ghost(false))
            .on_press(Message::Terminal(Event::Interrupt)),
        "Interrumpir (Ctrl+C)",
        tooltip::Position::Top,
    )
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
