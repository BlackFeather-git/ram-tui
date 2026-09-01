//! Terminal cell-width arithmetic — mirrors Python `visible_cell_width`, etc.

use unicode_width::UnicodeWidthChar;

use crate::ansi::strip_ansi;

/// Returns the display-cell width of a single character.
pub fn char_cell_width(ch: char) -> usize {
    let code = ch as u32;
    // Fast path: standard printable ASCII
    if (32..=126).contains(&code) {
        return 1;
    }
    if code < 32 || code == 127 {
        return 0;
    }
    // Zero-width joiners, BOM, combining marks
    if ch == '\u{200d}' || ch == '\u{200b}' || ch == '\u{feff}' {
        return 0;
    }
    ch.width().unwrap_or(0)
}

/// Exact printable terminal column width of a string (ANSI stripped).
pub fn visible_cell_width(s: &str) -> usize {
    if s.is_empty() {
        return 0;
    }
    let plain = strip_ansi(s);
    plain.chars().map(char_cell_width).sum()
}

/// Truncate plain text to fit within `max_cells` terminal columns.
pub fn truncate_plain_cells(text: &str, max_cells: usize, ellipsis: &str) -> String {
    if max_cells == 0 {
        return String::new();
    }
    let total_w = visible_cell_width(text);
    if total_w <= max_cells {
        return text.to_string();
    }
    let el_width = visible_cell_width(ellipsis);
    let target_w = max_cells.saturating_sub(el_width);
    let mut cur_width = 0usize;
    let mut result = String::new();
    for c in text.chars() {
        let w = char_cell_width(c);
        if cur_width + w > target_w {
            break;
        }
        result.push(c);
        cur_width += w;
    }
    result.push_str(ellipsis);
    result
}

/// Pad a string with spaces to reach `target_cells` terminal column width.
pub fn pad_plain_cells(text: &str, target_cells: usize, align_left: bool) -> String {
    if target_cells == 0 {
        return String::new();
    }
    let cur_w = visible_cell_width(text);
    if cur_w >= target_cells {
        return text.to_string();
    }
    let pad: String = " ".repeat(target_cells - cur_w);
    if align_left {
        format!("{text}{pad}")
    } else {
        format!("{pad}{text}")
    }
}

/// Clamp a line (including ANSI codes) so it never exceeds `max_cols` display cells.
pub fn clamp_line_to_cols(line: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let w = visible_cell_width(line);
    if w <= max_cols {
        return line.to_string();
    }
    let plain = strip_ansi(line);
    truncate_plain_cells(&plain, max_cols, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_width() {
        assert_eq!(visible_cell_width("hello"), 5);
    }

    #[test]
    fn test_cjk_width() {
        // Each CJK character is 2 cells wide
        assert_eq!(visible_cell_width("你好"), 4);
    }

    #[test]
    fn test_truncate() {
        let t = truncate_plain_cells("hello world", 7, "~");
        assert!(visible_cell_width(&t) <= 7);
        assert!(t.ends_with('~'));
    }

    #[test]
    fn test_pad_left() {
        let p = pad_plain_cells("hi", 6, true);
        assert_eq!(p, "hi    ");
        assert_eq!(visible_cell_width(&p), 6);
    }

    #[test]
    fn test_pad_right() {
        let p = pad_plain_cells("hi", 6, false);
        assert_eq!(p, "    hi");
    }

    #[test]
    fn test_clamp() {
        let line = "a]b]c]d]e]f]g]h]i]j]k]l]m]n]o]p";
        let clamped = clamp_line_to_cols(line, 5);
        assert!(visible_cell_width(&clamped) <= 5);
    }

    #[test]
    fn test_zero_width_chars() {
        assert_eq!(char_cell_width('\u{200d}'), 0);
        assert_eq!(char_cell_width('\u{200b}'), 0);
        assert_eq!(char_cell_width('\u{feff}'), 0);
    }
}
