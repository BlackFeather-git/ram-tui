//! Formatting utilities — byte formatting, percentage, text sanitisation, cross-platform timestamps.

#![allow(non_snake_case, non_camel_case_types, dead_code, unused_imports)]

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

/// Return the sanitized system hostname.
pub fn get_hostname() -> String {
    if let Ok(h) = std::env::var("HOSTNAME") {
        return sanitize_text(&h);
    }
    if let Ok(h) = std::env::var("COMPUTERNAME") {
        return sanitize_text(&h);
    }
    #[cfg(unix)]
    {
        let mut buf = [0u8; 256];
        let ret = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
        if ret == 0 {
            let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            let raw = String::from_utf8_lossy(&buf[..len]);
            return sanitize_text(&raw);
        }
    }
    "unknown".into()
}

/// Convert Unix epoch seconds into civil date-time (Year, Month, Day, Hour, Min, Sec) in pure Rust.
/// Uses Howard Hinnant's integer algorithm (Euclidean affine calendar).
pub fn epoch_to_civil(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
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

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct SYSTEMTIME {
    wYear: u16,
    wMonth: u16,
    wDayOfWeek: u16,
    wDay: u16,
    wHour: u16,
    wMinute: u16,
    wSecond: u16,
    wMilliseconds: u16,
}

#[cfg(windows)]
extern "system" {
    fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
}

/// Obtain current local civil date-time (Year, Month, Day, Hour, Min, Sec).
/// Uses native OS timezone on Unix and Windows with deterministic pure-Rust fallback.
pub fn local_now_civil() -> (u32, u32, u32, u32, u32, u32) {
    #[cfg(unix)]
    {
        let mut t: libc::time_t = 0;
        unsafe {
            libc::time(&mut t);
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&t, &mut tm);
            (
                (tm.tm_year + 1900) as u32,
                (tm.tm_mon + 1) as u32,
                tm.tm_mday as u32,
                tm.tm_hour as u32,
                tm.tm_min as u32,
                tm.tm_sec as u32,
            )
        }
    }

    #[cfg(windows)]
    {
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe {
            GetLocalTime(&mut st);
        }
        (
            st.wYear as u32,
            st.wMonth as u32,
            st.wDay as u32,
            st.wHour as u32,
            st.wMinute as u32,
            st.wSecond as u32,
        )
    }

    #[cfg(not(any(unix, windows)))]
    {
        let now = std::time::SystemTime::now();
        let secs = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        epoch_to_civil(secs)
    }
}

/// Return the current time as formatted ISO-8601 string in local time (e.g. "2026-09-02T00:25:30").
pub fn iso_timestamp() -> String {
    let (y, m, d, hh, mm, ss) = local_now_civil();
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}")
}

/// Return the current time formatted as "YYYY-MM-DD HH:MM:SS" for logging.
pub fn log_timestamp() -> String {
    let (y, m, d, hh, mm, ss) = local_now_civil();
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}")
}

/// Return current local time as "HH:MM:SS".
pub fn local_now_hms() -> String {
    let (_, _, _, hh, mm, ss) = local_now_civil();
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

    #[test]
    fn test_epoch_to_civil_known_dates() {
        // Unix epoch: 1970-01-01 00:00:00 UTC
        assert_eq!(epoch_to_civil(0), (1970, 1, 1, 0, 0, 0));

        // 2000-01-01 00:00:00 UTC = 946684800
        assert_eq!(epoch_to_civil(946684800), (2000, 1, 1, 0, 0, 0));

        // Leap day 2000-02-29 12:34:56 UTC = 951827696
        assert_eq!(epoch_to_civil(951827696), (2000, 2, 29, 12, 34, 56));

        // Leap day 2024-02-29 00:00:00 UTC = 1709164800
        assert_eq!(epoch_to_civil(1709164800), (2024, 2, 29, 0, 0, 0));

        // Year boundary 2026-12-31 23:59:59 UTC = 1798761599
        assert_eq!(epoch_to_civil(1798761599), (2026, 12, 31, 23, 59, 59));

        // Next day 2027-01-01 00:00:00 UTC = 1798761600
        assert_eq!(epoch_to_civil(1798761600), (2027, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_get_hostname_sanitized() {
        let host = get_hostname();
        assert!(!host.is_empty());
        // Verify no ANSI escapes or raw control bytes exist
        assert_eq!(crate::ansi::strip_ansi(&host), host);
        for ch in host.chars() {
            let code = ch as u32;
            assert!(
                code >= 32 && code != 127,
                "hostname contains raw control char: {code}"
            );
        }
    }
}
