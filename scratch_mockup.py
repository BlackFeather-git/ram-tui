import re
import shutil

ANSI_RE = re.compile(r"\x1b\[[0-9;]*[a-zA-Z]")

def visible_len(s):
    return len(ANSI_RE.sub("", s))

def pad_card_line(content, width, border_color="", reset=""):
    vis = visible_len(content)
    padding = max(0, width - 2 - vis)
    return f"{border_color}│{reset}{content}{' ' * padding}{border_color}│{reset}"

PAL = {
    "HEADER": "\033[38;2;203;166;247m",     # Mauve
    "ACCENT": "\033[38;2;116;199;236m",     # Sapphire
    "GOOD": "\033[38;2;166;227;161m",       # Green
    "WARNING": "\033[38;2;249;226;175m",    # Peach
    "CRITICAL": "\033[38;2;243;139;168m",   # Red
    "MUTED": "\033[38;2;147;153;178m",      # Overlay
    "TRACK": "\033[38;2;69;71;90m",         # Surface1
    "BORDER": "\033[38;2;88;91;112m",       # Surface2
    "TEXT": "\033[38;2;205;214;244m",       # Text
    "BOLD": "\033[1m",
    "DIM": "\033[2m",
    "RESET": "\033[0m"
}

def render_smooth_bar(ratio, width=28, pal=PAL):
    ratio = max(0.0, min(1.0, ratio))
    total_eighths = int(round(ratio * width * 8))
    full_blocks = total_eighths // 8
    rem_eighths = total_eighths % 8
    empty_blocks = max(0, width - full_blocks - (1 if rem_eighths > 0 else 0))

    if ratio < 0.60:
        c = pal["GOOD"]
    elif ratio < 0.85:
        c = pal["WARNING"]
    else:
        c = pal["CRITICAL"]

    fractions = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"]
    filled_part = f"{c}{'█' * full_blocks}{fractions[rem_eighths]}{pal['RESET']}"
    empty_part = f"{pal['TRACK']}{'░' * empty_blocks}{pal['RESET']}"
    return f"{filled_part}{empty_part}"

def build_modern_ui():
    p = PAL
    cols = shutil.get_terminal_size((80, 24)).columns
    card_w = min(76, max(38, cols - 2))
    
    b = p["BORDER"]
    r = p["RESET"]
    t = p["TEXT"]
    h = p["HEADER"]
    a = p["ACCENT"]
    g = p["GOOD"]
    w = p["WARNING"]
    m = p["MUTED"]
    dim = p["DIM"]
    bold = p["BOLD"]

    lines = []
    
    # ── Top Rounded Container Header ──────────────────────────────────────────
    title_badge = f" {h}{bold}⚡ RAM-TUI{r} {m}v0.5.0{r} "
    theme_badge = f" {a}catppuccin{r} "
    status_badge = f" {g}● LIVE{r} "
    
    header_title = f"{title_badge}{b}─{r}{theme_badge}{b}─{r}{status_badge}"
    vis_header = visible_len(header_title)
    rem_top_border = max(2, card_w - 2 - vis_header - 1)
    top_line = f"{b}╭─{r}{header_title}{b}{'─' * rem_top_border}╮{r}"
    lines.append(top_line)
    
    # Machine Subheader
    host_info = f" {t}{bold}shadow{r} {m}— Arch Linux x86_64{r}"
    time_info = f"{m}Mon 13:40:00{r} "
    pad_sub = max(2, card_w - 2 - visible_len(host_info) - visible_len(time_info))
    lines.append(f"{b}│{r}{host_info}{' ' * pad_sub}{time_info}{b}│{r}")
    lines.append(f"{b}├{'─' * (card_w - 2)}┤{r}")
    
    # Main Meter Gauge Section
    used_gb = 5.24
    total_gb = 31.01
    avail_gb = 25.77
    pct = (used_gb / total_gb) * 100.0
    
    bar_width = max(10, card_w - 38)
    smooth_bar = render_smooth_bar(used_gb / total_gb, width=bar_width, pal=p)
    pct_color = g if pct < 60 else (w if pct < 85 else p["CRITICAL"])
    
    gauge_line = f"  {t}{bold}RAM{r}  {b}[{r}{smooth_bar}{b}]{r} {pct_color}{bold}{pct:>5.1f}%{r}  {m}({used_gb:.2f}G / {total_gb:.2f}G){r}"
    lines.append(pad_card_line(gauge_line, card_w, b, r))
    lines.append(pad_card_line("", card_w, b, r))
    
    # Detailed Stats Grid
    grid1 = f"  {m}● Used:{r} {g}{bold}5.24 GB{r}      {m}● Available:{r} {g}{bold}25.77 GB{r}      {m}● Total:{r} {t}{bold}31.01 GB{r}"
    lines.append(pad_card_line(grid1, card_w, b, r))
    
    grid2 = f"  {m}⚡ Commit:{r} {t}17.40G/46.51G (37%){r}   {m}📦 Cached:{r} {t}11.28 GB{r}   {m}🔄 Swap:{r} {t}1.38 MB (zram){r}"
    lines.append(pad_card_line(grid2, card_w, b, r))

    # Process Table Section
    lines.append(f"{b}├{'─' * (card_w - 2)}┤{r}")
    table_header = f"  {h}{bold}TOP PROCESSES (RESIDENT SET){r}"
    lines.append(pad_card_line(table_header, card_w, b, r))
    
    procs = [
        ("brave (21)", 4.2 * 1024**3, 13.5),
        ("qs", 689.5 * 1024**2, 2.2),
        ("agy", 431.9 * 1024**2, 1.4),
        ("Hyprland", 198.1 * 1024**2, 0.6),
        ("kitty", 193.8 * 1024**2, 0.6),
        ("Xorg", 135.6 * 1024**2, 0.4),
    ]
    max_p_rss = procs[0][1]
    
    for name, rss, p_pct in procs:
        p_bar_w = max(6, card_w - 48)
        p_bar = render_smooth_bar(rss / max_p_rss, width=p_bar_w, pal=p)
        name_disp = name[:18].ljust(18)
        rss_str = f"{rss / (1024**3 if rss >= 1024**3 else 1024**2):.1f} {'GB' if rss >= 1024**3 else 'MB'}".rjust(8)
        proc_row = f"  {t}{name_disp}{r} {g}{rss_str}{r}  {b}[{r}{p_bar}{b}]{r} {m}{p_pct:>4.1f}%{r}"
        lines.append(pad_card_line(proc_row, card_w, b, r))
    
    # Bottom Rounded Card Footer & Keybindings
    footer_keys = f" {m}{dim}q:quit  p:pause  t:theme  m:mode  1/2:group  +/-:rate{r} "
    vis_footer = visible_len(footer_keys)
    rem_bot_border = max(2, card_w - 2 - vis_footer - 1)
    lines.append(f"{b}╰─{r}{footer_keys}{b}{'─' * rem_bot_border}╯{r}")
    
    return "\n".join(lines)

print(build_modern_ui())
