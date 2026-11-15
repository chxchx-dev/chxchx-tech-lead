//! How the CLI looks in a terminal: color, glyphs and small layouts. Piped or redirected output
//! (and `NO_COLOR`) stays exactly the plain text it always was, so scripts keep working.
use std::io::IsTerminal;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Cyan,
    Blue,
    Green,
    Yellow,
    Red,
    Purple,
    Orange,
    Muted,
}

impl Tone {
    /// `(red, green, blue)` for terminals with 24-bit color, the same palette as Studio.
    fn rgb(self) -> (u8, u8, u8) {
        match self {
            Tone::Cyan => (0x2F, 0xD3, 0xF0),
            Tone::Blue => (0x3B, 0x82, 0xF6),
            Tone::Green => (0x3D, 0xD6, 0x8C),
            Tone::Yellow => (0xF5, 0xB8, 0x4B),
            Tone::Red => (0xF2, 0x70, 0x7A),
            Tone::Purple => (0xB0, 0x8C, 0xFF),
            Tone::Orange => (0xF5, 0x9E, 0x6B),
            Tone::Muted => (0x8F, 0xA1, 0xBC),
        }
    }

    /// The nearest of the 16 basic colors, for terminals without 24-bit color.
    fn basic(self) -> &'static str {
        match self {
            Tone::Cyan => "36",
            Tone::Blue => "34",
            Tone::Green => "32",
            Tone::Yellow => "33",
            Tone::Red => "31",
            Tone::Purple => "35",
            Tone::Orange => "33",
            Tone::Muted => "90",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ui {
    rich: bool,
    truecolor: bool,
}

const BANNER: [&str; 6] = [
    " ██████╗██╗  ██╗██╗  ██╗ ██████╗██╗  ██╗██╗  ██╗",
    "██╔════╝██║  ██║╚██╗██╔╝██╔════╝██║  ██║╚██╗██╔╝",
    "██║     ███████║ ╚███╔╝ ██║     ███████║ ╚███╔╝ ",
    "██║     ██╔══██║ ██╔██╗ ██║     ██╔══██║ ██╔██╗ ",
    "╚██████╗██║  ██║██╔╝ ██╗╚██████╗██║  ██║██╔╝ ██╗",
    " ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝",
];

fn decide(is_terminal: bool, var: impl Fn(&str) -> Option<String>) -> Ui {
    let set = |name: &str| var(name).is_some_and(|v| !v.is_empty() && v != "0");
    let forced = set("CLICOLOR_FORCE") || set("FORCE_COLOR");
    let refused = set("NO_COLOR") || var("TERM").as_deref() == Some("dumb");
    let rich = !refused && (forced || is_terminal);
    let truecolor = matches!(var("COLORTERM").as_deref(), Some("truecolor" | "24bit"))
        // Windows Terminal and recent consoles speak 24-bit color without saying so.
        || cfg!(windows);
    Ui { rich, truecolor }
}

impl Ui {
    /// Rich only when stdout is a terminal that wants color.
    pub fn stdout() -> Self {
        let ui = decide(std::io::stdout().is_terminal(), |n| std::env::var(n).ok());
        ui.enabled_on_this_console()
    }

    pub fn stderr() -> Self {
        let ui = decide(std::io::stderr().is_terminal(), |n| std::env::var(n).ok());
        ui.enabled_on_this_console()
    }

    /// Always plain: what a pipe or a script sees.
    #[cfg(test)]
    pub fn plain() -> Self {
        Self {
            rich: false,
            truecolor: false,
        }
    }

    #[cfg(test)]
    pub fn rich() -> Self {
        Self {
            rich: true,
            truecolor: true,
        }
    }

    #[cfg(windows)]
    fn enabled_on_this_console(mut self) -> Self {
        // Older Windows consoles print the escape codes as text unless asked to understand them.
        if self.rich && !windows_vt::enable() {
            self.rich = false;
        }
        self
    }

    #[cfg(not(windows))]
    fn enabled_on_this_console(self) -> Self {
        self
    }

    pub fn is_rich(&self) -> bool {
        self.rich
    }

    fn code(&self, tone: Tone) -> String {
        if self.truecolor {
            let (r, g, b) = tone.rgb();
            format!("38;2;{r};{g};{b}")
        } else {
            tone.basic().to_owned()
        }
    }

    /// `text` in `tone`.
    pub fn paint(&self, tone: Tone, text: &str) -> String {
        if self.rich {
            format!("\x1b[{}m{text}\x1b[0m", self.code(tone))
        } else {
            text.to_owned()
        }
    }

    pub fn bold(&self, text: &str) -> String {
        if self.rich {
            format!("\x1b[1m{text}\x1b[0m")
        } else {
            text.to_owned()
        }
    }

    pub fn strong(&self, tone: Tone, text: &str) -> String {
        if self.rich {
            format!("\x1b[1;{}m{text}\x1b[0m", self.code(tone))
        } else {
            text.to_owned()
        }
    }

    pub fn dim(&self, text: &str) -> String {
        self.paint(Tone::Muted, text)
    }

    /// ` text ` on a raised background; `[text]` when plain.
    pub fn badge(&self, tone: Tone, text: &str) -> String {
        if !self.rich {
            return format!("[{text}]");
        }
        let background = if self.truecolor { "48;2;18;28;46;" } else { "" };
        format!("\x1b[{background}1;{}m {text} \x1b[0m", self.code(tone))
    }

    /// The first line of a command: `◆ ChxChx · title` and the detail under it. When plain it is
    /// the one line the command always printed (`plain`).
    pub fn heading(&self, plain: &str, title: &str, detail: &str) -> String {
        if !self.rich {
            return plain.to_owned();
        }
        let mut text = format!(
            "\n  {} {}  {}",
            self.strong(Tone::Cyan, "◆"),
            self.strong(Tone::Cyan, "ChxChx"),
            self.bold(title)
        );
        if !detail.is_empty() {
            text.push_str(&format!("\n    {}", self.dim(detail)));
        }
        text.push('\n');
        text
    }

    /// A planned change.
    pub fn bullet(&self, text: &str) -> String {
        if self.rich {
            format!("  {} {text}", self.paint(Tone::Cyan, "•"))
        } else {
            format!("• {text}")
        }
    }

    /// A finished change.
    pub fn done(&self, text: &str) -> String {
        if self.rich {
            format!("  {} {text}", self.strong(Tone::Green, "✓"))
        } else {
            format!("✓ {text}")
        }
    }

    pub fn warning(&self, text: &str) -> String {
        if self.rich {
            format!("  {} {text}", self.strong(Tone::Yellow, "▲ Aviso:"))
        } else {
            format!("Aviso: {text}")
        }
    }

    pub fn blocked(&self, reason: &str) -> String {
        if self.rich {
            format!(
                "  {} {reason}",
                self.strong(Tone::Red, "✗ No se puede aplicar:")
            )
        } else {
            format!("No se puede aplicar: {reason}")
        }
    }

    /// What to run next, set apart from the plan.
    pub fn hint(&self, text: &str) -> String {
        if self.rich {
            format!(
                "\n  {} {}",
                self.strong(Tone::Cyan, "▸"),
                self.paint(Tone::Muted, text)
            )
        } else {
            format!("\n{text}")
        }
    }

    /// `label: value` with the label quieter than the value.
    pub fn field(&self, label: &str, value: &str) -> String {
        if self.rich {
            format!("  {} {value}", self.dim(&format!("{label}:")))
        } else {
            format!("{label}: {value}")
        }
    }

    /// A section title inside a command's output.
    pub fn section(&self, plain: &str) -> String {
        if self.rich {
            format!(
                "\n  {}",
                self.strong(Tone::Cyan, plain.trim_end_matches(':'))
            )
        } else {
            plain.to_owned()
        }
    }

    /// The ChxChx banner with its colors fading from cyan to blue, one line per row.
    pub fn banner(&self) -> Vec<String> {
        BANNER
            .iter()
            .enumerate()
            .map(|(i, row)| {
                if !self.rich {
                    return (*row).to_owned();
                }
                let t = i as f32 / 5.0;
                let (a, b) = (Tone::Cyan.rgb(), Tone::Blue.rgb());
                let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
                let color = if self.truecolor {
                    format!(
                        "38;2;{};{};{}",
                        lerp(a.0, b.0),
                        lerp(a.1, b.1),
                        lerp(a.2, b.2)
                    )
                } else if i < 3 {
                    "96".to_owned()
                } else {
                    "34".to_owned()
                };
                format!("  \x1b[1;{color}m{row}\x1b[0m")
            })
            .collect()
    }
}

/// Pads `text` (without color codes) to `width` columns before it is painted.
pub fn pad(text: &str, width: usize) -> String {
    let len = text.chars().count();
    format!("{text}{}", " ".repeat(width.saturating_sub(len)))
}

#[cfg(windows)]
mod windows_vt {
    use std::ffi::c_void;

    unsafe extern "system" {
        fn GetStdHandle(id: u32) -> *mut c_void;
        fn GetConsoleMode(handle: *mut c_void, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: *mut c_void, mode: u32) -> i32;
    }

    const STD_OUTPUT_HANDLE: u32 = -11_i32 as u32;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    /// Turns on escape-code processing for this console; false when it cannot.
    pub fn enable() -> bool {
        // SAFETY: plain console calls on the process's own standard output handle.
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode = 0;
            if handle.is_null() || GetConsoleMode(handle, &mut mode) == 0 {
                return false;
            }
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_owned())
        }
    }

    #[test]
    fn color_needs_a_terminal_and_respects_no_color() {
        assert!(!decide(false, env(&[])).is_rich(), "a pipe stays plain");
        assert!(decide(true, env(&[])).is_rich());
        assert!(!decide(true, env(&[("NO_COLOR", "1")])).is_rich());
        assert!(!decide(true, env(&[("TERM", "dumb")])).is_rich());
        assert!(
            decide(false, env(&[("CLICOLOR_FORCE", "1")])).is_rich(),
            "forced"
        );
        assert!(!decide(false, env(&[("CLICOLOR_FORCE", "0")])).is_rich());
        assert!(
            !decide(false, env(&[("FORCE_COLOR", "1"), ("NO_COLOR", "1")])).is_rich(),
            "NO_COLOR wins"
        );
    }

    #[test]
    fn plain_output_is_exactly_the_text_scripts_have_always_seen() {
        let ui = Ui::plain();
        assert_eq!(ui.heading("Init: /p", "Init", "/p"), "Init: /p");
        assert_eq!(ui.bullet("crear x"), "• crear x");
        assert_eq!(ui.done("listo"), "✓ listo");
        assert_eq!(ui.warning("ojo"), "Aviso: ojo");
        assert_eq!(ui.blocked("roto"), "No se puede aplicar: roto");
        assert_eq!(ui.hint("usa --apply"), "\nusa --apply");
        assert_eq!(ui.field("Backup", "/b"), "Backup: /b");
        assert_eq!(ui.badge(Tone::Green, "sí"), "[sí]");
        assert_eq!(ui.paint(Tone::Red, "x"), "x");
    }

    #[test]
    fn rich_output_adds_color_and_keeps_the_words() {
        let ui = Ui::rich();
        let text = ui.blocked("roto");
        assert!(
            text.contains("\x1b[")
                && text.contains("No se puede aplicar:")
                && text.contains("roto")
        );
        assert!(ui.done("listo").contains('✓'));
        assert!(ui.heading("x", "Título", "/ruta").contains("Título"));
        assert!(ui.banner().iter().all(|l| l.contains("\x1b[")));
    }

    #[test]
    fn basic_terminals_get_the_sixteen_colors() {
        let basic = Ui {
            rich: true,
            truecolor: false,
        };
        assert_eq!(basic.paint(Tone::Green, "x"), "\x1b[32mx\x1b[0m");
        assert!(!basic.paint(Tone::Green, "x").contains("38;2"));
    }

    #[test]
    fn the_banner_is_a_clean_rectangle() {
        let width = BANNER[0].chars().count();
        assert!(BANNER.iter().all(|r| r.chars().count() == width));
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("abcdef", 3), "abcdef");
    }
}
