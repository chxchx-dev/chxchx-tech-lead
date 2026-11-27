//! Look of Colmena: near-black surfaces, One Dark status colors and the brand blue.
//! Presentation only; the tokens come from the active theme.
use super::design;
use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};

/// One Dark accent for each card status; "doing" takes the brand blue.
pub(super) fn status_color(status: &str) -> Color {
    match status {
        "ready" => Color::from_rgb8(0x56, 0xB6, 0xC2),
        "doing" => design::brand_blue(),
        "review" => Color::from_rgb8(0xC6, 0x78, 0xDD),
        "done" => Color::from_rgb8(0x98, 0xC3, 0x79),
        "blocked" => Color::from_rgb8(0xE0, 0x6C, 0x75),
        _ => Color::from_rgb8(0x7F, 0x84, 0x8E),
    }
}

fn edge(color: Color, radius: f32) -> Border {
    Border {
        color,
        width: 1.0,
        radius: radius.into(),
    }
}

/// A column of the board: a dark well with a hairline border.
pub(super) fn lane(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(design::bg_base())),
        border: edge(design::border(), 14.0),
        ..Default::default()
    }
}

/// A card of the board; the selected one glows with the brand blue.
pub(super) fn tile(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (background, border) = if selected {
            (
                design::bg_active(),
                edge(design::alpha(design::brand_blue(), 0.9), 10.0),
            )
        } else if hovered {
            (
                design::bg_hover(),
                edge(design::alpha(design::brand_blue(), 0.45), 10.0),
            )
        } else {
            (design::bg_panel(), edge(design::border(), 10.0))
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: design::fg(),
            border,
            ..Default::default()
        }
    }
}

/// A panel for details and decisions: raised, with a tinted hairline.
pub(super) fn panel(tint: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(design::bg_panel())),
        border: edge(design::alpha(tint, 0.35), 12.0),
        ..Default::default()
    }
}

/// The small rounded chip that carries a key or a count.
pub(super) fn chip(tint: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(design::alpha(tint, 0.14))),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
