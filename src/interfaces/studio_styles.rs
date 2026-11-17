//! Widget styles built on the design tokens. Presentation only.
use super::design::*;
use super::{Element, Message};
use iced::widget::{button, container, scrollable, text_editor, text_input, tooltip};

pub(super) const PAGE_SCROLL_ID: iced::widget::Id = iced::widget::Id::new("studio-main-page");
#[path = "studio_styles_panes.rs"]
mod panes;
use iced::{Background, Border, Color, Shadow, Theme, Vector};
#[path = "studio_styles_chat.rs"]
mod chat_styles;
pub(super) use chat_styles::{composer_card, composer_editor};
pub(super) use panes::{grid as pane_grid, pane, pane_title};

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

/// A brand "gradient" painted as one flat blend of its ends: the software renderer fills every
/// gradient pixel through a shader on each frame of a scroll, which made pages stutter.
fn brand_gradient(from: Color, to: Color) -> Background {
    let mid = |a: f32, b: f32| (a + b) / 2.0;
    Background::Color(Color::from_rgb(
        mid(from.r, to.r),
        mid(from.g, to.g),
        mid(from.b, to.b),
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
        background: solid(bg_raised()),
        border: Border::default().rounded(12.0),
        ..Default::default()
    }
}

pub(super) fn well(_: &Theme) -> container::Style {
    container::Style {
        background: solid(bg_deep()),
        border: Border::default().rounded(10.0),
        ..Default::default()
    }
}

pub(super) fn hero(_: &Theme) -> container::Style {
    container::Style {
        background: Some(brand_gradient(hex_mix(brand_deep(), 0.34), bg_raised())),
        border: outline(alpha(brand_blue(), 0.22), 16.0),
        ..Default::default()
    }
}

/// Blends a brand color into the base surface without using transparency.
fn hex_mix(color: Color, amount: f32) -> Color {
    let mix = |a: f32, b: f32| b + (a - b) * amount;
    Color::from_rgb(
        mix(color.r, bg_base().r),
        mix(color.g, bg_base().g),
        mix(color.b, bg_base().b),
    )
}

/// A card tinted with `color`: notices, guidance and confirmations.
pub(super) fn tinted(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: solid(hex_mix(color, 0.12)),
        border: outline(hex_mix(color, 0.28), 10.0),
        ..Default::default()
    }
}

/// Multi-line message box (commit message).
pub(super) fn message_box(_: &Theme, status: text_editor::Status) -> text_editor::Style {
    let focused = matches!(status, text_editor::Status::Focused { .. });
    text_editor::Style {
        background: Background::Color(bg_deep()),
        border: if focused {
            outline(alpha(brand_blue(), 0.7), 10.0)
        } else {
            Border::default().rounded(10.0)
        },
        placeholder: faint(),
        value: fg(),
        selection: alpha(brand_blue(), 0.4),
    }
}

pub(super) fn brand_fill(_: &Theme) -> container::Style {
    container::Style {
        background: Some(brand_gradient(brand_blue(), brand_deep())),
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

/// A session tab: the active one is raised, the others only keep their outline.
pub(super) fn chip_surface(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: solid(if active { bg_active() } else { bg_raised() }),
        border: Border::default().rounded(8.0),
        ..Default::default()
    }
}

pub(super) fn tooltip_box(_: &Theme) -> container::Style {
    container::Style {
        background: solid(bg_hover()),
        border: outline(border_strong(), 6.0),
        text_color: Some(fg()),
        ..Default::default()
    }
}

pub(super) fn modal(_: &Theme) -> container::Style {
    container::Style {
        background: solid(bg_raised()),
        border: outline(border_strong(), 12.0),
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
        background: solid(bg_raised()),
        border: outline(border_strong(), 8.0),
        ..Default::default()
    }
}

pub(super) fn splitter(active: bool) -> impl Fn(&Theme) -> container::Style {
    surface(if active { brand_blue() } else { bg_deep() })
}

/// Borderless button for toolbars, rails and tree rows.
pub(super) fn ghost(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let disabled = matches!(status, button::Status::Disabled);
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: match (active, hovered) {
                (true, _) => solid(bg_active()),
                (false, true) => solid(bg_hover()),
                (false, false) => None,
            },
            text_color: if disabled {
                faint()
            } else if active || hovered {
                fg()
            } else {
                muted()
            },
            border: Border::default().rounded(5.0),
            ..Default::default()
        }
    }
}

/// A toolbar button: quiet until hovered; the active one only brightens (its marker is a line).
pub(super) fn tool(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let disabled = matches!(status, button::Status::Disabled);
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: (hovered && !disabled).then(|| Background::Color(bg_hover())),
            text_color: if disabled {
                faint()
            } else if active || hovered {
                fg()
            } else {
                muted()
            },
            border: Border::default().rounded(6.0),
            ..Default::default()
        }
    }
}

/// A row of a dropdown menu; disabled rows stay dim and do not react to the pointer.
pub(super) fn menu_item(_: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: hovered.then_some(Background::Color(bg_active())),
        text_color: if matches!(status, button::Status::Disabled) {
            faint()
        } else {
            fg()
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
            fg()
        } else {
            muted()
        },
        ..Default::default()
    }
}

pub(super) fn primary(_: &Theme, status: button::Status) -> button::Style {
    let (from, to) = match status {
        button::Status::Hovered => (brand_cyan(), brand_blue()),
        button::Status::Pressed => (brand_deep(), brand_deep()),
        _ => (brand_blue(), brand_deep()),
    };
    button::Style {
        background: Some(brand_gradient(from, to)),
        text_color: Color::WHITE,
        border: Border::default().rounded(8.0),
        ..Default::default()
    }
}

pub(super) fn secondary(_: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: solid(if hovered { bg_hover() } else { bg_raised() }),
        text_color: fg(),
        border: if hovered {
            outline(alpha(brand_blue(), 0.6), 8.0)
        } else {
            Border::default().rounded(8.0)
        },
        ..Default::default()
    }
}

pub(super) fn danger(_: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: solid(hex_mix(
            super::design::danger(),
            if hovered { 0.30 } else { 0.14 },
        )),
        text_color: super::design::danger(),
        border: Border::default().rounded(8.0),
        ..Default::default()
    }
}

pub(super) fn input(_: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(bg_deep()),
        border: if focused {
            outline(alpha(brand_blue(), 0.7), 8.0)
        } else {
            Border::default().rounded(8.0)
        },
        icon: muted(),
        placeholder: faint(),
        value: fg(),
        selection: alpha(brand_blue(), 0.45),
    }
}

pub(super) fn editor(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(bg_base()),
        border: Border::default(),
        placeholder: faint(),
        value: fg(),
        selection: alpha(brand_blue(), 0.38),
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
        rail.scroller.background = Background::Color(alpha(fg(), strength));
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
    label: impl iced::widget::text::IntoFragment<'a>,
    position: tooltip::Position,
) -> Element<'a, Message> {
    tooltip(content, iced::widget::text(label).size(11), position)
        .gap(6)
        .padding(6)
        .style(tooltip_box)
        .into()
}
