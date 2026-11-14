//! Keyboard to terminal: the bytes a program running in the terminal expects for each key press.
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};

/// Combinations Studio keeps for itself even while the terminal has the focus: Ctrl+J toggles the
/// panel, and Ctrl+Shift+<letter> is for the app (Ctrl+Shift+C/V are copy/paste, handled apart).
pub(super) fn reserved(key: &Key, modifiers: Modifiers) -> bool {
    if is_paste(key, modifiers) {
        return false;
    }
    let letter = matches!(key.as_ref(), Key::Character(c) if c.eq_ignore_ascii_case("j"));
    (modifiers.control() && letter && !modifiers.shift() && !modifiers.alt())
        || (modifiers.control() && modifiers.shift() && matches!(key.as_ref(), Key::Character(_)))
        || modifiers.logo() && matches!(key.as_ref(), Key::Character(_))
}

/// Ctrl+Shift+V everywhere, and Cmd+V on macOS.
pub(super) fn is_paste(key: &Key, modifiers: Modifiers) -> bool {
    let v = matches!(key.as_ref(), Key::Character(c) if c.eq_ignore_ascii_case("v"));
    v && ((modifiers.control() && modifiers.shift())
        || (modifiers.logo() && !modifiers.control() && !modifiers.shift()))
}

/// `None` for keys that send nothing (a lone modifier, an unknown key).
pub(super) fn encode(
    key: &Key,
    modifiers: Modifiers,
    text: Option<&str>,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    match key.as_ref() {
        Key::Named(named) => named_key(named, modifiers, application_cursor).or_else(|| {
            text.filter(|t| !t.is_empty() && !modifiers.control())
                .map(|t| prefixed(t, modifiers))
        }),
        Key::Character(character) => {
            if modifiers.control() {
                return control_byte(character).map(|byte| prefixed_bytes(vec![byte], modifiers));
            }
            let text = text.filter(|t| !t.is_empty()).unwrap_or(character);
            Some(prefixed(text, modifiers))
        }
        Key::Unidentified => text
            .filter(|t| !t.is_empty())
            .map(|t| prefixed(t, modifiers)),
    }
}

/// Alt sends the character with an Escape in front (what shells and editors read as Meta).
fn prefixed(text: &str, modifiers: Modifiers) -> Vec<u8> {
    prefixed_bytes(text.as_bytes().to_vec(), modifiers)
}

fn prefixed_bytes(mut bytes: Vec<u8>, modifiers: Modifiers) -> Vec<u8> {
    if modifiers.alt() {
        bytes.insert(0, 0x1b);
    }
    bytes
}

/// Ctrl+letter is 1..=26; a few punctuation keys have their own control codes.
fn control_byte(character: &str) -> Option<u8> {
    let byte = *character.as_bytes().first()?;
    if character.len() != 1 {
        return None;
    }
    match byte {
        b'a'..=b'z' => Some(byte - b'a' + 1),
        b'A'..=b'Z' => Some(byte - b'A' + 1),
        b'@' | b' ' | b'2' => Some(0),
        b'[' | b'3' => Some(0x1b),
        b'\\' | b'4' => Some(0x1c),
        b']' | b'5' => Some(0x1d),
        b'^' | b'6' => Some(0x1e),
        b'_' | b'7' | b'/' => Some(0x1f),
        b'?' | b'8' => Some(0x7f),
        _ => None,
    }
}

/// xterm's modifier parameter: 1 + Shift(1) + Alt(2) + Ctrl(4).
fn modifier_parameter(modifiers: Modifiers) -> u8 {
    1 + u8::from(modifiers.shift())
        + 2 * u8::from(modifiers.alt())
        + 4 * u8::from(modifiers.control())
}

fn named_key(named: Named, modifiers: Modifiers, application_cursor: bool) -> Option<Vec<u8>> {
    let modified = modifier_parameter(modifiers) > 1;
    // Cursor keys: `ESC [ A`, `ESC O A` in application mode, `ESC [ 1 ; m A` with modifiers.
    let cursor = |letter: char| -> Vec<u8> {
        if modified {
            format!("\x1b[1;{}{letter}", modifier_parameter(modifiers)).into_bytes()
        } else if application_cursor {
            format!("\x1bO{letter}").into_bytes()
        } else {
            format!("\x1b[{letter}").into_bytes()
        }
    };
    let tilde = |code: u8| -> Vec<u8> {
        if modified {
            format!("\x1b[{code};{}~", modifier_parameter(modifiers)).into_bytes()
        } else {
            format!("\x1b[{code}~").into_bytes()
        }
    };
    let function = |ss3: char, code: u8| -> Vec<u8> {
        if modified {
            format!("\x1b[{code};{}~", modifier_parameter(modifiers)).into_bytes()
        } else if code < 15 {
            format!("\x1bO{ss3}").into_bytes()
        } else {
            format!("\x1b[{code}~").into_bytes()
        }
    };
    Some(match named {
        Named::Enter => prefixed_bytes(vec![b'\r'], modifiers),
        Named::Backspace if modifiers.control() => vec![0x08],
        Named::Backspace => prefixed_bytes(vec![0x7f], modifiers),
        Named::Tab if modifiers.shift() => b"\x1b[Z".to_vec(),
        Named::Tab => vec![b'\t'],
        Named::Escape => vec![0x1b],
        Named::Space if modifiers.control() => vec![0],
        Named::Space => prefixed_bytes(vec![b' '], modifiers),
        Named::ArrowUp => cursor('A'),
        Named::ArrowDown => cursor('B'),
        Named::ArrowRight => cursor('C'),
        Named::ArrowLeft => cursor('D'),
        Named::Home => cursor('H'),
        Named::End => cursor('F'),
        Named::PageUp => tilde(5),
        Named::PageDown => tilde(6),
        Named::Insert => tilde(2),
        Named::Delete => tilde(3),
        Named::F1 => function('P', 11),
        Named::F2 => function('Q', 12),
        Named::F3 => function('R', 13),
        Named::F4 => function('S', 14),
        Named::F5 => function(' ', 15),
        Named::F6 => function(' ', 17),
        Named::F7 => function(' ', 18),
        Named::F8 => function(' ', 19),
        Named::F9 => function(' ', 20),
        Named::F10 => function(' ', 21),
        Named::F11 => function(' ', 23),
        Named::F12 => function(' ', 24),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: Key, modifiers: Modifiers, text: Option<&str>) -> Option<Vec<u8>> {
        encode(&key, modifiers, text, false)
    }

    fn ch(c: &str) -> Key {
        Key::Character(c.into())
    }

    fn none() -> Modifiers {
        Modifiers::empty()
    }

    #[test]
    fn plain_typing_sends_the_text_including_shifted_and_accented_characters() {
        assert_eq!(press(ch("a"), none(), Some("a")), Some(b"a".to_vec()));
        assert_eq!(
            press(ch("a"), Modifiers::SHIFT, Some("A")),
            Some(b"A".to_vec())
        );
        assert_eq!(
            press(ch("n"), none(), Some("ñ")),
            Some("ñ".as_bytes().to_vec())
        );
        assert_eq!(
            press(Key::Named(Named::Space), none(), Some(" ")),
            Some(b" ".to_vec())
        );
    }

    #[test]
    fn control_letters_become_control_characters() {
        assert_eq!(press(ch("c"), Modifiers::CTRL, None), Some(vec![3]));
        assert_eq!(
            press(ch("z"), Modifiers::CTRL, Some("\u{1a}")),
            Some(vec![26])
        );
        assert_eq!(press(ch("["), Modifiers::CTRL, None), Some(vec![0x1b]));
        assert_eq!(press(ch("1"), Modifiers::CTRL, None), None);
    }

    #[test]
    fn alt_puts_an_escape_in_front() {
        assert_eq!(
            press(ch("b"), Modifiers::ALT, Some("b")),
            Some(b"\x1bb".to_vec())
        );
        assert_eq!(
            press(Key::Named(Named::Enter), Modifiers::ALT, None),
            Some(b"\x1b\r".to_vec())
        );
    }

    #[test]
    fn editing_keys_send_what_a_shell_expects() {
        let named = |n| press(Key::Named(n), none(), None);
        assert_eq!(named(Named::Enter), Some(b"\r".to_vec()));
        assert_eq!(named(Named::Backspace), Some(vec![0x7f]));
        assert_eq!(named(Named::Tab), Some(b"\t".to_vec()));
        assert_eq!(
            press(Key::Named(Named::Tab), Modifiers::SHIFT, None),
            Some(b"\x1b[Z".to_vec())
        );
        assert_eq!(named(Named::Escape), Some(vec![0x1b]));
        assert_eq!(named(Named::Delete), Some(b"\x1b[3~".to_vec()));
        assert_eq!(named(Named::PageUp), Some(b"\x1b[5~".to_vec()));
        assert_eq!(named(Named::F1), Some(b"\x1bOP".to_vec()));
        assert_eq!(named(Named::F5), Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn arrows_follow_the_mode_the_program_asked_for_and_carry_modifiers() {
        let up = Key::Named(Named::ArrowUp);
        assert_eq!(encode(&up, none(), None, false), Some(b"\x1b[A".to_vec()));
        assert_eq!(encode(&up, none(), None, true), Some(b"\x1bOA".to_vec()));
        let left = Key::Named(Named::ArrowLeft);
        assert_eq!(
            encode(&left, Modifiers::CTRL, None, true),
            Some(b"\x1b[1;5D".to_vec())
        );
        assert_eq!(
            encode(&left, Modifiers::SHIFT | Modifiers::ALT, None, false),
            Some(b"\x1b[1;4D".to_vec())
        );
    }

    #[test]
    fn a_lone_modifier_sends_nothing() {
        for key in [
            Named::Shift,
            Named::Control,
            Named::Alt,
            Named::Super,
            Named::CapsLock,
        ] {
            assert_eq!(press(Key::Named(key), none(), None), None);
        }
        assert_eq!(press(Key::Unidentified, none(), None), None);
    }

    #[test]
    fn studio_keeps_its_own_shortcuts_but_paste_goes_to_the_terminal() {
        assert!(reserved(&ch("j"), Modifiers::CTRL));
        assert!(reserved(&ch("b"), Modifiers::CTRL | Modifiers::SHIFT));
        assert!(
            !reserved(&ch("c"), Modifiers::CTRL),
            "Ctrl+C interrupts the program"
        );
        assert!(
            !reserved(&ch("r"), Modifiers::CTRL),
            "Ctrl+R is the shell's history search"
        );
        assert!(!reserved(&ch("v"), Modifiers::CTRL | Modifiers::SHIFT));
        assert!(is_paste(&ch("V"), Modifiers::CTRL | Modifiers::SHIFT));
        assert!(is_paste(&ch("v"), Modifiers::LOGO), "Cmd+V pastes on macOS");
        assert!(!reserved(&ch("v"), Modifiers::LOGO));
        assert!(!is_paste(&ch("v"), Modifiers::CTRL));
    }
}
