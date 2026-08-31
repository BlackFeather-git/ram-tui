# Changelog

All notable changes to `ram-tui` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
