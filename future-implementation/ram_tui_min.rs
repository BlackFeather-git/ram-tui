// ram_tui_min.rs
// Minimal single-file Rust prototype of ram-tui (Linux-only, std only).
// Build: rustc -O ram_tui_min.rs -o ram-tui-min

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Simple data structures
#[derive(Clone, Debug)]
struct MemInfo {
    total_kb: u64,
    free_kb: u64,
    available_kb: u64,
    cached_kb: u64,
    buffers_kb: u64,
    swap_total_kb: u64,
    swap_used_kb: u64,
}

#[derive(Clone, Debug)]
struct ProcInfo {
    pid: i32,
    name: String,
    rss_kb: u64,
}

/// Parse /proc/meminfo into MemInfo (best-effort)
fn read_meminfo() -> io::Result<MemInfo> {
    let s = fs::read_to_string("/proc/meminfo")?;
    let mut m = HashMap::new();
    for line in s.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let val = v.trim().split_whitespace().next().unwrap_or("0");
            if let Ok(n) = val.parse::<u64>() {
                m.insert(k.to_string(), n);
            }
        }
    }
    let total_kb = *m.get("MemTotal").unwrap_or(&0);
    let free_kb = *m.get("MemFree").unwrap_or(&0);
    let available_kb = *m.get("MemAvailable").unwrap_or(&free_kb);
    let cached_kb = *m.get("Cached").unwrap_or(&0);
    let buffers_kb = *m.get("Buffers").unwrap_or(&0);
    let swap_total_kb = *m.get("SwapTotal").unwrap_or(&0);
    let swap_free_kb = *m.get("SwapFree").unwrap_or(&swap_total_kb);
    let swap_used_kb = swap_total_kb.saturating_sub(swap_free_kb);
    Ok(MemInfo {
        total_kb,
        free_kb,
        available_kb,
        cached_kb,
        buffers_kb,
        swap_total_kb,
        swap_used_kb,
    })
}

/// Read RSS from /proc/<pid>/statm (pages) and convert to KB
fn read_proc_rss_kb(pid: i32) -> Option<u64> {
    let path = format!("/proc/{}/statm", pid);
    if let Ok(s) = fs::read_to_string(path) {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(pages) = parts[1].parse::<u64>() {
                // Standard x86_64 Linux page size (4096 bytes = 4 KB)
                let page_size_kb = 4u64;
                return Some(pages * page_size_kb);
            }
        }
    }
    None
}

/// Read process list by scanning /proc for numeric dirs and reading comm + statm
fn read_processes(limit: usize) -> io::Result<Vec<ProcInfo>> {
    let mut procs = Vec::new();
    for entry in fs::read_dir("/proc")? {
        if let Ok(entry) = entry {
            if let Ok(fname) = entry.file_name().into_string() {
                if let Ok(pid) = fname.parse::<i32>() {
                    // read comm
                    let comm_path = format!("/proc/{}/comm", pid);
                    let name = fs::read_to_string(&comm_path).unwrap_or_default();
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        continue;
                    }
                    if let Some(rss_kb) = read_proc_rss_kb(pid) {
                        procs.push(ProcInfo { pid, name, rss_kb });
                    }
                }
            }
        }
    }
    // sort by rss desc
    procs.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
    if procs.len() > limit {
        procs.truncate(limit);
    }
    Ok(procs)
}

/// Format bytes/kb to human string
fn kb_to_human(kb: u64) -> String {
    let bytes = kb * 1024;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Simple ANSI helpers
mod ansi {
    pub const CLEAR_LINE: &str = "\x1b[2K";
    pub const HOME: &str = "\x1b[H";
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub fn fg_rgb(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    }
    pub fn bg_rgb(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[48;2;{};{};{}m", r, g, b)
    }
}

/// Minimal themes
#[derive(Clone)]
struct Theme {
    name: &'static str,
    accent: (u8, u8, u8),
    text: (u8, u8, u8),
    bg: (u8, u8, u8),
}

fn themes() -> Vec<Theme> {
    vec![
        Theme { name: "default", accent: (180, 120, 255), text: (220, 220, 220), bg: (10, 10, 12) },
        Theme { name: "solar", accent: (255, 180, 0), text: (230, 230, 230), bg: (12, 12, 20) },
        Theme { name: "mint", accent: (80, 220, 180), text: (230, 230, 230), bg: (8, 10, 8) },
    ]
}

fn get_timestamp_str() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let secs = now % 60;
    let mins = (now / 60) % 60;
    let hours = (now / 3600) % 24;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

/// Render a simple TUI to stdout
fn render_tui(mem: &MemInfo, procs: &[ProcInfo], theme: &Theme, symbol_mode: bool, _width: usize, _height: usize) {
    let mut out = String::new();
    // move home and clear minimal
    out.push_str(ansi::HOME);
    // header
    out.push_str(&format!("{}RAM-TUI{}\n", ansi::BOLD, ansi::RESET));
    out.push_str(&format!("{} - {} - {}\n\n", "shadow", "Linux x86_64", get_timestamp_str()));
    // usage percent
    let used_kb = mem.total_kb.saturating_sub(mem.available_kb);
    let used_pct = if mem.total_kb > 0 { (used_kb as f64 / mem.total_kb as f64) * 100.0 } else { 0.0 };
    out.push_str(&format!("{:.1}%  {} used of {}\n\n", used_pct, kb_to_human(used_kb), kb_to_human(mem.total_kb)));
    // table
    out.push_str("USED        AVAILABLE        TOTAL\n");
    out.push_str(&format!("{:12} {:12} {:12}\n\n", kb_to_human(used_kb), kb_to_human(mem.available_kb), kb_to_human(mem.total_kb)));
    // commit/cached/swap line
    out.push_str("COMMIT      CACHED           SWAP\n");
    out.push_str(&format!("{:12} {:12} {:12}\n\n", format!("{:.1} GB", (used_kb as f64)/(1024.0*1024.0)), kb_to_human(mem.cached_kb), kb_to_human(mem.swap_used_kb)));
    // processes
    out.push_str("PROCESS (RESIDENT SET)\n");
    for p in procs.iter().take(8) {
        let name = if p.name.len() > 20 { format!("{}...", &p.name[..17]) } else { p.name.clone() };
        out.push_str(&format!("{:20} {:>10}\n", format!("{} ({})", name, p.pid), kb_to_human(p.rss_kb)));
    }
    out.push_str("\nUSAGE (bar graph)\n");
    // simple bar graph using accent color and symbol
    let total_display = procs.iter().map(|p| p.rss_kb).sum::<u64>().max(1);
    for p in procs.iter().take(8) {
        let pct = (p.rss_kb as f64 / total_display as f64) * 100.0;
        let bar_len = ((pct / 100.0) * 30.0).round() as usize;
        let sym = if symbol_mode { "█" } else { "#" };
        let bar = sym.repeat(bar_len);
        let color = ansi::fg_rgb(theme.accent.0, theme.accent.1, theme.accent.2);
        out.push_str(&format!("{:20} {:>6.1}% {}{}{}\n", p.name, pct, color, bar, ansi::RESET));
    }
    out.push_str("\n");
    out.push_str("q quit  p pause  t theme  s symbol  +/- rate  h help\n");
    // write to stdout
    print!("{}", out);
    io::stdout().flush().ok();
}

/// Emit JSON snapshot
fn emit_json(mem: &MemInfo, procs: &[ProcInfo]) {
    use std::fmt::Write as FmtWrite;
    let mut s = String::new();
    write!(&mut s, "{{").ok();
    write!(&mut s, "\"total_kb\":{},", mem.total_kb).ok();
    write!(&mut s, "\"available_kb\":{},", mem.available_kb).ok();
    write!(&mut s, "\"used_kb\":{},", mem.total_kb.saturating_sub(mem.available_kb)).ok();
    write!(&mut s, "\"swap_used_kb\":{},", mem.swap_used_kb).ok();
    write!(&mut s, "\"processes\":[").ok();
    for (i, p) in procs.iter().enumerate() {
        write!(&mut s, "{{\"pid\":{},\"name\":\"{}\",\"rss_kb\":{}}}", p.pid, p.name.replace('"', "'"), p.rss_kb).ok();
        if i + 1 != procs.len() { write!(&mut s, ",").ok(); }
    }
    write!(&mut s, "]").ok();
    write!(&mut s, "}}").ok();
    println!("{}", s);
}

/// Enable simple interactive mode using stty (best-effort)
fn enable_stty_raw() {
    let _ = Command::new("stty").arg("-echo").arg("cbreak").status();
}

/// Restore terminal
fn restore_stty() {
    let _ = Command::new("stty").arg("sane").status();
}

/// Read single bytes from stdin in a background thread and push to queue
fn spawn_input_reader(queue: Arc<Mutex<Vec<u8>>>) {
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buf = [0u8; 1];
        loop {
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let mut q = queue.lock().unwrap();
                    q.push(buf[0]);
                }
                Err(_) => break,
            }
        }
    });
}

/// Parse args
struct Config {
    once: bool,
    json: bool,
    rate_ms: u64,
}

fn parse_args() -> Config {
    let mut once = false;
    let mut json = false;
    let mut rate_ms = 500u64;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--once" => { once = true; }
            "--json" => { json = true; }
            "--rate" => {
                if i + 1 < args.len() {
                    if let Ok(r) = args[i+1].parse::<u64>() {
                        rate_ms = r;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Config { once, json, rate_ms }
}

fn main() {
    // Basic panic hook to restore terminal
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_stty();
        default_hook(info);
    }));

    let cfg = parse_args();

    // If --once and --json, just print JSON and exit
    if cfg.once && cfg.json {
        if let Ok(mem) = read_meminfo() {
            if let Ok(procs) = read_processes(20) {
                emit_json(&mem, &procs);
            }
        }
        return;
    }

    // If --once only, print human snapshot and exit
    if cfg.once {
        if let Ok(mem) = read_meminfo() {
            if let Ok(procs) = read_processes(20) {
                // simple one-shot render to stdout
                render_tui(&mem, &procs, &themes()[0], true, 80, 24);
            }
        }
        return;
    }

    // Interactive mode
    enable_stty_raw();
    // ensure restore on exit
    let _guard = scopeguard::guard((), || {
        restore_stty();
    });

    // input queue
    let queue = Arc::new(Mutex::new(Vec::new()));
    spawn_input_reader(queue.clone());

    // state
    let mut paused = false;
    let mut theme_idx = 0usize;
    let mut symbol_mode = true;
    let mut rate_ms = cfg.rate_ms;
    let theme_list = themes();
    let mut last_render = Instant::now() - Duration::from_millis(rate_ms);
    let width = 80usize;
    let height = 24usize;

    // main loop
    loop {
        // handle input
        {
            let mut q = queue.lock().unwrap();
            while let Some(b) = q.pop() {
                match b {
                    b'q' => {
                        restore_stty();
                        println!("\nExiting.");
                        return;
                    }
                    b'p' => paused = !paused,
                    b't' => theme_idx = (theme_idx + 1) % theme_list.len(),
                    b's' => symbol_mode = !symbol_mode,
                    b'+' => { if rate_ms > 50 { rate_ms = rate_ms.saturating_sub(50); } },
                    b'-' => { rate_ms = rate_ms.saturating_add(50); },
                    b'h' => {
                        // show help briefly
                        println!("\nHelp: q quit, p pause, t theme, s symbol, +/- rate, h help\n");
                    }
                    _ => {}
                }
            }
        }

        if paused {
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        if last_render.elapsed() < Duration::from_millis(rate_ms) {
            // small sleep to avoid busy loop
            thread::sleep(Duration::from_millis(5));
            continue;
        }

        last_render = Instant::now();

        // read mem and procs
        let mem = match read_meminfo() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let procs = match read_processes(20) {
            Ok(p) => p,
            Err(_) => Vec::new(),
        };

        // render
        render_tui(&mem, &procs, &theme_list[theme_idx], symbol_mode, width, height);
    }
}

// Minimal scopeguard implementation to avoid external crate
mod scopeguard {
    pub struct Guard<T: FnOnce()> {
        f: Option<T>,
    }
    pub fn guard<T: FnOnce()>(_: (), f: T) -> Guard<T> {
        Guard { f: Some(f) }
    }
    impl<T: FnOnce()> Drop for Guard<T> {
        fn drop(&mut self) {
            if let Some(f) = self.f.take() {
                f();
            }
        }
    }
}
