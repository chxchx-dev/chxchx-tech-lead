//! Inline vector icon set. Each icon is parsed once and tinted at draw time.
use iced::widget::svg::{self, Handle};
use iced::{Color, Theme};
use std::collections::HashMap;
use std::sync::LazyLock;

const ICONS: &[(&str, &str)] = &[
    (
        "home",
        r#"<path d="m3 10 9-7 9 7"/><path d="M5.5 9v11h13V9M9.5 20v-6h5v6"/>"#,
    ),
    (
        "folder",
        r#"<path d="M3 6.5A1.5 1.5 0 0 1 4.5 5h5l2 2.5h8A1.5 1.5 0 0 1 21 9v9.5a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z"/>"#,
    ),
    (
        "folder-search",
        r#"<path d="M3 6.5A1.5 1.5 0 0 1 4.5 5h5l2 2.5h8A1.5 1.5 0 0 1 21 9v9.5a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z"/><circle cx="11.5" cy="13" r="2.6"/><path d="m13.5 15 2.5 2.5"/>"#,
    ),
    ("arrow-down", r#"<path d="M12 4.5v15m-6-6 6 6 6-6"/>"#),
    ("arrow-up", r#"<path d="M12 19.5v-15m-6 6 6-6 6 6"/>"#),
    (
        "git-commit",
        r#"<circle cx="12" cy="12" r="3.5"/><path d="M3 12h5.5m7 0H21"/>"#,
    ),
    (
        "trash",
        r#"<path d="M4 7h16M9 7V4.5h6V7m-8 0 1 13h8l1-13"/>"#,
    ),
    ("plus", r#"<path d="M12 5v14M5 12h14"/>"#),
    ("minus", r#"<path d="M5 12h14"/>"#),
    (
        "archive",
        r#"<rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9v10h14V9M10 13h4"/>"#,
    ),
    (
        "folder-open",
        r#"<path d="M3 7V6.5A1.5 1.5 0 0 1 4.5 5h5l2 2.5h6"/><path d="m3 17 2-7h16l-2 9H4.5A1.5 1.5 0 0 1 3 17Z"/>"#,
    ),
    (
        "file",
        r#"<path d="M6 3.5h8l5 5V20a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4.5a1 1 0 0 1 1-1Z"/><path d="M14 3.5V9h5"/>"#,
    ),
    (
        "file-text",
        r#"<path d="M6 3.5h8l5 5V20a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4.5a1 1 0 0 1 1-1Z"/><path d="M14 3.5V9h5M8.5 13h7M8.5 17h7"/>"#,
    ),
    (
        "code",
        r#"<path d="m8 7-5 5 5 5m8-10 5 5-5 5M13.5 5l-3 14"/>"#,
    ),
    (
        "braces",
        r#"<path d="M8 4c-2 0-3 1-3 3v2c0 1.5-.8 3-2 3 1.2 0 2 1.5 2 3v2c0 2 1 3 3 3m8-16c2 0 3 1 3 3v2c0 1.5.8 3 2 3-1.2 0-2 1.5-2 3v2c0 2-1 3-3 3"/>"#,
    ),
    (
        "save",
        r#"<path d="M5 3.5h12l3.5 3.5v13a1 1 0 0 1-1 1h-14a1 1 0 0 1-1-1v-15a1 1 0 0 1 1-1Z"/><path d="M8 4v5h8V4M8 21v-8h8v8"/>"#,
    ),
    (
        "search",
        r#"<circle cx="10.5" cy="10.5" r="6.5"/><path d="m15.5 15.5 5 5"/>"#,
    ),
    ("undo", r#"<path d="m9 6-5 5 5 5M4 11h9a7 7 0 0 1 7 7"/>"#),
    (
        "redo",
        r#"<path d="m15 6 5 5-5 5M20 11h-9a7 7 0 0 0-7 7"/>"#,
    ),
    (
        "terminal",
        r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="m7 9 3 3-3 3m6 0h4"/>"#,
    ),
    ("activity", r#"<path d="M3 12h4l3-7 4 14 3-7h4"/>"#),
    (
        "bell",
        r#"<path d="M6 16V11a6 6 0 0 1 12 0v5l2 2H4zM10 21h4"/>"#,
    ),
    (
        "setup",
        r#"<path d="M4 6h16M4 12h16M4 18h16"/><circle cx="9" cy="6" r="2"/><circle cx="15" cy="12" r="2"/><circle cx="8" cy="18" r="2"/>"#,
    ),
    (
        "projects",
        r#"<rect x="3" y="3" width="7.5" height="7.5" rx="1.5"/><rect x="13.5" y="3" width="7.5" height="7.5" rx="1.5"/><rect x="3" y="13.5" width="7.5" height="7.5" rx="1.5"/><rect x="13.5" y="13.5" width="7.5" height="7.5" rx="1.5"/>"#,
    ),
    (
        "integrations",
        r#"<path d="M9 7V4m6 3V4M7 7h10v5a5 5 0 0 1-10 0zM12 17v3m-4 0h8"/>"#,
    ),
    (
        "diagnostics",
        r#"<path d="M20.5 12a8.5 8.5 0 1 1-17 0 8.5 8.5 0 0 1 17 0Z"/><path d="M7 12h2.5l1.5-3.5 2.5 7 1.5-3.5H17"/>"#,
    ),
    (
        "refresh",
        r#"<path d="M20 5v5h-5M4 19v-5h5"/><path d="M19.5 10A8 8 0 0 0 5.5 7.5M4.5 14a8 8 0 0 0 14 2.5"/>"#,
    ),
    ("close", r#"<path d="m6.5 6.5 11 11m0-11-11 11"/>"#),
    (
        "shield",
        r#"<path d="m12 3 7 3v5c0 4.5-2.800 7.700-7 10-4.2-2.300-7-5.5-7-10V6z"/><path d="m9 12 2 2 4-4"/>"#,
    ),
    (
        "warning",
        r#"<path d="M12 3.5 21.5 20h-19z"/><path d="M12 10v4.5m0 2.5v.1"/>"#,
    ),
    (
        "info",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5m0-9v.1"/>"#,
    ),
    ("check", r#"<path d="m5 12.5 4.5 4.5L19 7.5"/>"#),
    ("chevron-right", r#"<path d="m9.5 6 6 6-6 6"/>"#),
    ("chevron-down", r#"<path d="m6 9.5 6 6 6-6"/>"#),
    ("arrow-right", r#"<path d="M4.5 12h15m-6-6 6 6-6 6"/>"#),
    (
        "branch",
        r#"<circle cx="7" cy="5.5" r="2.2"/><circle cx="7" cy="18.5" r="2.2"/><circle cx="17" cy="8.5" r="2.2"/><path d="M7 7.700v8.600M17 10.700c0 3-3 3.300-6 3.800-2.5.4-4 1-4 1.800"/>"#,
    ),
    (
        "panel-left",
        r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>"#,
    ),
    (
        "panel-right",
        r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/>"#,
    ),
    (
        "panel-bottom",
        r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 14.5h18"/>"#,
    ),
    (
        "stop",
        r#"<rect x="6.5" y="6.5" width="11" height="11" rx="1.5"/>"#,
    ),
    ("columns", r#"<path d="M4 7h16M4 12h16M4 17h10"/>"#),
    (
        "sparkle",
        r#"<path d="M12 3.5 13.800 10l6.700 2-6.700 2L12 20.5 10.2 14l-6.700-2 6.700-2z"/>"#,
    ),
    (
        "layers",
        r#"<path d="m12 3.5 9 4.5-9 4.5L3 8z"/><path d="m3 12 9 4.5 9-4.5M3 16l9 4.5 9-4.5"/>"#,
    ),
    ("dot", r##"<circle cx="12" cy="12" r="4.5" fill="#fff"/>"##),
];

static HANDLES: LazyLock<HashMap<&'static str, Handle>> = LazyLock::new(|| {
    ICONS
        .iter()
        .map(|(name, drawing)| {
            let data = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{drawing}</svg>"##
            );
            (*name, Handle::from_memory(data.into_bytes()))
        })
        .collect()
});

/// Returns the named icon tinted with `color`. Unknown names draw a dot.
pub(super) fn icon<'a>(name: &str, color: Color, size: f32) -> svg::Svg<'a, Theme> {
    let handle = HANDLES
        .get(name)
        .or_else(|| HANDLES.get("dot"))
        .cloned()
        .expect("the icon set always contains the fallback dot");
    svg::Svg::new(handle)
        .width(size)
        .height(size)
        .style(move |_: &Theme, _| svg::Style { color: Some(color) })
}
