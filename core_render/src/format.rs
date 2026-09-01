//! Formatting utilities — byte formatting, percentage, text sanitisation, cross-platform timestamps.

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

/// Return human-readable operating system name.
pub fn os_display_name() -> &'static str {
    match std::env::consts::OS {
        "linux" => "Linux",
        "macos" => "macOS",
        "windows" => "Windows",
        "freebsd" => "FreeBSD",
        "openbsd" => "OpenBSD",
        "netbsd" => "NetBSD",
        "dragonfly" => "DragonFly",
        "solaris" => "Solaris",
        "android" => "Android",
        "ios" => "iOS",
        other => other,
    }
}

/// System civil date-time (Year, Month, Day, Hour, Min, Sec) derived in pure Rust from SystemTime.
pub fn system_now_civil() -> (u32, u32, u32, u32, u32, u32) {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut days = (secs / 86400) as i64;
    let day_secs = (secs % 86400) as u32;

    let hour = day_secs / 3600;
    let min = (day_secs % 3600) / 60;
    let sec = day_secs % 60;

    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y } as u32;

    (year, m, d, hour, min, sec)
}

/// Return the current time as formatted ISO-8601 string (e.g. "2026-09-02T00:25:30").
pub fn iso_timestamp() -> String {
    let (y, m, d, hh, mm, ss) = system_now_civil();
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}")
}

/// Return the current time formatted as "YYYY-MM-DD HH:MM:SS" for logging.
pub fn log_timestamp() -> String {
    let (y, m, d, hh, mm, ss) = system_now_civil();
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}")
}

/// Return current time as "HH:MM:SS".
pub fn local_now_hms() -> String {
    let (_, _, _, hh, mm, ss) = system_now_civil();
    format!("{hh:02}:{mm:02}:{ss:02}")
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

    #[test]
    fn test_timestamps_and_os() {
        let ts = iso_timestamp();
        assert!(ts.contains('T'));
        assert_eq!(ts.len(), 19);

        let hms = local_now_hms();
        assert_eq!(hms.len(), 8);
        assert_eq!(hms.chars().filter(|&c| c == ':').count(), 2);

        let os = os_display_name();
        assert!(!os.is_empty());
    }
}
