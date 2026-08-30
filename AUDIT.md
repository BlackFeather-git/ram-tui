# ram-tui v0.4.2 — Maintainer Architectural Audit

Date: 2026-08-31  
Maintainer: Raven (BlackFeather) https://github.com/BlackFeather-git/ram-tui  
Repository State: `v0.4.2` (Verified with GitHub Actions CI across Linux, macOS, and Windows)

---

## 1. Executive Summary

`ram-tui` is a single-file, zero-external-dependency terminal memory monitor supporting **Linux, macOS, and Windows**. It provides dynamic visual usage tracks, memory breakdown, process grouping, and machine-readable JSON exports.

This document details the resolution of all confirmed peer audit findings (`C-001` through `C-102`), explicit 64-bit FFI declarations, platform semantics, and CI verification.

---

## 2. Platform Architecture & Data Sources

| Platform | System Memory Source | Process Inspection | Architecture Notes |
|---|---|---|---|
| **Linux** | `/proc/meminfo`, `/proc/swaps` | `/proc/<pid>/statm`, `/proc/<pid>/comm`, `/proc/<pid>/stat` | Direct kernel `/proc` parsing with documented field 22 (`starttime`) cache to prevent PID-reuse races. |
| **macOS** | `vm_stat`, `sysctl` (`hw.memsize`, `vm.swapusage`) | `ps -axo pid,rss,comm` | Python 3.6+ compatible subprocess calls with 1.0s timeout. Commit limit is marked unsupported/NA as macOS does not enforce global commit limits. |
| **Windows** | `GlobalMemoryStatusEx`, `GetPerformanceInfo` (PSAPI) | Tool Help API (`CreateToolhelp32Snapshot`) + `GetProcessMemoryInfo` | Pure `ctypes` Win32 API calls with explicit pointer-sized FFI types (`restype`/`argtypes`). Restores initial console modes on exit. Pagefile capacity reported separately from physical RAM. |

---

## 3. Resolution of Confirmed Findings

1. **`C-101` (Windows 64-bit FFI Declarations)**: Defined explicit `argtypes` and pointer-sized `restype` (e.g. `c_void_p` for `HANDLE` returns from `GetStdHandle`, `CreateToolhelp32Snapshot`, `OpenProcess`, and `CloseHandle`) ensuring total ABI correctness on 64-bit Windows.
2. **`C-102` (Windows Unavailable Cache Semantics)**: If `GetPerformanceInfo` fails, `cached` is set to `None` and rendered as `N/A`, avoiding manufactured zero values.
3. **`C-001` (Python 3.6 / macOS compatibility)**: Replaced Python 3.7+ `text=True` in subprocess calls with `universal_newlines=True` via `run_command()`, ensuring universal compatibility across Python 3.6 through 3.14+.
4. **`C-002` (Windows Commit Fallback)**: Removed fallback calculations that approximated commit from pagefile fields. If `GetPerformanceInfo` fails, `commit_as` and `commit_limit` are set to `None`, preserving total semantic honesty.
5. **`C-003` (Linux Starttime Identity)**: Keyed the process name cache by `(pid, starttime)` where `starttime` is extracted directly from documented field 22 of `/proc/<pid>/stat`.
6. **`C-004` (Windows Console Mode Preservation)**: `TerminalManager` queries and saves the initial Windows console output mode via `GetConsoleMode` and restores it upon `restore()`.
7. **`C-005` (Non-TTY Execution)**: When executed non-interactively in pipelines or redirected output without flags, the application automatically degrades to one-shot snapshot mode instead of running an infinite loop.
8. **`C-006` (Broken Pipe Handling)**: Added clean `BrokenPipeError` exception handling around terminal writes.
9. **`C-007` (Adaptive Narrow-Terminal Layout)**: Main usage bar and process name columns adapt dynamically to terminal widths down to 30 columns.
10. **`C-008` (True ANSI Stripping)**: `sanitize_text()` uses regex (`\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])`) to completely strip full ANSI escape sequences, ASCII control codes, and Unicode directional overrides.
11. **`C-009` (Idempotent Terminal Restore)**: Added `self._restored` state guard to eliminate duplicate restoration escape sequences.
12. **`CI/CD Verification`**: GitHub Actions workflow (`.github/workflows/test.yml`) exercises the matrix of Ubuntu, macOS, and Windows across Python 3.8, 3.10, 3.12, and 3.13.

---

## 4. Automated Testing

The project includes an automated test suite (`tests/test_ram.py`) validating:
- IEC byte formatting and boundary thresholds.
- Percentage bounds and zero-denominator handling.
- Full ANSI sequence and Unicode bidi character removal.
- Linux `/proc/meminfo` parsing and field 22 starttime parsing.
- macOS `vm_stat` regex parsing and unavailable commit state.
- Narrow terminal responsive rendering and unavailable cache state.
- Idempotent terminal cleanup.
- CLI argument bounds (`--rate 20..2000`, `--count 1..10000`).
- Real subprocess CLI JSON stdout execution and schema validation.

Run tests locally:
```bash
python3 -m unittest tests/test_ram.py
```
