use std::collections::HashSet;
use std::io::{self, Write};
use std::process;

use clap::Parser;
use serde::Serialize;

use collector::{
    collect_meminfo, collect_processes_sorted, CgroupInfo, MemInfo, ProcessInfo, SortMetric,
};
use core_render::format::sanitize_text;
use core_render::framebuf::FrameBuffer;
use core_render::meter::GraphSymbol;
use ui::layout::render_snapshot;
use ui::terminal::{should_use_color, terminal_size, Key, TerminalManager};
use ui::themes::{get_palette, next_cycling_mode, next_theme, THEME_NAMES};

pub mod diagnostics;

const VERSION: &str = "1.0.0-rc.5";

/// ram-tui v1.0.0-rc.5 — Fast, aesthetic, native terminal memory monitor
#[derive(Parser, Debug)]
#[command(name = "ram", version = VERSION, about)]
struct Args {
    /// Refresh interval in milliseconds (20–2000, default: 50)
    #[arg(short = 'r', long, default_value_t = 50, value_parser = clap::value_parser!(u64).range(20..=2000))]
    rate: u64,

    /// Number of top processes (1–10000, default: 8)
    #[arg(short = 'n', long = "count", default_value_t = 8)]
    count: usize,

    /// Output one snapshot and exit
    #[arg(short = '1', long)]
    once: bool,

    /// Output one JSON snapshot and exit
    #[arg(long)]
    json: bool,

    /// Show individual process PIDs instead of grouping
    #[arg(long = "no-group")]
    no_group: bool,

    /// Compact mode: memory meters only, no process list
    #[arg(long, group = "display_mode")]
    compact: bool,

    /// Mini mode: single usage bar + percentage only
    #[arg(long, group = "display_mode")]
    mini: bool,

    /// Tiny mode: single line output for status bars
    #[arg(long, group = "display_mode")]
    tiny: bool,

    /// Color theme
    #[arg(long, default_value = "default", value_parser = theme_parser)]
    theme: String,

    /// Meter graph style: 'block' or 'braille'
    #[arg(long, default_value = "block", value_parser = ["block", "braille"])]
    symbol: String,

    /// Process sorting metric: 'rss', 'pss' (Linux), 'uss' (Linux/Windows), or 'name' (default: rss)
    #[arg(long, default_value = "rss", value_parser = ["rss", "pss", "uss", "name"])]
    sort: String,

    /// Enable 60-second rolling memory trend sparkline (default: off, toggle live with 'g')
    #[arg(long = "spark")]
    spark: bool,

    /// Enable verbose diagnostic error logging to ~/.cache/ram-tui/debug.log
    #[arg(long = "debug")]
    debug: bool,

    /// Initial process search filter string
    #[arg(long)]
    filter: Option<String>,
}

fn theme_parser(s: &str) -> Result<String, String> {
    let lower = s.to_lowercase();
    if THEME_NAMES.contains(&lower.as_str()) {
        Ok(lower)
    } else {
        Err(format!(
            "unknown theme '{s}'. Available themes: {}",
            THEME_NAMES.join(", ")
        ))
    }
}

// ---------------------------------------------------------------------------
// JSON serialization structs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct JsonSnapshot {
    timestamp: String,
    hostname: String,
    os: String,
    version: &'static str,
    memory: JsonMemory,
    top_processes: Vec<JsonProcess>,
}

#[derive(Serialize)]
struct JsonMemory {
    total: u64,
    available: u64,
    used: u64,
    commit_as: u64,
    commit_limit: u64,
    cached: u64,
    swap_used: u64,
    swap_total: u64,
    swap_desc: String,
    cgroup: Option<CgroupInfo>,
    valid: bool,
}

impl From<&MemInfo> for JsonMemory {
    fn from(m: &MemInfo) -> Self {
        Self {
            total: m.total,
            available: m.available,
            used: m.used,
            commit_as: m.commit_as,
            commit_limit: m.commit_limit,
            cached: m.cached,
            swap_used: m.swap_used,
            swap_total: m.swap_total,
            swap_desc: m.swap_desc.clone(),
            cgroup: m.cgroup.clone(),
            valid: m.valid,
        }
    }
}

#[derive(Serialize)]
struct JsonProcess {
    name: String,
    rss: u64,
    pss: Option<u64>,
    uss: Option<u64>,
    count: u32,
    pid: Option<u32>,
    children: Vec<JsonChild>,
}

#[derive(Serialize)]
struct JsonChild {
    pid: u32,
    name: String,
    rss: u64,
    pss: Option<u64>,
    uss: Option<u64>,
}

impl From<&ProcessInfo> for JsonProcess {
    fn from(p: &ProcessInfo) -> Self {
        Self {
            name: p.name.clone(),
            rss: p.rss,
            pss: p.pss,
            uss: p.uss,
            count: p.count,
            pid: p.pid,
            children: p
                .children
                .iter()
                .map(|c| JsonChild {
                    pid: c.pid,
                    name: c.name.clone(),
                    rss: c.rss,
                    pss: c.pss,
                    uss: c.uss,
                })
                .collect(),
        }
    }
}

fn iso_timestamp() -> String {
    unsafe {
        let mut t: libc::time_t = 0;
        libc::time(&mut t);
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
}

fn get_hostname() -> String {
    let mut buf = [0u8; 256];
    let ret = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if ret == 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        let raw = String::from_utf8_lossy(&buf[..len]);
        sanitize_text(&raw)
    } else {
        "unknown".into()
    }
}

struct KillTarget {
    pid: u32,
    name: String,
    #[cfg(target_os = "linux")]
    pidfd: Option<i32>,
    #[cfg(target_os = "linux")]
    starttime: Option<String>,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

pub fn run() {
    diagnostics::install_panic_hook(VERSION);

    let args = Args::parse();
    diagnostics::init_diagnostics(args.debug);
    diagnostics::log_debug("Starting ram-tui execution");

    let mut current_mode = if args.compact {
        "compact"
    } else if args.mini {
        "mini"
    } else if args.tiny {
        "tiny"
    } else {
        "hero"
    }
    .to_string();

    let mut current_theme = args.theme.clone();
    let mut current_symbol = if args.symbol == "braille" {
        GraphSymbol::Braille
    } else {
        GraphSymbol::Block
    };

    let mut sort_metric = match args.sort.as_str() {
        "pss" => {
            #[cfg(target_os = "linux")]
            {
                SortMetric::Pss
            }
            #[cfg(target_os = "windows")]
            {
                diagnostics::log_debug(
                    "PSS is Linux smaps specific; falling back to USS on Windows",
                );
                SortMetric::Uss
            }
            #[cfg(target_os = "macos")]
            {
                diagnostics::log_debug("PSS is Linux smaps specific; falling back to RSS on macOS");
                SortMetric::Rss
            }
            #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
            {
                SortMetric::Rss
            }
        }
        "uss" => {
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                SortMetric::Uss
            }
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            {
                diagnostics::log_debug("USS is unavailable on macOS Mach VM; falling back to RSS");
                SortMetric::Rss
            }
        }
        "name" => SortMetric::Name,
        _ => SortMetric::Rss,
    };

    // JSON snapshot mode
    if args.json {
        let mem = collect_meminfo();
        let procs = collect_processes_sorted(!args.no_group, args.count, sort_metric);
        let snapshot = JsonSnapshot {
            timestamp: iso_timestamp(),
            hostname: get_hostname(),
            os: std::env::consts::OS.to_string(),
            version: VERSION,
            memory: JsonMemory::from(&mem),
            top_processes: procs.iter().map(JsonProcess::from).collect(),
        };
        match serde_json::to_string_pretty(&snapshot) {
            Ok(json) => {
                let _ = writeln!(io::stdout(), "{json}");
                let _ = io::stdout().flush();
            }
            Err(_) => process::exit(1),
        }
        return;
    }

    // One-shot mode
    let mut is_once = args.once;
    let mut term = TerminalManager::new();
    if !term.is_tty && !is_once {
        is_once = true;
    }

    if is_once {
        let mem = collect_meminfo();
        let procs = if current_mode == "hero" {
            collect_processes_sorted(!args.no_group, args.count, sort_metric)
        } else {
            Vec::new()
        };
        let color_enabled = should_use_color(false);
        let pal = get_palette(&current_theme, color_enabled);
        let (cols, rows) = terminal_size();
        let empty_set = HashSet::new();
        let initial_ratio = if mem.total > 0 {
            mem.used as f64 / mem.total as f64
        } else {
            0.0
        };
        let history = vec![initial_ratio; 60];
        let output = render_snapshot(
            &mem,
            &procs,
            !args.no_group,
            false,
            &current_mode,
            &current_theme,
            &pal,
            current_symbol,
            color_enabled,
            false,
            None,
            false,
            cols,
            rows,
            sort_metric,
            &history,
            args.spark,
            None,
            &empty_set,
            args.filter.as_deref(),
            false,
            None,
            None,
        );
        let _ = writeln!(io::stdout(), "{output}");
        let _ = io::stdout().flush();
        return;
    }

    // Interactive TUI mode
    let mut refresh_ms = args.rate;
    let proc_count = args.count;
    let mut group_procs = !args.no_group;
    let mut paused = false;
    let mut show_help = false;
    let color_enabled = should_use_color(true);
    let mut fb = FrameBuffer::new();

    let mut cached_mem = collect_meminfo();
    let mut cached_procs = if current_mode == "hero" {
        collect_processes_sorted(group_procs, proc_count, sort_metric)
    } else {
        Vec::new()
    };

    let initial_ratio = if cached_mem.total > 0 {
        cached_mem.used as f64 / cached_mem.total as f64
    } else {
        0.0
    };
    let mut history: Vec<f64> = vec![initial_ratio; 60];
    let mut last_spark_sample = std::time::Instant::now();
    let mut show_sparkline = args.spark;
    let mut selected_idx: usize = 0;
    let mut expanded_groups: HashSet<String> = HashSet::new();
    let mut search_active = false;
    let mut search_query: Option<String> = args.filter.clone();
    let mut kill_prompt: Option<KillTarget> = None;
    let mut theme_modal_open = false;
    let mut theme_modal_idx: usize = THEME_NAMES
        .iter()
        .position(|&t| t == current_theme)
        .unwrap_or(0);

    term.setup_raw();

    loop {
        if !paused {
            cached_mem = collect_meminfo();
            cached_procs = if current_mode == "hero" {
                collect_processes_sorted(group_procs, proc_count, sort_metric)
            } else {
                Vec::new()
            };

            let ratio = if cached_mem.total > 0 {
                cached_mem.used as f64 / cached_mem.total as f64
            } else {
                0.0
            };
            if last_spark_sample.elapsed() >= std::time::Duration::from_millis(1000) {
                history.push(ratio);
                if history.len() > 60 {
                    history.remove(0);
                }
                last_spark_sample = std::time::Instant::now();
            } else if let Some(last) = history.last_mut() {
                *last = ratio;
            }

            let pal = get_palette(&current_theme, color_enabled);
            let (cols, rows) = terminal_size();
            let kill_arg = kill_prompt.as_ref().map(|t| (t.pid, t.name.as_str()));
            let t_modal = if theme_modal_open {
                Some(theme_modal_idx)
            } else {
                None
            };
            let output = render_snapshot(
                &cached_mem,
                &cached_procs,
                group_procs,
                paused,
                &current_mode,
                &current_theme,
                &pal,
                current_symbol,
                color_enabled,
                show_help,
                None,
                true,
                cols,
                rows,
                sort_metric,
                &history,
                show_sparkline,
                Some(selected_idx),
                &expanded_groups,
                search_query.as_deref(),
                search_active,
                kill_arg,
                t_modal,
            );
            let lines: Vec<String> = output.lines().map(|l| l.to_string()).collect();
            if fb.render(&lines, &mut io::stdout()).is_err() {
                break;
            }
        }

        let events = term.get_events(refresh_ms);
        let mut re_render = false;

        for event in events {
            if let Some(target) = kill_prompt.take() {
                if let Key::Char('y' | 'Y') = event {
                    #[cfg(target_os = "linux")]
                    {
                        let mut killed = false;
                        if let Some(fd) = target.pidfd {
                            if collector::pidfd_send_sigterm(fd) {
                                killed = true;
                                diagnostics::log_debug(&format!(
                                    "Sent SIGTERM via pidfd to process {} (PID: {})",
                                    target.name, target.pid
                                ));
                            }
                            unsafe {
                                libc::close(fd);
                            }
                        }
                        if !killed {
                            if collector::validate_process_identity(
                                std::path::Path::new("/proc"),
                                target.pid,
                                &target.name,
                                target.starttime.as_deref(),
                            ) {
                                unsafe {
                                    libc::kill(target.pid as libc::pid_t, libc::SIGTERM);
                                }
                                diagnostics::log_debug(&format!(
                                    "Sent SIGTERM to process {} (PID: {})",
                                    target.name, target.pid
                                ));
                            } else {
                                diagnostics::log_debug(&format!(
                                    "Process {} (PID: {}) identity mismatch — kill cancelled",
                                    target.name, target.pid
                                ));
                            }
                        }
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        unsafe {
                            libc::kill(target.pid as libc::pid_t, libc::SIGTERM);
                        }
                    }
                }
                re_render = true;
                continue;
            }

            if theme_modal_open {
                match event {
                    Key::Esc | Key::Char('q' | 'Q') => {
                        theme_modal_open = false;
                        re_render = true;
                    }
                    Key::Up | Key::Char('k') => {
                        theme_modal_idx = theme_modal_idx.saturating_sub(1);
                        re_render = true;
                    }
                    Key::Down | Key::Char('j') => {
                        theme_modal_idx =
                            (theme_modal_idx + 1).min(THEME_NAMES.len().saturating_sub(1));
                        re_render = true;
                    }
                    Key::Enter => {
                        current_theme = THEME_NAMES[theme_modal_idx].to_string();
                        theme_modal_open = false;
                        re_render = true;
                    }
                    _ => {}
                }
                continue;
            }

            if search_active {
                match event {
                    Key::Esc => {
                        search_active = false;
                        search_query = None;
                        selected_idx = 0;
                        re_render = true;
                    }
                    Key::Enter => {
                        search_active = false;
                        re_render = true;
                    }
                    Key::Backspace => {
                        if let Some(ref mut q) = search_query {
                            q.pop();
                        }
                        selected_idx = 0;
                        re_render = true;
                    }
                    Key::Char(c) if (' '..='~').contains(&c) => {
                        search_query.get_or_insert_with(String::new).push(c);
                        selected_idx = 0;
                        re_render = true;
                    }
                    _ => {}
                }
                continue;
            }

            match event {
                Key::Char('q' | 'Q' | '\x03') => {
                    term.restore();
                    process::exit(0);
                }
                Key::Esc => {
                    if search_query.is_some() {
                        search_query = None;
                        selected_idx = 0;
                        re_render = true;
                    }
                }
                Key::Char('p' | 'P' | ' ') => {
                    paused = !paused;
                    re_render = true;
                }
                Key::Char('g' | 'G') => {
                    show_sparkline = !show_sparkline;
                    re_render = true;
                }
                Key::Char('/') => {
                    search_active = true;
                    if search_query.is_none() {
                        search_query = Some(String::new());
                    }
                    selected_idx = 0;
                    re_render = true;
                }
                Key::Up | Key::Char('k') => {
                    selected_idx = selected_idx.saturating_sub(1);
                    re_render = true;
                }
                Key::Down | Key::Char('j') => {
                    let num_items = if let Some(ref q) = search_query {
                        if !q.is_empty() {
                            let q_lower = q.to_lowercase();
                            cached_procs
                                .iter()
                                .filter(|p| p.name.to_lowercase().contains(&q_lower))
                                .count()
                        } else {
                            cached_procs.len()
                        }
                    } else {
                        cached_procs.len()
                    };
                    if num_items > 0 {
                        selected_idx = (selected_idx + 1).min(num_items.saturating_sub(1));
                    }
                    re_render = true;
                }
                Key::Enter | Key::Tab | Key::Char('e' | 'E') => {
                    let filtered_procs: Vec<&ProcessInfo> = if let Some(ref q) = search_query {
                        if !q.is_empty() {
                            let q_lower = q.to_lowercase();
                            cached_procs
                                .iter()
                                .filter(|p| p.name.to_lowercase().contains(&q_lower))
                                .collect()
                        } else {
                            cached_procs.iter().collect()
                        }
                    } else {
                        cached_procs.iter().collect()
                    };
                    if let Some(p) = filtered_procs.get(selected_idx) {
                        if expanded_groups.contains(&p.name) {
                            expanded_groups.remove(&p.name);
                        } else {
                            expanded_groups.insert(p.name.clone());
                        }
                        re_render = true;
                    }
                }
                Key::Char('x' | 'X' | 'K') => {
                    let filtered_procs: Vec<&ProcessInfo> = if let Some(ref q) = search_query {
                        if !q.is_empty() {
                            let q_lower = q.to_lowercase();
                            cached_procs
                                .iter()
                                .filter(|p| p.name.to_lowercase().contains(&q_lower))
                                .collect()
                        } else {
                            cached_procs.iter().collect()
                        }
                    } else {
                        cached_procs.iter().collect()
                    };
                    if let Some(p) = filtered_procs.get(selected_idx) {
                        let target_pid = p.pid.or_else(|| p.children.first().map(|c| c.pid));
                        if let Some(pid) = target_pid {
                            #[cfg(target_os = "linux")]
                            let (pidfd, st) = (
                                collector::open_pidfd(pid),
                                collector::read_starttime(
                                    std::path::Path::new("/proc"),
                                    &pid.to_string(),
                                ),
                            );
                            kill_prompt = Some(KillTarget {
                                pid,
                                name: p.name.clone(),
                                #[cfg(target_os = "linux")]
                                pidfd,
                                #[cfg(target_os = "linux")]
                                starttime: st,
                            });
                            re_render = true;
                        }
                    }
                }
                Key::Char('+') | Key::Char('=') => {
                    refresh_ms = refresh_ms.saturating_sub(25).max(20);
                    re_render = true;
                }
                Key::Char('-') | Key::Char('_') => {
                    refresh_ms = (refresh_ms + 50).min(2000);
                    re_render = true;
                }
                Key::Char('1') => {
                    group_procs = true;
                    re_render = true;
                }
                Key::Char('2') => {
                    group_procs = false;
                    re_render = true;
                }
                Key::Char('o' | 'O') => {
                    #[cfg(target_os = "linux")]
                    {
                        sort_metric = match sort_metric {
                            SortMetric::Rss => SortMetric::Pss,
                            SortMetric::Pss => SortMetric::Uss,
                            SortMetric::Uss => SortMetric::Name,
                            SortMetric::Name => SortMetric::Rss,
                        };
                    }
                    #[cfg(target_os = "windows")]
                    {
                        sort_metric = match sort_metric {
                            SortMetric::Rss => SortMetric::Uss,
                            SortMetric::Uss => SortMetric::Name,
                            _ => SortMetric::Rss,
                        };
                    }
                    #[cfg(target_os = "macos")]
                    {
                        sort_metric = match sort_metric {
                            SortMetric::Rss => SortMetric::Name,
                            _ => SortMetric::Rss,
                        };
                    }
                    #[cfg(not(any(
                        target_os = "linux",
                        target_os = "windows",
                        target_os = "macos"
                    )))]
                    {
                        sort_metric = match sort_metric {
                            SortMetric::Rss => SortMetric::Name,
                            _ => SortMetric::Rss,
                        };
                    }
                    cached_procs.sort_by(|a, b| {
                        match sort_metric {
                            SortMetric::Rss => b.rss.cmp(&a.rss),
                            SortMetric::Pss => b.pss.unwrap_or(b.rss).cmp(&a.pss.unwrap_or(a.rss)),
                            SortMetric::Uss => b.uss.unwrap_or(b.rss).cmp(&a.uss.unwrap_or(a.rss)),
                            SortMetric::Name => a.name.cmp(&b.name),
                        }
                        .then_with(|| a.name.cmp(&b.name))
                        .then_with(|| a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0)))
                    });
                    re_render = true;
                }
                Key::Char('t') => {
                    current_theme = next_theme(&current_theme).to_string();
                    theme_modal_idx = THEME_NAMES
                        .iter()
                        .position(|&t| t == current_theme)
                        .unwrap_or(0);
                    re_render = true;
                }
                Key::Char('T') => {
                    theme_modal_open = true;
                    theme_modal_idx = THEME_NAMES
                        .iter()
                        .position(|&t| t == current_theme)
                        .unwrap_or(0);
                    re_render = true;
                }
                Key::Char('s' | 'S') => {
                    current_symbol = current_symbol.cycle();
                    re_render = true;
                }
                Key::Char('m' | 'M') => {
                    current_mode = next_cycling_mode(&current_mode).to_string();
                    re_render = true;
                }
                Key::Char('h' | 'H' | '?') => {
                    show_help = !show_help;
                    re_render = true;
                }
                _ => {}
            }
        }

        if re_render {
            let pal = get_palette(&current_theme, color_enabled);
            let (cols, rows) = terminal_size();
            let kill_arg = kill_prompt.as_ref().map(|t| (t.pid, t.name.as_str()));
            let t_modal = if theme_modal_open {
                Some(theme_modal_idx)
            } else {
                None
            };
            let output = render_snapshot(
                &cached_mem,
                &cached_procs,
                group_procs,
                paused,
                &current_mode,
                &current_theme,
                &pal,
                current_symbol,
                color_enabled,
                show_help,
                None,
                true,
                cols,
                rows,
                sort_metric,
                &history,
                show_sparkline,
                Some(selected_idx),
                &expanded_groups,
                search_query.as_deref(),
                search_active,
                kill_arg,
                t_modal,
            );
            let lines: Vec<String> = output.lines().map(|l| l.to_string()).collect();
            if fb.render(&lines, &mut io::stdout()).is_err() {
                break;
            }
        }
    }

    term.restore();
}
