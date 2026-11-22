//! A QR code for the terminal: what a phone scans to open the client with the pairing code in it.
//!
//! Drawn with half blocks on an explicit white background, so it scans the same on a dark or a
//! light terminal (a code with light modules on a dark background is not read by every scanner).
use qrcode::{Color, EcLevel, QrCode};

/// Quiet zone around the code, in modules. The standard asks for four; two read fine on a phone.
const QUIET: usize = 2;
const WHITE_ON_BLACK: &str = "\x1b[30;47m";
const RESET: &str = "\x1b[0m";

/// The code as lines of text, or `None` when the data does not fit in a QR code.
pub(super) fn render(data: &str) -> Option<String> {
    let code = QrCode::with_error_correction_level(data, EcLevel::M).ok()?;
    let width = code.width();
    Some(half_blocks(&code.to_colors(), width))
}

/// `modules` is row after row, `width` wide. Two rows go in each line of text.
fn half_blocks(modules: &[Color], width: usize) -> String {
    let side = width + QUIET * 2;
    let dark = |row: usize, column: usize| {
        let (row, column) = (
            row as isize - QUIET as isize,
            column as isize - QUIET as isize,
        );
        row >= 0
            && column >= 0
            && (row as usize) < width
            && (column as usize) < width
            && modules[row as usize * width + column as usize] == Color::Dark
    };
    let mut out = String::new();
    for top in (0..side).step_by(2) {
        out.push_str(WHITE_ON_BLACK);
        for column in 0..side {
            out.push(match (dark(top, column), dark(top + 1, column)) {
                (false, false) => ' ',
                (true, false) => '▀',
                (false, true) => '▄',
                (true, true) => '█',
            });
        }
        out.push_str(RESET);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_rows_share_a_line_and_a_white_border_surrounds_the_code() {
        use Color::{Dark as D, Light as L};
        // A 3x3 code with a dark centre column.
        let text = half_blocks(&[L, D, L, L, D, L, L, D, L], 3);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), (3 + QUIET * 2).div_ceil(2));
        for line in &lines {
            assert!(line.starts_with(WHITE_ON_BLACK) && line.ends_with(RESET));
        }
        let plain: Vec<String> = lines
            .iter()
            .map(|line| line.replace(WHITE_ON_BLACK, "").replace(RESET, ""))
            .collect();
        assert!(
            plain[0].chars().all(|c| c == ' '),
            "the top border is white"
        );
        // Rows 2..4 of the 7 (border included) hold the code: the dark column is at index 3.
        assert_eq!(
            plain[1].chars().nth(3),
            Some('█'),
            "rows 2 and 3 are both dark there"
        );
        assert_eq!(plain[1].chars().nth(2), Some(' '));
        assert_eq!(
            plain[2].chars().nth(3),
            Some('▀'),
            "row 4 is dark, row 5 is border"
        );
    }

    #[test]
    fn a_pairing_address_fits_and_is_drawn_square() {
        let text = render("https://mi-pc.tail1234.ts.net/#code=ABCDE-FGH23").unwrap();
        let widths: Vec<usize> = text
            .lines()
            .map(|line| {
                line.replace(WHITE_ON_BLACK, "")
                    .replace(RESET, "")
                    .chars()
                    .count()
            })
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] == pair[1]),
            "every line is as wide"
        );
        // Half blocks: twice as many modules tall as lines.
        assert!(widths[0] >= 25 && (widths[0] as f32 / widths.len() as f32 - 2.0).abs() < 0.6);
        assert!(
            render(&"x".repeat(5000)).is_none(),
            "too much data is not drawn"
        );
    }
}
