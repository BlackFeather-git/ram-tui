<div align="center">

# ⚡ ram-tui

*A lightweight, aesthetic, zero-dependency real-time terminal memory monitor for Linux, macOS & Windows.*

[![CI](https://github.com/BlackFeather-git/ram-tui/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/BlackFeather-git/ram-tui/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.6+](https://img.shields.io/badge/Python-3.6+-3776AB.svg?logo=python&logoColor=white)](https://www.python.org)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-brightgreen.svg)]()

> *"Small enough to understand. Accurate enough to trust. Fast enough to leave running."*

</div>

---

## ✨ Features

- 🏎️ **Ultra-Fast & Kernel-Direct:** Sub-millisecond reads from native kernel interfaces (`/proc`, `sysctl`/`vm_stat`, Win32 API) with zero subshell overhead.
- 📦 **Zero External Dependencies:** Built entirely with Python's standard library (`argparse`, `ctypes`, `subprocess`, `heapq`). No `pip`, no wheels, no external packages.
- 🎨 **Built-in Theme Engine:** Beautiful 24-bit TrueColor palettes matching popular ricing themes (**Catppuccin**, **Nord**, **Tokyo Night**, **Dracula**, **Gruvbox**, **Cyberpunk**, and **Monochrome**).
- 🎛️ **Multiple Display Modes:**
  - **Hero (Default):** Full visual dashboard with memory stats and top processes.
  - **Compact (`--compact`):** Memory meters only, omitting process lists for small windows.
  - **Mini (`--mini`):** Ultra-minimal usage bar and percentage for tiny terminal splits.
  - **Tiny (`--tiny`):** Single-line raw text for **Waybar**, **Polybar**, and **tmux** status bars.
- 📊 **Smart Process Aggregation:** Automatically groups multi-process instances (e.g. `brave (21)`, `kitty`) by resident set size (RSS) with $O(N \log K)$ bounded extraction.
- 🔒 **Starttime-Keyed PID Cache:** Keyed by documented field 22 starttime, completely eliminating PID-reuse race conditions.
- 🤖 **Machine-Readable JSON:** Full `--json` export mode for scripting, pipelines, and alerting.
- 🛡️ **Terminal-Safe:** Complete stripping of ANSI escape sequences, control codes, and Unicode directional overrides.

---

## 🚀 Quick Install

### One-Line Script (Linux / macOS)
```bash
curl -sSL https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/install.sh | bash
```

### Arch Linux (AUR)
```bash
yay -S ram-tui
```

### Manual Download
```bash
curl -sSL https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/ram -o ~/.local/bin/ram
chmod +x ~/.local/bin/ram
```

---

## 🎮 Interactive Hotkeys

While running interactively, press:

| Key | Action |
| `q` / `Ctrl+C` | Quit |
| `Space` / `p` | Pause / Resume real-time updates |
| `t` / `T` | Cycle color theme live (**Dracula**, **Catppuccin**, **Nord**, **Tokyo Night**, etc.) |
| `s` / `S` | Toggle meter graph symbol live (**Block** `█` $\leftrightarrow$ **Braille** `⣿`) |
| `m` / `M` | Cycle display mode live (**Hero** $\rightarrow$ **Compact** $\rightarrow$ **Mini**) |
| `1` | Group processes by name (default) |
| `2` | Show individual process PIDs |
| `+` / `=` | Increase refresh rate (+25ms) |
| `-` / `_` | Decrease refresh rate (-50ms) |
| `h` / `?` | Toggle hotkey help footer |

---

## 💻 CLI Usage & Options

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
ram --json

# Custom refresh rate (250ms) and top 12 processes
ram -r 250 -n 12
```

---

## 🎨 Color Themes

Choose your aesthetic with `--theme <name>`:

| Theme | Preview Description |
|---|---|
| `default` | Dynamic gradient (Green <60%, Yellow <85%, Red $\ge$ 85%) |
| `dracula` | Official Dracula Vampire (Purple, Pink, Cyan, Red) |
| `catppuccin` | Catppuccin Mocha (Mauve, Sapphire, Teal, Peach) |
| `nord` | Arctic Frost Cyan (Nord8) and Snow Storm palette |
| `tokyo-night` | Tokyo Night neon storm cyan and magenta |
| `gruvbox` | Gruvbox retro dark aqua and warm orange |
| `cyberpunk` | High-contrast neon hot pink, electric yellow, and cyan |
| `rose-pine` | Rosé Pine (Iris, Love, Foam, Gold) |
| `everforest` | Cozy Everforest (Forest Green, Sage, Warm Yellow) |
| `kanagawa` | Kanagawa Wave Blue, Sakura Pink, and Fuji White |
| `monokai` | Monokai Pro (Magenta, Bright Green, Cyan, Yellow) |
| `solarized` | Solarized Dark (Cyan, Blue, Warm Yellow) |
| `monochrome` | Pure grayscale / ANSI-free (auto-used on `NO_COLOR`) |

---

## 🧪 Testing

Run the automated test suite locally:

```bash
python3 -m unittest discover tests
```

---

## 📜 License

MIT License © 2026 Raven (BlackFeather) — See [LICENSE](LICENSE) for details.
