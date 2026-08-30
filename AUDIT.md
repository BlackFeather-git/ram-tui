# ram-tui v0.4.1 — Maintainer Architectural Audit

Date: 2026-08-31  
Maintainer: Raven (BlackFeather) https://github.com/BlackFeather-git/ram-tui  
Repository State: `v0.4.1` (with automated GitHub Actions CI across Linux, macOS, and Windows)

---

## 1. Executive Summary

`ram-tui` is a single-file, zero-external-dependency terminal memory monitor supporting **Linux, macOS, and Windows**. It provides dynamic visual usage tracks, memory breakdown, process grouping, and machine-readable JSON exports.

This audit documents the resolution of all confirmed findings from peer audits (`C-001` through `C-009`), platform semantics, and CI verification.

---

## 2. Platform Architecture & Data Sources

| Platform | System Memory Source | Process Inspection | Architecture Notes |
|---|---|---|---|
| **Linux** | `/proc/meminfo`, `/proc/swaps` | `/proc/<pid>/statm`, `/proc/<pid>/comm`, `/proc/<pid>/stat` | Direct kernel `/proc` parsing with documented field 22 (`starttime`) cache to prevent PID-reuse races. |
| **macOS** | `vm_stat`, `sysctl` (`hw.memsize`, `vm.swapusage`) | `ps -axo pid,rss,comm` | Python 3.6+ compatible subprocess calls with 1.0s timeout. Commit limit is marked unsupported/NA as macOS does not enforce global commit limits. |
| **Windows** | `GlobalMemoryStatusEx`, `GetPerformanceInfo` (PSAPI) | Tool Help API (`CreateToolhelp32Snapshot`) + `GetProcessMemoryInfo` | Pure `ctypes` Win32 API calls. Restores initial console modes on exit. Pagefile capacity reported separately from physical RAM. |

---

## 3. Resolution of Confirmed Findings (v0.4.1)

1. **`C-001` (Python 3.6 / macOS compatibility)**: Replaced Python 3.7+ `text=True` in subprocess calls with `universal_newlines=True` via `run_command()`, ensuring universal compatibility across Python 3.6 through 3.14+.
2. **`C-002` (Windows Commit Fallback)**: Removed fallback calculations that approximated commit from pagefile fields. If `GetPerformanceInfo` fails, `commit_as` and `commit_limit` are set to `None`, preserving total semantic honesty.
3. **`C-003` (Linux Starttime Identity)**: Keyed the process name cache by `(pid, starttime)` where `starttime` is extracted directly from documented field 22 of `/proc/<pid>/stat`.
4. **`C-004` (Windows Console Mode Preservation)**: `TerminalManager` queries and saves the initial Windows console output mode via `GetConsoleMode` and restores it upon `restore()`.
5. **`C-005` (Non-TTY Execution)**: When executed non-interactively in pipelines or redirected output without flags, the application automatically degrades to one-shot snapshot mode instead of running an infinite loop.
6. **`C-006` (Broken Pipe Handling)**: Added clean `BrokenPipeError` exception handling around terminal writes.
7. **`C-007` (Adaptive Narrow-Terminal Layout)**: Main usage bar and process name columns adapt dynamically to terminal widths below 50 columns.
8. **`C-008` (True ANSI Stripping)**: `sanitize_text()` uses regex (`\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])`) to completely strip full ANSI escape sequences, ASCII control codes, and Unicode directional overrides.
9. **`C-009` (Idempotent Terminal Restore)**: Added `self._restored` state guard to eliminate duplicate restoration escape sequences.
10. **`CI/CD Integration`**: Added `.github/workflows/test.yml` running automated unit tests on every push across Ubuntu, macOS, and Windows matrix.

---

## 4. Automated Testing

The project includes an automated test suite (`tests/test_ram.py`) validating:
- IEC byte formatting and boundary thresholds.
- Percentage bounds and zero-denominator handling.
- Full ANSI sequence and Unicode bidi character removal.
- Linux `/proc/meminfo` parsing and field 22 starttime parsing.
- macOS `vm_stat` regex parsing and unavailable commit state.
- Narrow terminal responsive rendering.
- Idempotent terminal cleanup.
- CLI argument bounds (`--rate 20..2000`, `--count 1..10000`).
- JSON output schema and type stability.

Run tests locally:
```bash
python3 -m unittest tests/test_ram.py
```
