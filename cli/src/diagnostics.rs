//! System diagnostics, crash reporting, and error logging subsystem.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Get the persistent cache directory for ram-tui logs (~/.cache/ram-tui).
pub fn get_log_dir() -> PathBuf {
    if let Ok(cache_home) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(cache_home).join("ram-tui")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache").join("ram-tui")
    } else {
        std::env::temp_dir().join("ram-tui")
    }
}

/// Initialize diagnostic logging if requested via CLI flag or RAM_DEBUG=1.
pub fn init_diagnostics(debug: bool) {
    let enabled = debug
        || std::env::var("RAM_DEBUG")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false);
    DEBUG_ENABLED.store(enabled, Ordering::Relaxed);

    if enabled {
        let log_dir = get_log_dir();
        let _ = fs::create_dir_all(&log_dir);
        let log_file = log_dir.join("debug.log");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_file) {
            let _ = writeln!(
                f,
                "[{}] --- ram-tui diagnostics initialized (PID: {}) ---",
                iso_timestamp(),
                std::process::id()
            );
        }
    }
}

/// Write a formatted message to debug.log if debugging is enabled.
pub fn log_debug(msg: &str) {
    if DEBUG_ENABLED.load(Ordering::Relaxed) {
        let log_dir = get_log_dir();
        let log_file = log_dir.join("debug.log");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_file) {
            let _ = writeln!(f, "[{}] [DEBUG] {msg}", iso_timestamp());
        }
    }
}

/// Install panic hook to restore terminal and generate detailed crash dump.
pub fn install_panic_hook(version: &'static str) {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // 1. Idempotently restore terminal screen, raw mode, and cursor
        let _ = std::io::stdout().write_all(b"\x1b[?1049l\x1b[?25h\x1b[0m\n");
        let _ = std::io::stdout().flush();

        // 2. Extract panic details
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let backtrace = std::backtrace::Backtrace::capture();

        // 3. Inspect system and kernel telemetry accessibility
        let proc_meminfo_readable = std::fs::read_to_string("/proc/meminfo").is_ok();
        let proc_dir_readable = std::fs::read_dir("/proc").is_ok();

        let log_dir = get_log_dir();
        let _ = fs::create_dir_all(&log_dir);
        let crash_file = log_dir.join("crash.log");

        // 4. Write full crash dump to disk
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_file)
        {
            let _ = writeln!(
                f,
                "================================================================================"
            );
            let _ = writeln!(f, "RAM-TUI CRASH REPORT — {}", iso_timestamp());
            let _ = writeln!(f, "Version:     {version}");
            let _ = writeln!(
                f,
                "OS:          {} ({})",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            let _ = writeln!(f, "Location:    {location}");
            let _ = writeln!(f, "Cause:       {payload}");
            let _ = writeln!(f, "Diagnostics: /proc/meminfo readable: {proc_meminfo_readable}, /proc readable: {proc_dir_readable}");
            let _ = writeln!(f, "Backtrace:\n{backtrace:?}");
            let _ = writeln!(f, "================================================================================\n");
        }

        // 5. Emit clean, informative error message to stderr
        eprintln!(
            "\n┌─────────────────────────────────────────────────────────────────────────────┐"
        );
        eprintln!(
            "│                         [RAM-TUI ERROR REPORT]                              │"
        );
        eprintln!(
            "├─────────────────────────────────────────────────────────────────────────────┤"
        );
        eprintln!(
            "│ An unexpected error occurred during execution:                              │"
        );
        eprintln!(
            "│   Cause:    {:<64}│",
            payload.chars().take(64).collect::<String>()
        );
        eprintln!(
            "│   Location: {:<64}│",
            location.chars().take(64).collect::<String>()
        );
        eprintln!("│   Version:  {:<64}│", version);
        eprintln!(
            "│                                                                             │"
        );
        eprintln!(
            "│ Detailed diagnostic crash log saved to:                                     │"
        );
        eprintln!(
            "│   {:<74}│",
            crash_file
                .display()
                .to_string()
                .chars()
                .take(74)
                .collect::<String>()
        );
        eprintln!(
            "│                                                                             │"
        );
        eprintln!(
            "│ Please report this issue at:                                                │"
        );
        eprintln!(
            "│   https://github.com/BlackFeather-git/ram-tui/issues                        │"
        );
        eprintln!(
            "└─────────────────────────────────────────────────────────────────────────────┘\n"
        );

        default_hook(panic_info);
    }));
}

fn iso_timestamp() -> String {
    unsafe {
        let mut t: libc::time_t = 0;
        libc::time(&mut t);
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
}
