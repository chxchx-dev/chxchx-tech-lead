//! Looks of the chat: the message card and its borderless editor.
use super::super::design::*;
use iced::widget::{container, text_editor};
use iced::{Background, Border, Theme};

/// The rounded card the message box sits in.
pub(in crate::interfaces::studio) fn composer_card(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(bg_raised())),
        border: Border {
            color: alpha(border_strong(), 0.55),
            width: 1.0,
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

/// The editor inside the card: no frame of its own.
pub(in crate::interfaces::studio) fn composer_editor(
    _: &Theme,
    _: text_editor::Status,
) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(iced::Color::TRANSPARENT),
        border: Border::default(),
        placeholder: faint(),
        value: fg(),
        selection: alpha(brand_blue(), 0.4),
    }
}
