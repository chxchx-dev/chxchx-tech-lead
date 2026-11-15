//! The ASCII-art banner each agent greets you with when its chat opens.
use super::super::agents;
use super::super::*;
use crate::features::agents::Provider;
use iced::widget::{column, responsive};

const CLAUDE: [&str; 6] = [
    " ██████╗██╗      █████╗ ██╗   ██╗██████╗ ███████╗",
    "██╔════╝██║     ██╔══██╗██║   ██║██╔══██╗██╔════╝",
    "██║     ██║     ███████║██║   ██║██║  ██║█████╗  ",
    "██║     ██║     ██╔══██║██║   ██║██║  ██║██╔══╝  ",
    "╚██████╗███████╗██║  ██║╚██████╔╝██████╔╝███████╗",
    " ╚═════╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═════╝ ╚══════╝",
];

const CODEX: [&str; 6] = [
    " ██████╗ ██████╗ ██████╗ ███████╗██╗  ██╗",
    "██╔════╝██╔═══██╗██╔══██╗██╔════╝╚██╗██╔╝",
    "██║     ██║   ██║██║  ██║█████╗   ╚███╔╝ ",
    "██║     ██║   ██║██║  ██║██╔══╝   ██╔██╗ ",
    "╚██████╗╚██████╔╝██████╔╝███████╗██╔╝ ██╗",
    " ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝",
];

const OPENCODE: [&str; 6] = [
    " ██████╗ ██████╗ ███████╗███╗   ██╗ ██████╗ ██████╗ ██████╗ ███████╗",
    "██╔═══██╗██╔══██╗██╔════╝████╗  ██║██╔════╝██╔═══██╗██╔══██╗██╔════╝",
    "██║   ██║██████╔╝█████╗  ██╔██╗ ██║██║     ██║   ██║██║  ██║█████╗  ",
    "██║   ██║██╔═══╝ ██╔══╝  ██║╚██╗██║██║     ██║   ██║██║  ██║██╔══╝  ",
    "╚██████╔╝██║     ███████╗██║ ╚████║╚██████╗╚██████╔╝██████╔╝███████╗",
    " ╚═════╝ ╚═╝     ╚══════╝╚═╝  ╚═══╝ ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝",
];

const GENERIC: [&str; 6] = [
    " █████╗  ██████╗ ███████╗███╗   ██╗████████╗",
    "██╔══██╗██╔════╝ ██╔════╝████╗  ██║╚══██╔══╝",
    "███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║   ",
    "██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║   ",
    "██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║   ",
    "╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝   ",
];

fn art(provider: Provider) -> &'static [&'static str; 6] {
    match provider {
        Provider::Claude => &CLAUDE,
        Provider::Codex => &CODEX,
        Provider::OpenCode => &OPENCODE,
        Provider::Other => &GENERIC,
    }
}

fn mix(from: iced::Color, to: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: 1.0,
    }
}

/// Width in pixels of one glyph of the banner font.
const GLYPH: f32 = 6.6;
const SIZE: f32 = 11.0;

/// The banner, in the agent's color fading toward the brand cyan; a short wordmark when the
/// pane is too narrow for it.
pub(super) fn banner<'a>(provider: Provider, name: String) -> Element<'a, Message> {
    let (tint, _) = agents::identity(provider);
    let lines = art(provider);
    let needed = lines[0].chars().count() as f32 * GLYPH + 48.0;
    responsive(move |size| {
        if size.width < needed {
            return text(name.to_uppercase())
                .size(22)
                .font(design::UI_BOLD)
                .color(tint)
                .into();
        }
        let mut art = column![].spacing(0);
        for (index, line) in lines.iter().enumerate() {
            let color = mix(tint, design::BRAND_CYAN, index as f32 / 7.0);
            art = art.push(
                text(*line)
                    .size(SIZE)
                    .font(design::MONO)
                    .color(color)
                    .wrapping(iced::widget::text::Wrapping::None),
            );
        }
        art.into()
    })
    .height(iced::Length::Shrink)
    .into()
}
