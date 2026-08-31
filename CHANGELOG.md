# Changelog

All notable changes to `ram-tui` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0-beta.1] - 2026-08-31

### Added
- **In-Place Self-Updater (`ram --update`)**: Added native self-update command that queries GitHub Releases, verifies download checksum and Python bytecode compilation, validates semantic version declarations, and atomically replaces the installed executable.
- **Cryptographic SHA-256 Integrity Verification**: Validates downloaded source against official release SHA-256 checksums (`ram.sha256`), preventing corrupted or unauthorized file replacements.
- **AST Semantic Source Validation**: Employs Python standard library `ast.parse()` to semantically verify module-level `__version__` declarations and `if __name__ == "__main__":` entry point blocks, preventing docstring spoofing.
- **Offline-First Background Update Checker**: Added non-blocking daemon thread check with configurable 12-hour default cache interval (`RAM_UPDATE_INTERVAL` supporting suffixes like `30m`, `1h`, `12h`), bounded 64 KiB API size limit, quiet footer update notifications, and `--no-update-check` suppression.
- **Inter-Process Lock Guard**: Prevents concurrent instances from issuing duplicate background update checks.
- **Package Manager Safety & `--force` Flag**: Detects system-managed installs (`pacman`, `brew`, `scoop`, `apt`) and warns users to prevent package database desynchronization unless `--force` is passed.
- **Update Inspection CLI (`ram --check-update`)**: Added instant version check command reporting update status without modifying the running installation.

---

## [0.5.3] - 2026-08-31

### Fixed
- **Dynamic Header & Process Column Alignment**: Sized the process column dynamically using `max(hdr_w, ...)` based on the active mode label (`PROCESS (RESIDENT SET)` vs `PROCESS (PID)`), guaranteeing that `RSS`, `USAGE`, and `SHARE` headers align with zero horizontal offset across all terminal widths.

---

## [0.5.2] - 2026-08-31

### Fixed
- **Windows Scoop Launcher Shims**: Corrected relative executable path in `packaging/scoop/ram.cmd` and `packaging/scoop/ram.ps1` (`..\..\ram`) ensuring native CMD and PowerShell launches succeed from Scoop `bin/`.
- **Installer Portability & Safety**: Added unified `fetch_file()` helper supporting both `curl` and `wget` for the binary and all completion scripts, plus interactive prompt protection when replacing existing installations without `--force`.
- **Debian Maintainer Format**: Formatted maintainer contact in `packaging/debian/control` with standard RFC-822 compliant email address.
- **CI Packaging Validation**: Added automated GitHub Actions CI step validating installer dry-run, completion script syntax, and Scoop JSON manifest structure.

---

## [0.5.1] - 2026-08-31

### Fixed
- **Pixel-Perfect Process Table Column Alignment**: Created `pad_plain_cells()` to guarantee exact terminal display-cell padding for process names and right-justified RSS strings to 9 characters, eliminating wobble/horizontal shifts across byte units (KB/MB/GB).
- **Proportional Process Meter Coloring**: Process meters now color-code according to actual system RAM consumption (`<40%` green, `40-70%` yellow, `>70%` red) instead of falsely reporting critical red alarms for harmless top processes.
- **80-Column Breakdown Spacing**: Budgeted the 6-column metrics grid (`USED`, `AVAILABLE`, `TOTAL`, `COMMIT`, `CACHED`, `SWAP`) to 75 columns, preventing `SWAP` descriptors from getting clipped on standard 80-column terminals.

---

## [0.5.0] - 2026-08-31

### Added
- **13 Built-in 24-bit TrueColor Ricing Themes**: `default`, `dracula`, `catppuccin`, `nord`, `tokyo-night`, `gruvbox`, `cyberpunk`, `rose-pine`, `everforest`, `kanagawa`, `monokai`, `solarized`, and `monochrome` with live runtime switching (`t`).
- **Multi-Display Modes**: `--hero` (default), `--compact` (meters only), `--mini` (single bar + percentage), and `--tiny` (single-line status bar format for Waybar/tmux/Polybar) with live hotkey toggling (`m`).
- **Braille & Block Progress Engine**: `--symbol {block,braille}` with live `s` hotkey toggling 2-column horizontal sub-character Braille progress (`⣿`/`⡇`).
- **Multi-Shell Static Completions**: Full static autocompletion scripts for **Bash** (`completions/ram.bash`), **Zsh** (`completions/_ram`), and **Fish** (`completions/ram.fish`).
- **Cross-Platform Distribution Ecosystem**: Packaging formulas and manifests for **Arch Linux AUR** (`PKGBUILD`), **macOS Homebrew** (`packaging/homebrew/ram.rb`), **Windows Scoop** (`packaging/scoop/ram.json`, `ram.cmd`, `ram.ps1`), and **Debian/Ubuntu** (`packaging/debian/`).
- **Interactive Help Overlay**: Active live hotkey cheat sheet (`h` / `?`).

### Fixed & Hardened
- **2D Physical Cell Geometry & Anti-Wrapping Guarantee**: Strict per-line printable cell clamping (`clamp_line_to_cols(line, cols)`) ensuring that no line ever wraps to 2+ physical rows under narrow splits (40x8, 30x6, 20x4).
- **Unicode Display Width Arithmetic**: Standardized terminal display width calculation via `unicodedata` accurately handling wide East Asian characters (CJK = 2 cells), emoji, ZWJ sequences, and zero-width combining characters.
- **Terminal & Alternate Screen Idempotency**: Full Alternate Screen Buffer (`\033[?1049h` / `\033[?1049l`) with re-entrant state tracking (`_raw_active`, `_alt_screen_active`) eliminating scrollback pollution and repeating headers in GPU terminals (Kitty, Alacritty, WezTerm).
- **Instantaneous Input Event Loop**: Pure non-blocking `select.select([self.fd], ..., timeout)` and raw OS read for `<1ms` hotkey responsiveness.
- **Kernel-Safe Platform Architecture**:
  - Linux: PID starttime-keyed cache (`(pid, starttime)`) preventing PID-reuse race conditions and `/sys/block/zram*` fallback detection.
  - Windows: Explicit pointer-sized ctypes Win32 types (`c_void_p`, `c_size_t`) and guaranteed `try...finally: CloseHandle(h_snap)` kernel handle release.
  - macOS: Numeric `sysctl -n` parsing, non-negative value clamping, and compressed RAM accounting.
- **POSIX Pipe & EPIPE Lifecycle**: Protected `--once` and `--json` against `BrokenPipeError` when piped downstream into `head`, `grep`, or jq.

---

## [0.5.0-beta.8] - 2026-08-31

### Added
- **Windows Scoop Shims (`ram.cmd` & `ram.ps1`)**: Added native Command Prompt and PowerShell launcher shims in `packaging/scoop/` for seamless Scoop package integration.
- **Installer Safety Flags**: Added `--dry-run` and `--force` flags to `install.sh` alongside complete per-shell configuration instructions (`~/.bashrc`, `~/.zshrc`, `~/.config/fish/config.fish`, `~/.profile`).

### Fixed
- **Terminal & Alternate Screen Idempotency**: Hardened `setup_raw()` and `restore()` in `TerminalManager` with re-entrant state tracking (`_raw_active`, `_alt_screen_active`), guaranteeing zero terminal corruption on repeated lifecycle calls.
- **Defensive `/proc` Line Parsing**: Added empty-value guards in `get_meminfo_linux()` and `get_processes_linux()` to handle truncated kernel reads without throwing exceptions.
- **ZRAM Fallback Detection**: Added `/sys/block/zram*` presence checking when `/proc/swaps` contains minimal swap descriptors.
- **Grapheme & ZWJ Sequence Handling**: Zero-width joiners (`\u200d`), zero-width spaces, and combining marks now evaluate with 0 printable column width in `char_cell_width()`.

---

## [0.5.0-beta.7] - 2026-08-31

### Added
- **Multi-Shell Static Completions**: Full static autocompletion scripts for **Bash** (`completions/ram.bash`), **Zsh** (`completions/_ram`), and **Fish** (`completions/ram.fish`) with theme descriptions and flag parsing.
- **Cross-Platform Distribution Recipes**: Created official packaging manifests for **Homebrew** (`packaging/homebrew/ram.rb`), **Windows Scoop** (`packaging/scoop/ram.json`), and **Debian/Ubuntu** (`packaging/debian/`).
- **Hardened Secure Installer**: Rewrote `install.sh` with `set -euo pipefail`, atomic `mktemp` downloads, non-destructive PATH diagnostics, and automated user-level shell completion setup.
- **Enhanced Arch Linux `PKGBUILD`**: Added test suite validation via `check()`, `provides=('ram')`/`conflicts=('ram')`, and full multi-shell completion directory installs.

### Fixed
- **POSIX Pipe & EPIPE Lifecycle**: Protected `--once` and `--json` against `BrokenPipeError` when piped downstream into `head`, `grep`, or jq.
- **CI Smoke Matrix**: Hardened `.github/workflows/test.yml` with automated Unix pipeline smoke tests and non-interactive degradation checks across Ubuntu, macOS, and Windows.

---

## [0.5.0-beta.6] - 2026-08-31

### Fixed
- **2D Physical Cell Geometry & Anti-Wrapping Guarantee (`C-001`)**: Implemented complete per-line printable cell clamping (`clamp_line_to_cols(line, cols)`) ensuring that no line ever wraps to 2+ physical rows, preventing horizontal overflow from breaking vertical height bounds in narrow splits (40x8, 30x6, 20x4).
- **Unicode Display Width Arithmetic (`C-002`)**: Standardized terminal display width calculation via `unicodedata` (`char_cell_width`, `visible_cell_width`, `truncate_plain_cells`) accurately handling wide East Asian characters (CJK = 2 cells), emoji, and zero-width combining characters.
- **Alternate Screen Buffer State Tracking (`C-003`)**: Added explicit `_alt_screen_active` state tracking in `TerminalManager` to ensure matching restore sequences only execute when alternate screen entry succeeds.
- **CI Matrix Trigger Coverage (`E-001`)**: Updated `.github/workflows/test.yml` to trigger automated multi-platform testing on the `beta` branch across Ubuntu, macOS, and Windows.

---

## [0.5.0-beta.5] - 2026-08-31

### Fixed
- **Windows Snapshot Handle Ownership (`C-001` / `C-004`)**: Wrapped `CreateToolhelp32Snapshot` process enumeration in strict `try...finally: CloseHandle(h_snap)` block, preventing kernel handle leaks across unexpected exceptions.
- **Strict Viewport Height Budgeting (`C-002`)**: Implemented full-stack viewport height budgeting and progressive degradation (`tiny` for $\le 3$ rows, `mini` for $< 7$ rows, clamped metrics & process slots for larger splits), mathematically guaranteeing that total rendered lines never exceed terminal height.
- **Interactive Help Overlay (`C-003`)**: Enabled active `h` / `?` help cheat sheet across all display modes (`hero`, `compact`, `mini`, `tiny`).

---

## [0.5.0-beta.4] - 2026-08-31

### Fixed
- **Instantaneous Input Event Loop (`select.select` on `fd`)**: Fixed input freeze and dropped key events by replacing `time.sleep` with non-blocking `select.select([self.fd], ..., timeout)` and direct `os.read(self.fd, 64)`. Hotkeys (`t`, `s`, `m`, `q`, `p`, `+`/`-`, `1`/`2`) now trigger immediate `<1ms` response without lag or buffering.

---

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

### Added
- **Unified Multi-Platform Engine**: Integrated Linux (`/proc`), macOS (`sysctl`/`vm_stat`), and Windows (`ctypes` Win32 API) collectors into a unified codebase.
- **ANSI Sanitization Engine**: Added regex-based terminal escape stripping for clean process name display.
- **Non-Interactive Pipeline Support**: Auto-degradation to one-shot mode when redirected to files or pipes.

---

## [0.3.0] - 2026-08-30

### Added
- **Cross-Platform Foundation**: Added native macOS collection via `sysctl` (`hw.memsize`, `vm.swapusage`) and `vm_stat` page accounting, alongside Windows support via `ctypes` (`GlobalMemoryStatusEx`, `GetPerformanceInfo`, and `CreateToolhelp32Snapshot`).
- **Machine-Readable JSON Output**: Added `--json` export mode outputting structured system metrics and process tree for scripting and automation.
- **Zero-Dependency Single-File Packaging**: Consolidated all collectors into a standalone, portable Python executable.

---

## [0.2.0] - 2026-08-30

### Added
- **Comprehensive Memory Breakdown Table**: Displays `Used`, `Available`, `Total`, `Commit Limit` (% committed), `Cached` (reclaimable memory), and `Swap`.
- **ZRAM Compression Discovery**: Added runtime detection for compressed ZRAM swap devices via `/proc/swaps`.
- **Real-Time Interactive Controls**: Added hotkeys for pause/resume (`Space`/`p`), refresh rate tuning (`+`/`-`), and process aggregation toggling (`1` grouped by name, `2` individual PIDs).
- **Responsive Terminal Adaptation**: Dynamic horizontal layout scaling based on terminal columns.

---

## [0.1.0] - 2026-08-30

### Added
- **Initial Prototype**: Core terminal memory monitor for Linux using direct kernel `/proc/meminfo` and `/proc/[pid]/statm` parsing with zero subshell overhead.
- **Visual Memory Gauge**: ANSI usage progress tracks with real-time percentage and IEC byte formatting.
- **Process Memory Ranking**: Grouped resident set size (RSS) process footprints with bounded extraction.
- **Interactive TUI Loop**: Non-blocking keypress handling with live real-time updates.

