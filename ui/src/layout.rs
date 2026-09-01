//! TUI layout engine — renders the dashboard matching the Python v0.7.0 layout.
//!
//! Supports hero, compact, mini, and tiny display modes with proper centering,
//! viewport budgeting, and the same ASCII banner, metrics grid, and process table.

use std::collections::HashSet;

use core_render::{
    ansi::RESET,
    cellwidth::{clamp_line_to_cols, pad_plain_cells, truncate_plain_cells, visible_cell_width},
    color::{interpolate_color, rgb_to_ansi},
    format::{format_bytes, percentage},
    meter::{render_meter_track, GraphSymbol},
    sparkline::render_sparkline,
};

use collector::{MemInfo, ProcessInfo, SortMetric};

use crate::themes::{get_palette, Palette, THEME_NAMES};

/// Render the standalone Theme Selection Window.
pub fn render_standalone_theme_window(
    theme_idx: usize,
    pal: &Palette,
    cols: usize,
    rows: usize,
    enable_color: bool,
    center_vertical: bool,
) -> String {
    let ui_cols = cols.min(80);
    let pad_left = (cols.saturating_sub(ui_cols)) / 2;
    let margin = " ".repeat(pad_left);
    let r = pal.reset;
    let bold = pal.bold;
    let dim = &pal.dim;
    let a = &pal.accent;
    let m = &pal.muted;
    let t = &pal.text;
    let h = &pal.header;

    let mut lines: Vec<String> = Vec::new();
    let box_w = 42;
    let pad_box = " ".repeat((ui_cols.saturating_sub(box_w)) / 2);

    lines.push(clamp_line_to_cols(
        &format!("{pad_box}{h}{bold}RAM-TUI{r}  {m}\u{00b7}{r}  {a}THEME SELECTOR{r}"),
        ui_cols,
    ));
    lines.push(String::new());
    lines.push(clamp_line_to_cols(
        &format!("{pad_box}{a}┌─ SELECT PALETTE (13 Themes) ─────────┐{r}"),
        ui_cols,
    ));
    lines.push(clamp_line_to_cols(
        &format!("{pad_box}{a}│{r}                                      {a}│{r}"),
        ui_cols,
    ));

    for (i, name) in THEME_NAMES.iter().enumerate() {
        let is_sel = i == theme_idx;
        let prefix = if is_sel {
            format!("{bold}{a}▶ {r}")
        } else {
            "  ".to_string()
        };
        let theme_pal = get_palette(name, enable_color);
        let dot = format!("{}●{}", theme_pal.accent, r);
        let name_styled = if is_sel {
            format!("{bold}{a}{:<16}{r}", name)
        } else {
            format!("{t}{:<16}{r}", name)
        };
        lines.push(clamp_line_to_cols(
            &format!("{pad_box}{a}│{r}  {prefix}{dot} {name_styled}        {a}│{r}"),
            ui_cols,
        ));
    }

    lines.push(clamp_line_to_cols(
        &format!("{pad_box}{a}│{r}                                      {a}│{r}"),
        ui_cols,
    ));
    lines.push(clamp_line_to_cols(
        &format!("{pad_box}{a}├──────────────────────────────────────┤{r}"),
        ui_cols,
    ));
    lines.push(clamp_line_to_cols(
        &format!("{pad_box}{a}│{r} {dim}↑/↓ navigate  Enter apply  Esc cancel{r} {a}│{r}"),
        ui_cols,
    ));
    lines.push(clamp_line_to_cols(
        &format!("{pad_box}{a}└──────────────────────────────────────┘{r}"),
        ui_cols,
    ));

    let mut clamped: Vec<String> = lines
        .iter()
        .map(|line| clamp_line_to_cols(&format!("{margin}{line}"), cols))
        .collect();

    if clamped.len() > rows {
        clamped.truncate(rows);
    } else if center_vertical && rows > clamped.len() {
        let pad_top = (rows - clamped.len()) / 2;
        let mut padded = vec![String::new(); pad_top];
        padded.extend(clamped);
        clamped = padded;
    }

    clamped.join("\n")
}

/// Colorize a single banner line across a horizontal gradient.
fn colorize_ascii_banner(
    line: &str,
    stops: &[(f64, (u8, u8, u8))],
    max_w: usize,
    enable_color: bool,
) -> String {
    if !enable_color || stops.is_empty() {
        return line.to_string();
    }
    let mut res = String::with_capacity(line.len() * 20);
    for (i, ch) in line.chars().enumerate() {
        if ch == ' ' {
            res.push(' ');
        } else {
            if let Some(rgb) = interpolate_color(stops, i as f64 / max_w.max(1) as f64) {
                res.push_str(&rgb_to_ansi(rgb));
            }
            res.push(ch);
        }
    }
    res.push_str(RESET);
    res
}

/// Render the full dashboard snapshot.
#[allow(clippy::too_many_arguments)]
pub fn render_snapshot(
    mem: &MemInfo,
    procs: &[ProcessInfo],
    group_procs: bool,
    paused: bool,
    mode: &str,
    theme_name: &str,
    pal: &Palette,
    symbol: GraphSymbol,
    enable_color: bool,
    show_help: bool,
    update_notice: Option<&str>,
    center_vertical: bool,
    cols: usize,
    rows: usize,
    sort_metric: SortMetric,
    history: &[f64],
    show_sparkline: bool,
    selected_idx: Option<usize>,
    expanded_groups: &HashSet<String>,
    search_query: Option<&str>,
    search_active: bool,
    kill_prompt: Option<(u32, &str)>,
    theme_modal_idx: Option<usize>,
) -> String {
    // Standalone theme selector window
    if let Some(t_idx) = theme_modal_idx {
        return render_standalone_theme_window(
            t_idx,
            pal,
            cols,
            rows,
            enable_color,
            center_vertical,
        );
    }

    let hostname = hostname();
    let now_str = local_now();

    let total = mem.total;
    let available = mem.available;
    let used = mem.used;
    let used_pct = percentage(used, total);

    let cols = cols.max(16);
    let rows = rows.max(1);

    let r = pal.reset;
    let t = &pal.text;
    let h = &pal.header;
    let a = &pal.accent;
    let g = &pal.good;
    let w = &pal.warning;
    let crit = &pal.critical;
    let m = &pal.muted;
    let dim = &pal.dim;
    let bold = pal.bold;

    // ---- TINY MODE ----
    if mode == "tiny" || rows <= 3 {
        let used_str = format_bytes(used, 1);
        let total_str = format_bytes(total, 1);
        let mut res = clamp_line_to_cols(
            &format!("RAM: {used_str} / {total_str} ({used_pct:.1}%)"),
            cols,
        );
        if show_help && rows >= 2 {
            let help_line = clamp_line_to_cols(
                &format!("{m}q:quit p:pause t:theme s:symbol m:mode +/-:rate h:help{r}"),
                cols,
            );
            res.push('\n');
            res.push_str(&help_line);
        }
        return res;
    }

    // ---- MINI MODE ----
    if mode == "mini" || rows < 7 {
        let ui_cols = cols.min(64);
        let pad_left = (cols.saturating_sub(ui_cols)) / 2;
        let margin = " ".repeat(pad_left);
        let bar_w = (ui_cols.saturating_sub(28)).clamp(4, 44);
        let ratio = if total > 0 {
            used as f64 / total as f64
        } else {
            0.0
        };
        let meter = render_meter_track(
            ratio,
            bar_w,
            symbol,
            &pal.stops,
            None,
            &pal.good,
            &pal.warning,
            &pal.critical,
            &pal.track,
            r,
        );
        let pct_color = if used_pct < 60.0 {
            g
        } else if used_pct < 85.0 {
            w
        } else {
            crit
        };
        let line1 = clamp_line_to_cols(
            &format!(
                "{margin}{h}{bold}RAM-TUI{r}  {meter}  {pct_color}{bold}{used_pct:>5.1}%{r}  {t}{}{r}{m}/{r}{t}{}{r}",
                format_bytes(used, 1),
                format_bytes(total, 1),
            ),
            cols,
        );
        let mut mini_lines = vec![line1];
        if show_help && rows >= 2 {
            let line2 = clamp_line_to_cols(
                &format!("{margin}{m}{dim}q:quit p:pause t:theme s:symbol m:mode 1/2:group +/-:rate h:help{r}"),
                cols,
            );
            mini_lines.push(line2);
        }
        if center_vertical && rows > mini_lines.len() {
            let pad_top = (rows - mini_lines.len()) / 2;
            let mut padded = vec![String::new(); pad_top];
            padded.extend(mini_lines);
            mini_lines = padded;
        }
        return mini_lines.join("\n");
    }

    // ---- HERO & COMPACT MODES ----
    let ui_cols = cols.min(80);
    let pad_left = (cols.saturating_sub(ui_cols)) / 2;
    let margin = " ".repeat(pad_left);
    let mut lines: Vec<String> = Vec::new();
    let rule = format!("{m}{dim}{}{r}", "\u{2500}".repeat(ui_cols));

    // ASCII Banner (hero mode, large terminal only)
    let banner_raw = [
        "██████╗  █████╗ ███╗   ███╗   ████████╗██╗   ██╗██╗",
        "██╔══██╗██╔══██╗████╗ ████║   ╚══██╔══╝██║   ██║██║",
        "██████╔╝███████║██╔████╔██║█████╗██║   ██║   ██║██║",
        "██╔══██╗██╔══██║██║╚██╔╝██║╚════╝██║   ██║   ██║██║",
        "██║  ██║██║  ██║██║ ╚═╝ ██║      ██║   ╚██████╔╝██║",
        "╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     ╚═╝      ╚═╝    ╚═════╝ ╚═╝",
    ];
    let banner_w = 51;

    if mode == "hero" && ui_cols >= 60 && rows >= 28 {
        let pad_banner = " ".repeat(((ui_cols as isize - banner_w as isize).max(0) as usize) / 2);
        for b_line in &banner_raw {
            let c_line = colorize_ascii_banner(b_line, &pal.stops, banner_w, enable_color);
            lines.push(format!("{pad_banner}{c_line}"));
        }
        lines.push(String::new());
    }

    // Header line
    let state_str = if paused {
        format!("  {w}{bold}paused{r}")
    } else {
        String::new()
    };
    let clock = format!("{m}{now_str}{r}");
    let clock_w = visible_cell_width(&clock);
    let hostname_trunc = truncate_plain_cells(&hostname, 16, "~");

    let os_name = core_render::format::os_display_name();
    let header_left = if ui_cols >= 64 {
        format!(
            "{h}{bold}RAM-TUI{r}  {m}\u{00b7}{r}  {t}{hostname_trunc}{r}  {m}\u{00b7}{r}  \
             {a}{theme_name}{r}  {m}\u{00b7}{r}  {m}{os_name}{r}  {m}\u{00b7}{r}  {m}{mode}{r}{state_str}"
        )
    } else if ui_cols >= 44 {
        format!(
            "{h}{bold}RAM-TUI{r}  {m}\u{00b7}{r}  {a}{theme_name}{r}  {m}\u{00b7}{r}  {m}{mode}{r}{state_str}"
        )
    } else {
        format!("{h}{bold}RAM-TUI{r}  {m}\u{00b7}{r}  {a}{theme_name}{r}{state_str}")
    };

    let left_w = visible_cell_width(&header_left);
    if left_w + clock_w + 2 <= ui_cols {
        let gap = ui_cols - left_w - clock_w;
        lines.push(format!("{header_left}{}{clock}", " ".repeat(gap)));
    } else if left_w <= ui_cols {
        lines.push(header_left);
    } else {
        lines.push(clamp_line_to_cols(&header_left, ui_cols));
    }
    lines.push(rule.clone());

    // Hero usage bar
    let bar_width = (ui_cols.saturating_sub(16)).clamp(4, 64);
    let ratio = if total > 0 {
        used as f64 / total as f64
    } else {
        0.0
    };
    let meter = render_meter_track(
        ratio,
        bar_width,
        symbol,
        &pal.stops,
        None,
        &pal.good,
        &pal.warning,
        &pal.critical,
        &pal.track,
        r,
    );
    let pct_color = if used_pct < 60.0 {
        g
    } else if used_pct < 85.0 {
        w
    } else {
        crit
    };
    lines.push(clamp_line_to_cols(&meter, ui_cols));
    lines.push(clamp_line_to_cols(
        &format!(
            "{pct_color}{bold}{used_pct:>5.1}%{r}  {t}{bold}{}{r}{m} used of {r}{t}{}{r}",
            format_bytes(used, 2),
            format_bytes(total, 2),
        ),
        ui_cols,
    ));

    // Optional 60s historical trend sparkline
    if show_sparkline && !history.is_empty() && (rows >= 14 || mode == "hero") {
        let spark_width = (ui_cols.saturating_sub(24)).clamp(6, 48);
        let spark_line = render_sparkline(
            history,
            spark_width,
            &pal.stops,
            &pal.good,
            &pal.warning,
            &pal.critical,
            r,
        );
        lines.push(clamp_line_to_cols(
            &format!("{m}TREND (60s):{r} {spark_line}  {pct_color}{bold}{used_pct:>5.1}%{r}"),
            ui_cols,
        ));
    }

    // Metrics breakdown
    let footer_reserve = 2;
    let commit_str = format!(
        "{}/{}",
        format_bytes(mem.commit_as, 1),
        format_bytes(
            if mem.commit_limit > 0 {
                mem.commit_limit
            } else {
                total
            },
            1
        )
    );
    let cached_str = format_bytes(mem.cached, 1);
    let swap_str = format!("{} ({})", format_bytes(mem.swap_used, 1), mem.swap_desc);

    if rows >= lines.len() + 2 + footer_reserve {
        if rows >= 22 {
            lines.push(String::new());
        }
        if ui_cols >= 76 {
            lines.push(clamp_line_to_cols(
                &format!(
                    "{m}{:<12}{:<13}{:<12}{:<18}{:<11}{:<14}{r}",
                    "USED", "AVAILABLE", "TOTAL", "COMMIT", "CACHED", "SWAP"
                ),
                ui_cols,
            ));
            lines.push(clamp_line_to_cols(
                &format!(
                    "{g}{bold}{:<12}{r}{t}{bold}{:<13}{r}{t}{bold}{:<12}{r}{t}{:<18}{r}{t}{:<11}{r}{t}{:<14}{r}",
                    format_bytes(used, 2),
                    format_bytes(available, 2),
                    format_bytes(total, 2),
                    commit_str,
                    cached_str,
                    swap_str,
                ),
                ui_cols,
            ));
        } else if ui_cols >= 44 {
            lines.push(clamp_line_to_cols(
                &format!("{m}USED          AVAILABLE     TOTAL{r}"),
                ui_cols,
            ));
            lines.push(clamp_line_to_cols(
                &format!(
                    "{g}{bold}{:<14}{r}{t}{bold}{:<14}{r}{t}{bold}{:<14}{r}",
                    format_bytes(used, 2),
                    format_bytes(available, 2),
                    format_bytes(total, 2),
                ),
                ui_cols,
            ));
        } else {
            lines.push(clamp_line_to_cols(
                &format!(
                    "{m}USED:{r} {g}{bold}{}{r}  {m}AVAIL:{r} {t}{bold}{}{r}",
                    format_bytes(used, 1),
                    format_bytes(available, 1),
                ),
                ui_cols,
            ));
        }
    }

    // Process ranking & collapsible tree (hero mode only)
    if mode == "hero" && !procs.is_empty() {
        let filtered_procs: Vec<&ProcessInfo> = if let Some(q) = search_query {
            let q_lower = q.to_lowercase();
            procs
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&q_lower))
                .collect()
        } else {
            procs.iter().collect()
        };

        let needed_for_proc_header = 1;
        let available_slots = rows
            .saturating_sub(lines.len())
            .saturating_sub(needed_for_proc_header)
            .saturating_sub(footer_reserve);

        if !filtered_procs.is_empty() && available_slots > 0 {
            if rows >= 24 {
                lines.push(String::new());
            }
            let metric_label = match sort_metric {
                SortMetric::Rss => "RSS",
                SortMetric::Pss => "PSS",
                SortMetric::Uss => "USS",
                SortMetric::Name => "RSS",
            };
            let mode_label = if group_procs {
                match sort_metric {
                    SortMetric::Rss => "RESIDENT SET",
                    SortMetric::Pss => "PROP. SET - PSS",
                    SortMetric::Uss => "UNIQUE - USS",
                    SortMetric::Name => "ALPHABETICAL",
                }
            } else {
                "PID"
            };
            let hdr_title = if ui_cols >= 64 {
                format!("PROCESS ({mode_label})")
            } else {
                "PROCESS".to_string()
            };
            let hdr_w = visible_cell_width(&hdr_title);

            let (name_w, p_bar_w) = if ui_cols >= 80 {
                let nw = hdr_w.max((ui_cols.saturating_sub(38)).min(24));
                let pw = (ui_cols.saturating_sub(nw).saturating_sub(24)).clamp(4, 20);
                (nw, pw)
            } else if ui_cols >= 64 {
                let nw = hdr_w.max(22);
                let pw = (ui_cols.saturating_sub(nw).saturating_sub(24)).max(4);
                (nw, pw)
            } else {
                let nw = (ui_cols.saturating_sub(30)).max(8);
                let pw = (ui_cols.saturating_sub(nw).saturating_sub(24)).max(4);
                (nw, pw)
            };

            let hdr_name = pad_plain_cells(&hdr_title, name_w + 2, true);
            let hdr_line = format!(
                "{m}{hdr_name}   {:>9}   {:<bar_w$}   {:>6}{r}",
                metric_label,
                "USAGE",
                "SHARE",
                bar_w = p_bar_w
            );
            lines.push(clamp_line_to_cols(&hdr_line, ui_cols));

            let top_val = filtered_procs.first().map_or(1, |p| match sort_metric {
                SortMetric::Pss => p.pss.unwrap_or(p.rss).max(1),
                SortMetric::Uss => p.uss.unwrap_or(p.rss).max(1),
                _ => p.rss.max(1),
            });

            let mut rendered_rows = 0;
            for (p_idx, p) in filtered_procs.iter().enumerate() {
                if rendered_rows >= available_slots {
                    break;
                }

                let is_selected = selected_idx == Some(p_idx);
                let is_expanded =
                    group_procs && expanded_groups.contains(&p.name) && !p.children.is_empty();

                let count_str = if p.count > 1 {
                    format!(" ({})", p.count)
                } else {
                    String::new()
                };

                let expand_icon = if group_procs {
                    if p.count > 1 {
                        if is_expanded {
                            "▼ "
                        } else {
                            "▸ "
                        }
                    } else {
                        "  "
                    }
                } else {
                    ""
                };

                let cursor_icon = if is_selected {
                    format!("{a}{bold}▶{r} ")
                } else if selected_idx.is_some() {
                    "  ".to_string()
                } else {
                    String::new()
                };

                let name_full = format!("{expand_icon}{}{count_str}", p.name);
                let name_disp = truncate_plain_cells(&name_full, name_w + 1, "~");
                let name_padded = pad_plain_cells(&name_disp, name_w + 1, true);

                let (proc_val, val_str) = match sort_metric {
                    SortMetric::Pss => {
                        let v = p.pss.unwrap_or(p.rss);
                        (v, format!("{:>9}", format_bytes(v, 1)))
                    }
                    SortMetric::Uss => {
                        let v = p.uss.unwrap_or(p.rss);
                        (v, format!("{:>9}", format_bytes(v, 1)))
                    }
                    _ => (p.rss, format!("{:>9}", format_bytes(p.rss, 1))),
                };

                let p_pct = percentage(proc_val, total);
                let p_pct_str = format!("{p_pct:>5.1}%");
                let p_ratio = proc_val as f64 / top_val as f64;
                let p_meter = render_meter_track(
                    p_ratio,
                    p_bar_w,
                    symbol,
                    &pal.stops,
                    None,
                    &pal.good,
                    &pal.warning,
                    &pal.critical,
                    &pal.track,
                    r,
                );

                let name_style = if is_selected {
                    format!("{a}{bold}")
                } else {
                    t.to_string()
                };
                let p_line = format!(
                    "{cursor_icon}{name_style}{name_padded}{r}   {t}{bold}{val_str}{r}   {p_meter}   {m}{p_pct_str:>6}{r}"
                );
                lines.push(clamp_line_to_cols(&p_line, ui_cols));
                rendered_rows += 1;

                // Render children tree if expanded
                if is_expanded {
                    for (c_idx, child) in p.children.iter().enumerate() {
                        if rendered_rows >= available_slots {
                            break;
                        }
                        let is_last = c_idx == p.children.len() - 1;
                        let branch = if is_last { " └─" } else { " ├─" };
                        let child_title = format!("{branch} [{}]", child.pid);
                        let child_disp = truncate_plain_cells(&child_title, name_w + 1, "~");
                        let child_padded = pad_plain_cells(&child_disp, name_w + 1, true);

                        let c_val = match sort_metric {
                            SortMetric::Pss => child.pss.unwrap_or(child.rss),
                            SortMetric::Uss => child.uss.unwrap_or(child.rss),
                            _ => child.rss,
                        };
                        let c_val_str = format!("{:>9}", format_bytes(c_val, 1));
                        let c_pct = percentage(c_val, total);
                        let c_pct_str = format!("{c_pct:>5.1}%");
                        let c_ratio = c_val as f64 / top_val as f64;
                        let c_meter = render_meter_track(
                            c_ratio,
                            p_bar_w,
                            symbol,
                            &pal.stops,
                            None,
                            &pal.good,
                            &pal.warning,
                            &pal.critical,
                            &pal.track,
                            r,
                        );

                        let child_prefix = if selected_idx.is_some() { "  " } else { "" };
                        let c_line = format!(
                            "{child_prefix}{m}{child_padded}{r}   {m}{c_val_str}{r}   {c_meter}   {m}{c_pct_str:>6}{r}"
                        );
                        lines.push(clamp_line_to_cols(&c_line, ui_cols));
                        rendered_rows += 1;
                    }
                }
            }
        }
    }

    // Footer & help overlay / search bar / kill prompt
    lines.push(rule);
    if let Some((k_pid, k_name)) = kill_prompt {
        lines.push(clamp_line_to_cols(
            &format!(
                "{crit}{bold}KILL PROCESS?{r} Send SIGTERM to PID {k_pid} ({k_name})? {bold}[y/N]{r}"
            ),
            ui_cols,
        ));
    } else if search_active {
        let q = search_query.unwrap_or("");
        lines.push(clamp_line_to_cols(
            &format!(
                "{a}{bold}SEARCH:{r} {t}{q}{r}█  {m}{dim}(Enter: apply, Esc: cancel, Backspace: delete){r}"
            ),
            ui_cols,
        ));
    } else if let Some(q) = search_query {
        lines.push(clamp_line_to_cols(
            &format!(
                "{a}{bold}FILTER:{r} \"{t}{q}{r}\"  {m}{dim}(/: edit filter, Esc: clear filter){r}"
            ),
            ui_cols,
        ));
    } else if show_help {
        lines.push(clamp_line_to_cols(
            &format!(
                "{h}{bold}HOTKEYS:{r} {t}q{r} quit  {t}p{r} pause  {t}t{r} theme  {t}T{r} theme menu  \
                 {t}s{r} symbol  {t}m{r} mode  {t}1/2{r} group  {t}o{r} sort  {t}g{r} spark  \
                 {t}↑/↓{r} nav  {t}e/Enter{r} expand  {t}/{r} filter  {t}x/K{r} kill  {t}+/-{r} rate  {t}h{r} close"
            ),
            ui_cols,
        ));
    } else {
        let mut footer_text = format!(
            "{m}{dim}q:quit  p:pause  t/T:theme  s:glyph  m:mode  1/2:grp  o:sort  g:spark  ↑/↓:nav  /:find  h:help{r}"
        );
        if let Some(notice) = update_notice {
            footer_text.push_str(&format!("  {w}{notice}{r}"));
        }
        lines.push(clamp_line_to_cols(&footer_text, ui_cols));
    }

    // Apply margin and clamp
    let mut clamped: Vec<String> = lines
        .iter()
        .map(|line| clamp_line_to_cols(&format!("{margin}{line}"), cols))
        .collect();

    if clamped.len() > rows {
        clamped.truncate(rows);
    } else if center_vertical && rows > clamped.len() {
        let pad_top = (rows - clamped.len()) / 2;
        let mut padded = vec![String::new(); pad_top];
        padded.extend(clamped);
        clamped = padded;
    }

    clamped.join("\n")
}

/// Get the system hostname.
fn hostname() -> String {
    if let Ok(h) = std::env::var("HOSTNAME") {
        return h;
    }
    if let Ok(h) = std::env::var("COMPUTERNAME") {
        return h;
    }
    #[cfg(unix)]
    {
        let mut buf = [0u8; 256];
        let ret = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
        if ret == 0 {
            let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            return String::from_utf8_lossy(&buf[..len]).to_string();
        }
    }
    "unknown".to_string()
}

/// Get current local time as HH:MM:SS.
fn local_now() -> String {
    core_render::format::local_now_hms()
}
