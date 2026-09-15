//! Small helpers for working with strings that contain ANSI escape codes:
//! measuring the width actually printed to the terminal, ignoring color
//! codes (`\x1b[...m`) and other CSI sequences.

/// Visible width of `s` once ANSI CSI escape sequences (`\x1b[...<final>`)
/// are ignored. Assumes no wide (CJK) or zero-width characters, which holds
/// for the ASCII/box-drawing text this tool renders.
pub fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Expect a CSI sequence: ESC '[' <params/intermediates> <final byte>.
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_plain_text() {
        assert_eq!(visible_width("Lifetime"), 8);
    }

    #[test]
    fn ignores_color_codes() {
        assert_eq!(visible_width("\x1b[2m├\x1b[0m Wins: 5"), 9);
    }
}
