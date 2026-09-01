//! Formatting utilities — byte formatting, percentage, text sanitisation.

/// Format a byte count in compact IEC units (B, KB, MB, GB, TB).
pub fn format_bytes(value: u64, precision: usize) -> String {
    if value < 1024 {
        return format!("{value} B");
    }
    if value < 1024 * 1024 {
        return format!("{:.prec$} KB", value as f64 / 1024.0, prec = precision);
    }
    if value < 1024 * 1024 * 1024 {
        return format!(
            "{:.prec$} MB",
            value as f64 / (1024.0 * 1024.0),
            prec = precision
        );
    }
    if value < 1024u64 * 1024 * 1024 * 1024 {
        return format!(
            "{:.prec$} GB",
            value as f64 / (1024.0 * 1024.0 * 1024.0),
            prec = precision
        );
    }
    format!(
        "{:.prec$} TB",
        value as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0),
        prec = precision
    )
}

/// Return a bounded percentage; zero when denominator is unknown or zero.
pub fn percentage(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    ((part as f64 / whole as f64) * 100.0).clamp(0.0, 100.0)
}

/// Clamp a value to [low, high].
pub fn clamp_val<T: PartialOrd>(value: T, low: T, high: T) -> T {
    if value < low {
        low
    } else if value > high {
        high
    } else {
        value
    }
}

/// Bidi and zero-width control characters to strip.
const BIDI_CHARS: &[char] = &[
    '\u{200b}', '\u{200c}', '\u{200d}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}',
    '\u{202d}', '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{feff}',
];

/// Strip ANSI sequences, control characters, and bidi overrides from text.
pub fn sanitize_text(value: &str) -> String {
    let text = crate::ansi::strip_ansi(value);
    text.chars()
        .map(|ch| {
            let code = ch as u32;
            if code < 32 || code == 127 || BIDI_CHARS.contains(&ch) {
                '~'
            } else {
                ch
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_boundaries() {
        assert_eq!(format_bytes(0, 2), "0 B");
        assert_eq!(format_bytes(1023, 2), "1023 B");
        assert_eq!(format_bytes(1024, 2), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024, 2), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024, 2), "1.00 GB");
        assert_eq!(format_bytes(2 * 1024u64 * 1024 * 1024 * 1024, 2), "2.00 TB");
    }

    #[test]
    fn test_percentage() {
        assert_eq!(percentage(0, 0), 0.0);
        assert_eq!(percentage(50, 100), 50.0);
        assert_eq!(percentage(200, 100), 100.0);
    }

    #[test]
    fn test_sanitize() {
        let text =
            "hello\n\x1b[31;1mworld\x1b[0m\tok\u{202e}override\u{202d}lro\u{feff}bom\u{200b}zwsp";
        let clean = sanitize_text(text);
        assert!(!clean.contains('\n'));
        assert!(!clean.contains('\x1b'));
        assert!(!clean.contains('\u{202e}'));
        assert!(!clean.contains('\u{202d}'));
        assert!(!clean.contains('\u{feff}'));
        assert!(!clean.contains('\u{200b}'));
        assert_eq!(clean, "hello~world~ok~override~lro~bom~zwsp");
    }
}
