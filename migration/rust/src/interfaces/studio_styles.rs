//! Widget styles built on the design tokens. Presentation only.
use super::design::*;
use super::{Element, Message};
use iced::widget::{button, container, scrollable, text_editor, text_input, tooltip};
use iced::{Background, Border, Color, Shadow, Theme, Vector, gradient};

fn solid(color: Color) -> Option<Background> {
    Some(Background::Color(color))
}

fn outline(color: Color, radius: f32) -> Border {
    Border {
        color,
        width: 1.0,
        radius: radius.into(),
    }
}

fn brand_gradient(from: Color, to: Color) -> Background {
    Background::Gradient(iced::Gradient::Linear(
        gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(0.0, from)
            .add_stop(1.0, to),
    ))
}

/// A flat surface painted with one of the background tokens.
pub(super) fn surface(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: solid(color),
        ..Default::default()
    }
}

pub(super) fn card(_: &Theme) -> container::Style {
    container::Style {
        background: solid(BG_RAISED),
        border: outline(BORDER, 10.0),
        ..Default::default()
    }
}

pub(super) fn well(_: &Theme) -> container::Style {
    container::Style {
        background: solid(BG_DEEP),
        border: outline(BORDER, 8.0),
        ..Default::default()
    }
}

pub(super) fn hero(_: &Theme) -> container::Style {
    container::Style {
        background: Some(brand_gradient(hex_mix(BRAND_DEEP, 0.34), BG_RAISED)),
        border: outline(alpha(BRAND_BLUE, 0.45), 14.0),
        ..Default::default()
    }
}

/// Blends a brand color into the base surface without using transparency.
fn hex_mix(color: Color, amount: f32) -> Color {
    let mix = |a: f32, b: f32| b + (a - b) * amount;
    Color::from_rgb(
        mix(color.r, BG_BASE.r),
        mix(color.g, BG_BASE.g),
        mix(color.b, BG_BASE.b),
    )
}

/// A card tinted with `color`: notices, guidance and confirmations.
pub(super) fn tinted(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: solid(hex_mix(color, 0.12)),
        border: outline(hex_mix(color, 0.5), 10.0),
        ..Default::default()
    }
}

/// Multi-line message box (commit message).
pub(super) fn message_box(_: &Theme, status: text_editor::Status) -> text_editor::Style {
    let focused = matches!(status, text_editor::Status::Focused { .. });
    text_editor::Style {
        background: Background::Color(BG_DEEP),
        border: outline(if focused { BRAND_BLUE } else { BORDER }, 6.0),
        placeholder: FAINT,
        value: TEXT,
        selection: alpha(BRAND_BLUE, 0.4),
    }
}

pub(super) fn brand_fill(_: &Theme) -> container::Style {
    container::Style {
        background: Some(brand_gradient(BRAND_BLUE, BRAND_DEEP)),
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

/// A rounded pill tinted with `color`, used for chips and status badges.
pub(super) fn chip(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: solid(hex_mix(color, 0.14)),
        border: outline(hex_mix(color, 0.42), 999.0),
        text_color: Some(color),
        ..Default::default()
    }
}

pub(super) fn tooltip_box(_: &Theme) -> container::Style {
    container::Style {
        background: solid(BG_HOVER),
        border: outline(BORDER_STRONG, 6.0),
        text_color: Some(TEXT),
        ..Default::default()
    }
}

pub(super) fn modal(_: &Theme) -> container::Style {
    container::Style {
        background: solid(BG_RAISED),
        border: outline(BORDER_STRONG, 12.0),
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.55),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    }
}

/// Dropdown menu surface: flat with a strong border. A blurred shadow is avoided on purpose;
/// the software renderer paints it over the panel when it overlaps other widgets.
pub(super) fn menu_panel(_: &Theme) -> container::Style {
    container::Style {
        background: solid(BG_RAISED),
        border: outline(BORDER_STRONG, 8.0),
        ..Default::default()
    }
}

pub(super) fn splitter(active: bool) -> impl Fn(&Theme) -> container::Style {
    surface(if active { BRAND_BLUE } else { BG_DEEP })
}

/// Borderless button for toolbars, rails and tree rows.
pub(super) fn ghost(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let disabled = matches!(status, button::Status::Disabled);
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: match (active, hovered) {
                (true, _) => solid(BG_ACTIVE),
                (false, true) => solid(BG_HOVER),
                (false, false) => None,
            },
            text_color: if disabled {
                FAINT
            } else if active || hovered {
                TEXT
            } else {
                MUTED
            },
            border: Border::default().rounded(5.0),
            ..Default::default()
        }
    }
}

/// A row of a dropdown menu; disabled rows stay dim and do not react to the pointer.
pub(super) fn menu_item(_: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: hovered.then_some(Background::Color(BG_ACTIVE)),
        text_color: if matches!(status, button::Status::Disabled) {
            FAINT
        } else {
            TEXT
        },
        border: Border::default().rounded(5.0),
        ..Default::default()
    }
}

/// Text-only button for tabs: no background, the label brightens on hover.
pub(super) fn plain(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: None,
        text_color: if active || matches!(status, button::Status::Hovered | button::Status::Pressed)
        {
            TEXT
        } else {
            MUTED
        },
        ..Default::default()
    }
}

pub(super) fn primary(_: &Theme, status: button::Status) -> button::Style {
    let (from, to) = match status {
        button::Status::Hovered => (BRAND_CYAN, BRAND_BLUE),
        button::Status::Pressed => (BRAND_DEEP, BRAND_DEEP),
        _ => (BRAND_BLUE, BRAND_DEEP),
    };
    button::Style {
        background: Some(brand_gradient(from, to)),
        text_color: Color::WHITE,
        border: Border::default().rounded(7.0),
        ..Default::default()
    }
}

pub(super) fn secondary(_: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: solid(if hovered { BG_HOVER } else { BG_PANEL }),
        text_color: TEXT,
        border: outline(if hovered { BRAND_BLUE } else { BORDER_STRONG }, 7.0),
        ..Default::default()
    }
}

pub(super) fn danger(_: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: solid(hex_mix(DANGER, if hovered { 0.30 } else { 0.14 })),
        text_color: DANGER,
        border: outline(hex_mix(DANGER, 0.55), 7.0),
        ..Default::default()
    }
}

pub(super) fn input(_: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(BG_DEEP),
        border: outline(if focused { BRAND_BLUE } else { BORDER }, 6.0),
        icon: MUTED,
        placeholder: FAINT,
        value: TEXT,
        selection: alpha(BRAND_BLUE, 0.45),
    }
}

pub(super) fn editor(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(BG_BASE),
        border: Border::default(),
        placeholder: FAINT,
        value: TEXT,
        selection: alpha(BRAND_BLUE, 0.38),
    }
}

fn thin_scrollbar(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    let strength = match status {
        scrollable::Status::Active { .. } => 0.10,
        scrollable::Status::Hovered { .. } => 0.22,
        scrollable::Status::Dragged { .. } => 0.34,
    };
    for rail in [&mut style.vertical_rail, &mut style.horizontal_rail] {
        rail.background = None;
        rail.border = Border::default();
        rail.scroller.background = Background::Color(alpha(TEXT, strength));
        rail.scroller.border = Border::default().rounded(4.0);
    }
    style
}

/// Vertical scroll area with the slim Studio scrollbar.
pub(super) fn vscroll<'a>(
    content: impl Into<Element<'a, Message>>,
) -> scrollable::Scrollable<'a, Message> {
    scrollable(content)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(8).scroller_width(6),
        ))
        .style(thin_scrollbar)
}

/// Horizontal scroll area that keeps the bar out of the way (tab strips).
pub(super) fn hscroll<'a>(
    content: impl Into<Element<'a, Message>>,
) -> scrollable::Scrollable<'a, Message> {
    scrollable(content)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(3).scroller_width(3),
        ))
        .style(thin_scrollbar)
}

/// Wraps `content` with a short text tooltip.
pub(super) fn tip<'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'a str,
    position: tooltip::Position,
) -> Element<'a, Message> {
    tooltip(content, iced::widget::text(label).size(11), position)
        .gap(6)
        .padding(6)
        .style(tooltip_box)
        .into()
}
