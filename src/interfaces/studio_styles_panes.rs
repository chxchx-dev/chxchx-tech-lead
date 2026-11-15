//! Looks of the pane grid: cards that glow when focused and a title bar that says where you are.
use super::super::design::*;
use iced::widget::{container, pane_grid};
use iced::{Background, Border, Color, Theme};

pub(in crate::interfaces::studio) fn pane(focused: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(BG_BASE)),
        border: Border {
            color: if focused {
                alpha(ACCENT, 0.32)
            } else {
                alpha(BORDER, 0.55)
            },
            width: 1.0,
            radius: 12.0.into(),
        },
        // No blurred shadow: the software renderer re-blurs it on every frame of a scroll.
        ..Default::default()
    }
}

pub(in crate::interfaces::studio) fn pane_title(
    focused: bool,
) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(if focused { BG_PANEL } else { BG_BASE })),
        border: Border {
            radius: iced::border::Radius::new(12.0).bottom(0.0),
            ..Border::default()
        },
        ..Default::default()
    }
}

pub(in crate::interfaces::studio) fn grid(_: &Theme) -> pane_grid::Style {
    let line = |color: Color| pane_grid::Line { color, width: 3.0 };
    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: Background::Color(alpha(ACCENT, 0.16)),
            border: Border {
                color: ACCENT,
                width: 2.0,
                radius: 12.0.into(),
            },
        },
        hovered_split: line(alpha(ACCENT, 0.7)),
        picked_split: line(ACCENT),
    }
}
