# Changelog

All notable changes to `ram-tui` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0-beta.3] - 2026-08-31

### Fixed
- **Terminal Buffer & Scrollback Fix**: Enabled Alternate Screen Buffer (`\033[?1049h` / `\033[?1049l`) to eliminate repeated header duplication and scrollback pollution in GPU terminals (Kitty, Alacritty, WezTerm).
- **Dynamic Viewport Height Clamping**: Automatically adjusts visible process slots to ensure total layout height never exceeds terminal rows or triggers bottom-edge scroll events.
- **Dracula Theme Realignment**: Fixed Dracula theme palette to use signature Dracula Purple (`#bd93f9`), Pink (`#ff79c6`), and Comment (`#6272a4`) accents.
- **Braille Symbol Rendering**: Implemented clean 2-column horizontal Braille progression (`⣿` full, `⡇` half) with unambiguous track rendering (`░`).

### Added
- **Braille Graph Symbol Engine**: Added `--symbol {block,braille}` CLI option and interactive `s` / `S` live toggle hotkey.
- **Expanded Theme Library**: Added `rose-pine`, `everforest`, `kanagawa`, `monokai`, and `solarized` (13 themes total).

---

## [0.5.0-beta.2] - 2026-08-31

### Changed
- **Minimalist Unboxed UI Overhaul**: Replaced rigid box containers and emojis with a clean, breathable typographic layout with quiet horizontal dividers.
- **3-Tier Responsive Layout**: Adaptive stats grid switching dynamically between 6-column single line ($\ge 68$ cols), 3-column wrapped grid ($\ge 50$ cols), and single-column vertical stack ($< 50$ cols for tmux splits).
- **Sub-Character Smooth Gauges**: Fractional 1/8th sub-character unicode rendering (`█▉▊▋▌▍▎▏`) for smooth progress bar tracks.

---

## [0.5.0-beta.1] - 2026-08-31

### Added
- **Theme Engine**: Built-in 24-bit TrueColor palettes (`default`, `catppuccin`, `nord`, `tokyo-night`, `dracula`, `gruvbox`, `cyberpunk`, `monochrome`).
- **Display Modes**:
  - `--hero` (Default full dashboard).
  - `--compact` (Meters & memory breakdown, no process list).
  - `--mini` (Usage track bar + % only).
  - `--tiny` (Single-line raw text for Waybar, Polybar, tmux status bars).
- **Interactive Controls**:
  - `t` / `T`: Cycle color themes live on the fly.
  - `m` / `M`: Cycle display modes live (`hero` -> `compact` -> `mini`).
  - `h` / `?`: Toggle hotkey help overlay footer.
- **Easy Installation**: Added `install.sh` for one-line curl installation and `PKGBUILD` for Arch Linux AUR packaging.

---

## [0.4.3] - 2026-08-31

### Fixed
- Fixed 64-bit Windows `INVALID_HANDLE_VALUE` sentinel check for `CreateToolhelp32Snapshot`.

---

## [0.4.2] - 2026-08-31

### Fixed
- Explicit Win32 64-bit FFI pointer-sized types (`ctypes.c_void_p` for all HANDLE returns).
- Honest Windows cache fallback semantics (`cached = None` / `N/A` when API fails).

---

## [0.4.1] - 2026-08-31

### Added
- Automated GitHub Actions CI across Linux, macOS, and Windows matrix.

### Fixed
- Python 3.6 subprocess compatibility on macOS (`universal_newlines=True`).
- Documented field 22 `starttime` process cache identity on Linux.
- Windows console mode preservation and restoration.
- Non-TTY auto-degradation to one-shot mode.
- Complete regex-based ANSI escape sequence stripping.

---

## [0.4.0] - 2026-08-30
- Initial multi-platform engine supporting Linux (`/proc`), macOS (`sysctl`/`vm_stat`), and Windows (`ctypes` Win32 API).
