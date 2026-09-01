//! ANSI escape sequence utilities.

use regex::Regex;
use std::sync::LazyLock;

static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])").unwrap());

/// Strip all ANSI escape sequences from a string.
pub fn strip_ansi(s: &str) -> String {
    ANSI_RE.replace_all(s, "").into_owned()
}

/// Common ANSI control sequences.
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const SHOW_CURSOR: &str = "\x1b[?25h";
pub const ALT_SCREEN_ON: &str = "\x1b[?1049h";
pub const ALT_SCREEN_OFF: &str = "\x1b[?1049l";
pub const CLEAR_SCREEN: &str = "\x1b[H\x1b[2J";
pub const HOME: &str = "\x1b[H";
pub const CLEAR_LINE_RIGHT: &str = "\x1b[K";
pub const CLEAR_BELOW: &str = "\x1b[J";

/// Build a 24-bit TrueColor foreground escape from (r, g, b).
pub fn fg_rgb(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        let s = "\x1b[38;2;255;0;0mhello\x1b[0m world";
        assert_eq!(strip_ansi(s), "hello world");
    }

    #[test]
    fn test_fg_rgb() {
        assert_eq!(fg_rgb(100, 200, 50), "\x1b[38;2;100;200;50m");
    }
}
