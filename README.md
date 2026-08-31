<div align="center">

# ram-tui

*A lightweight, aesthetic, zero-dependency real-time terminal memory monitor for Linux, macOS & Windows.*

[![CI](https://github.com/BlackFeather-git/ram-tui/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/BlackFeather-git/ram-tui/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.6+](https://img.shields.io/badge/Python-3.6+-3776AB.svg?logo=python&logoColor=white)](https://www.python.org)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-brightgreen.svg)]()

> *"Small enough to understand. Accurate enough to trust. Fast enough to leave running."*

</div>

---

## Features

- **Kernel-Direct & Ultra-Low Latency:** Sub-millisecond direct reads from native kernel interfaces (`/proc` on Linux, `sysctl`/`vm_stat` on macOS, Win32 PSAPI on Windows).
- **Real-Time Fluid Updates:** Default 50ms (20 FPS) refresh rate with <0.6% single-core CPU utilization and flat memory footprint.
- **Flicker-Free Differential Rendering:** In-place cursor repositioning (`\033[H`) avoids full screen clears and eliminates display flicker.
- **Zero External Dependencies:** Built with Python standard library only (`argparse`, `ctypes`, `subprocess`, `heapq`, `ast`). No `pip`, no wheels, zero setup friction.
- **Cryptographic Self-Updater:** Fail-closed in-place updates verified via maintainer RSA-2048 digital signatures, SHA-256 digests, and AST validation.
- **Alternate Screen Buffer (`\033[?1049h`):** Dedicated terminal buffer prevents scrollback pollution and history corruption in GPU terminals (Kitty, Alacritty, WezTerm).
- **Sub-Character Smooth Gauges:** High-resolution fractional block tracks (`█▉▊▋▌▍▎▏`) and Braille tracks (`⣿⡇`).
- **Built-in Theme Engine:** 13 24-bit TrueColor palettes matching community ricing standards.
- **Multi-Display Modes:**
  - `hero` (Default): Full interactive dashboard with live memory meters and top processes.
  - `--compact`: Memory meters and breakdown grid only.
  - `--mini`: Single usage bar and percentage.
  - `--tiny`: Plain text string for **Waybar**, **Polybar**, and **tmux** status bars.
- **Smart Process Aggregation:** Groups multi-instance processes (e.g. `brave (21)`, `kitty`) with $O(N \log K)$ bounded extraction.
- **Starttime-Keyed PID Cache:** Linux proc starttime keying prevents PID-reuse race conditions.
- **Machine-Readable JSON:** Full `--json` stream and single-shot export for scripting and observability pipelines.

---

## Installation

### Linux & macOS (Quick Install)
```bash
curl -sSL https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/install.sh | bash
```

### Windows (PowerShell One-Liner)
Run in PowerShell or Windows Terminal:
```powershell
irm https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/install.ps1 | iex
```

### Windows (Scoop)
```powershell
scoop install https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/packaging/scoop/ram.json
```

### Arch Linux
```bash
# Option 1: Quick Install
curl -sSL https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/install.sh | bash

# Option 2: Build from included PKGBUILD
git clone https://github.com/BlackFeather-git/ram-tui.git
cd ram-tui && makepkg -si
```

### Manual Installation (Any OS)
Download the standalone `ram` file to any directory in your `PATH`:
```bash
# Linux / macOS
curl -sSL https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/ram -o ~/.local/bin/ram && chmod +x ~/.local/bin/ram

# Windows (Command Prompt / PowerShell)
curl -sSL https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/ram -o %USERPROFILE%\.local\bin\ram.py
```

---

## Interactive Hotkeys

While running interactively, press:

| Key | Action |
|:---|:---|
| `q` / `Ctrl+C` | Quit |
| `Space` / `p` | Pause / Resume real-time updates |
| `t` / `T` | Cycle color theme live (**Dracula**, **Catppuccin**, **Nord**, **Tokyo Night**, etc.) |
| `s` / `S` | Toggle meter graph symbol live (**Block** `█` <-> **Braille** `⣿`) |
| `m` / `M` | Cycle display mode live (**Hero** -> **Compact** -> **Mini**) |
| `1` | Group processes by name (default) |
| `2` | Show individual process PIDs |
| `+` / `=` | Increase refresh rate (+25ms) |
| `-` / `_` | Decrease refresh rate (-50ms) |
| `h` / `?` | Toggle hotkey help footer |

---

## CLI Usage

```bash
# Launch interactive monitor with Dracula theme & Braille symbols
ram --theme dracula --symbol braille

# Launch with Catppuccin theme
ram --theme catppuccin

# Compact mode (meters only) with Nord theme
ram --compact --theme nord

# Mini mode for small terminal splits
ram --mini --theme cyberpunk

# Single-line output for Waybar / tmux status bars
ram --tiny --once

# One-shot snapshot
ram --once

# Machine-readable JSON output
ram --json --once

# Custom refresh rate (250ms) and top 12 processes
ram -r 250 -n 12

# Check for updates without modifying installation
ram --check-update

# Update to latest release in-place with cryptographic verification
ram --update
```

---

## Self-Updater & Security Architecture

`ram-tui` includes a built-in cryptographic updater designed with defense-in-depth:

```text
embedded maintainer RSA public key
            |
download ram + ram.sha256 + ram.sig
            |
SHA-256 checksum verification
            |
RSA-2048 PKCS#1 v1.5 signature verification
            |
Python bytecode compilation & AST validation
            |
TOCTOU symlink check & atomic replacement
```

### Security Properties
1. **Independent Root of Trust:** The updater verifies the official release signature (`ram.sig`) against an embedded maintainer RSA-2048 public key before any code execution.
2. **Fail-Closed Integrity:** Verifies SHA-256 digest (`ram.sha256`) and full Base64 format validation.
3. **AST Semantic Validation:** Parses the downloaded Python source tree to confirm authentic `__version__` declarations and standard entry blocks, preventing docstring spoofing.
4. **TOCTOU & Symlink Guard:** Rejects symlink substitutions and validates directory permissions prior to atomic file swap.
5. **Package Manager Protection:** Detects system-managed paths (`pacman`, `apt`, `dnf`, `brew`, `scoop`) and requires `--force` to prevent package database conflicts.

### Updater Commands & Configuration
| Flag / Variable | Description |
|:---|:---|
| `ram --update` | Perform cryptographically verified in-place update. |
| `ram --update --force` | Force update even if package manager or matching version is detected. |
| `ram --check-update` | Check for updates immediately and print status. |
| `ram --no-update-check` | Disable background update checks during interactive sessions. |
| `RAM_UPDATE_INTERVAL` | Set background check interval (`12h`, `1d`, `30m`, `never`). Default: `12h`. |

---

## Color Themes

Choose your aesthetic with `--theme <name>`:

| Theme | Description |
|:---|:---|
| `default` | Dynamic gradient (Green <60%, Yellow <85%, Red >= 85%) |
| `dracula` | Official Dracula Vampire (Purple, Pink, Cyan, Red) |
| `catppuccin` | Catppuccin Mocha (Mauve, Sapphire, Teal, Peach) |
| `nord` | Arctic Frost Cyan (Nord8) and Snow Storm palette |
| `tokyo-night` | Tokyo Night neon storm cyan and magenta |
| `gruvbox` | Gruvbox retro dark aqua and warm orange |
| `cyberpunk` | High-contrast neon hot pink, electric yellow, and cyan |
| `rose-pine` | Rose Pine (Iris, Love, Foam, Gold) |
| `everforest` | Cozy Everforest (Forest Green, Sage, Warm Yellow) |
| `kanagawa` | Kanagawa Wave Blue, Sakura Pink, and Fuji White |
| `monokai` | Monokai Pro (Magenta, Bright Green, Cyan, Yellow) |
| `solarized` | Solarized Dark (Cyan, Blue, Warm Yellow) |
| `monochrome` | Pure grayscale / ANSI-free (auto-used on `NO_COLOR`) |

---

## Testing

Run the full cross-platform test suite locally (48 tests):

```bash
python3 -m unittest discover tests
```

---

## License

MIT License (c) 2026 Raven (BlackFeather) — See [LICENSE](LICENSE) for details.
