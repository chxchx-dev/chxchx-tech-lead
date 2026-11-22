//! The pairing code in two boxes, like the verification codes of professional apps.
//!
//! The person types five characters in the first box, the cursor moves on by itself and the dash
//! between the boxes is only drawn, never typed. Pasting a whole code (with the dash, with spaces,
//! in lower case) into either box spreads it over both. Characters no code can contain are dropped,
//! so a typo cannot use up one of the few attempts the node allows. What is sent is the same
//! `XXXXX-XXXXX` as always.

use chxchx_protocol::{PAIRING_ALPHABET, PAIRING_GROUP};

/// What is in the two boxes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Groups {
    pub first: String,
    pub second: String,
}

/// Upper case, only the characters codes are made of.
pub fn clean(text: &str) -> String {
    text.chars()
        .flat_map(char::to_uppercase)
        .filter(|c| PAIRING_ALPHABET.contains(*c))
        .collect()
}

fn take(text: &str) -> String {
    text.chars().take(PAIRING_GROUP).collect()
}

fn skip(text: &str) -> String {
    text.chars().skip(PAIRING_GROUP).collect()
}

/// Something typed or pasted into the first box. The flag says whether the cursor should move to
/// the second box (the first one is full).
pub fn edit_first(typed: &str, second: &str) -> (Groups, bool) {
    let all = clean(typed);
    let groups = if all.chars().count() > PAIRING_GROUP {
        // A pasted code: what does not fit goes on to the second box.
        Groups {
            first: take(&all),
            second: take(&skip(&all)),
        }
    } else {
        Groups {
            first: all.clone(),
            second: second.to_owned(),
        }
    };
    (groups, all.chars().count() >= PAIRING_GROUP)
}

/// Something typed or pasted into the second box.
pub fn edit_second(typed: &str, first: &str) -> Groups {
    let all = clean(typed);
    if all.chars().count() >= PAIRING_GROUP * 2 {
        // The whole code was pasted here.
        return Groups {
            first: take(&all),
            second: take(&skip(&all)),
        };
    }
    Groups {
        first: first.to_owned(),
        second: take(&all),
    }
}

/// The code to send, once both boxes are full.
pub fn join(groups: &Groups) -> Option<String> {
    let full = |text: &str| text.chars().count() == PAIRING_GROUP;
    (full(&groups.first) && full(&groups.second))
        .then(|| format!("{}-{}", groups.first, groups.second))
}

/// A code such as the one a QR carries, put into the boxes.
pub fn split(code: &str) -> Groups {
    edit_first(code, "").0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups(first: &str, second: &str) -> Groups {
        Groups {
            first: first.into(),
            second: second.into(),
        }
    }

    #[test]
    fn typing_the_first_box_moves_on_only_when_it_is_full() {
        assert_eq!(edit_first("abc", ""), (groups("ABC", ""), false));
        assert_eq!(edit_first("abcde", ""), (groups("ABCDE", ""), true));
    }

    #[test]
    fn a_whole_code_pasted_anywhere_is_spread_over_both_boxes() {
        for pasted in ["abcde-fgh23", "ABCDEFGH23", " abcde fgh23 ", "ABCDE–FGH23"] {
            assert_eq!(
                edit_first(pasted, "").0,
                groups("ABCDE", "FGH23"),
                "{pasted:?}"
            );
            assert_eq!(edit_second(pasted, "").first, "ABCDE", "{pasted:?}");
            assert_eq!(edit_second(pasted, "").second, "FGH23", "{pasted:?}");
        }
    }

    #[test]
    fn the_second_box_keeps_the_first_and_stops_at_five() {
        assert_eq!(edit_second("fgh2345", "ABCDE"), groups("ABCDE", "FGH23"));
        assert_eq!(edit_second("fg", "ABCDE"), groups("ABCDE", "FG"));
    }

    #[test]
    fn fixing_the_first_box_does_not_lose_the_second() {
        assert_eq!(edit_first("ABCD", "FGH23").0, groups("ABCD", "FGH23"));
    }

    #[test]
    fn characters_no_code_has_are_dropped() {
        // I, O, 0 and 1 are never in a code: they would only waste an attempt.
        assert_eq!(clean("a0b1c-i oD!"), "ABCD");
        // Accented letters are different characters: they are dropped, not simplified.
        assert_eq!(clean("ñandú"), "AND");
    }

    #[test]
    fn the_code_is_sent_in_the_usual_format_and_only_when_complete() {
        assert_eq!(
            join(&groups("ABCDE", "FGH23")).as_deref(),
            Some("ABCDE-FGH23")
        );
        assert_eq!(join(&groups("ABCDE", "FGH2")), None);
        assert_eq!(join(&groups("", "")), None);
    }

    #[test]
    fn a_qr_code_fills_both_boxes() {
        assert_eq!(split("ABCDE-FGH23"), groups("ABCDE", "FGH23"));
    }
}
