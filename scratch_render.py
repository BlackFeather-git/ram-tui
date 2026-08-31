import shutil

THEME = {
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

def render_smooth_bar(ratio, width=32, pal=THEME):
    ratio = max(0.0, min(1.0, ratio))
    total_eighths = int(round(ratio * width * 8))
    full_blocks = total_eighths // 8
    rem_eighths = total_eighths % 8
    empty_blocks = width - full_blocks - (1 if rem_eighths > 0 else 0)

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

print(render_smooth_bar(0.174, 30))
print(render_smooth_bar(0.685, 30))
print(render_smooth_bar(0.925, 30))
