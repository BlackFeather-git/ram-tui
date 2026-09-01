# ram-tui — Maintainer Architectural Audit Log

Date: 2026-09-01  
Maintainer: Raven (BlackFeather) https://github.com/BlackFeather-git/ram-tui  
Latest Verified State: `v0.7.0-rc.4` (12-job CI matrix verified across Ubuntu, macOS, and Windows)  
Historical Reference: Initial audit completed at `v0.4.3`, hardened through `v0.5.x` and `v0.6.x` to `v0.7.0-rc.4`.

---

## 1. Executive Summary

`ram-tui` is a single-file, zero-external-dependency terminal memory monitor supporting **Linux, macOS, and Windows**. It provides dynamic visual usage tracks, memory breakdown, process grouping, 13 TrueColor themes, sub-millisecond differential rendering, cryptographic in-place updates, and machine-readable JSON exports.

This document details the resolution of all confirmed peer audit findings (`C-001` through `C-305`), explicit 64-bit FFI declarations, platform semantics, cryptographic trust chain, and CI verification.

---

## 2. Platform Architecture & Data Sources

| Platform | System Memory Source | Process Inspection | Architecture Notes |
|---|---|---|---|
| **Linux** | `/proc/meminfo`, `/proc/swaps`, `/sys/block/zram*` | `/proc/<pid>/statm`, `/proc/<pid>/comm`, `/proc/<pid>/stat` | Direct zero-subprocess `/proc` parsing with documented field 22 (`starttime`) cache to prevent PID-reuse races. |
| **macOS** | `vm_stat`, `sysctl` (`hw.memsize`, `vm.swapusage`) | `ps -axo pid,rss,comm` | Direct sysctl and Mach page calculations with non-negative arithmetic bounds and 1.0s timeout. |
| **Windows** | `GlobalMemoryStatusEx`, `GetPerformanceInfo` (PSAPI) | Tool Help API (`CreateToolhelp32Snapshot`) + `GetProcessMemoryInfo` | Pure `ctypes` Win32 API calls with explicit pointer-sized FFI types (`restype`/`argtypes`) and robust `INVALID_HANDLE_VALUE` sentinel checks. Restores initial console modes on exit. |

---

## 3. Resolution of Confirmed Findings

1. **`C-301` (Cryptographic Root of Trust)**: Embedded maintainer RSA-2048 public key modulus (`RELEASE_PUBLIC_KEY_N`) and exponent (`RELEASE_PUBLIC_KEY_E`) mathematically verify release digital signatures (`ram.sig`) using pure standard-library arithmetic with strict representative bounds (`0 < s < n`) and `hmac.compare_digest()`.
2. **`C-302` (AST Semantic Source Verification)**: Uses `ast.parse()` to extract top-level `__version__` declarations and verify `if __name__ == "__main__":` entry blocks, preventing docstring/comment spoofing.
3. **`C-303` (TOCTOU & Symlink Guard)**: Verifies physical file and parent directory authenticity immediately prior to atomic `os.replace()` update.
4. **`C-304` (Dynamic Horizontal Centering & SIGWINCH Resize Handling)**: Centers dashboard layout on wide viewports ($>80$ cols), catches `SIGWINCH` resize signals, and appends `\033[K` on every line to eliminate reflow ghost artifacts.
5. **`C-305` (Offline-First Background Checker)**: Implements 12-hour default cache interval (`RAM_UPDATE_INTERVAL`), inter-process file locking, and non-intrusive footer notifications.
6. **`C-201` (Windows Snapshot Error Sentinel Check)**: Explicitly compares `CreateToolhelp32Snapshot` against `c_void_p(-1).value`.
7. **`C-101` (Windows 64-bit FFI Declarations)**: Defined explicit `argtypes` and pointer-sized `restype` ensuring ABI correctness on 64-bit Windows.
8. **`C-102` (Windows Unavailable Cache Semantics)**: `cached` is set to `None` and rendered as `N/A` if unavailable.
9. **`C-001` (Universal Python Compatibility)**: 100% standard library compatible across Python 3.6 through 3.14+ on Linux, macOS, and Windows.
10. **`C-003` (Linux Starttime Identity)**: Keyed the process name cache by `(pid, starttime)` where `starttime` is extracted directly from field 22 of `/proc/<pid>/stat`.
11. **`C-008` (True ANSI Stripping)**: `sanitize_text()` completely strips full ANSI escape sequences, ASCII control codes, and Unicode directional overrides.
12. **`C-009` (Idempotent Terminal Restore)**: `TerminalManager` guarantees idempotent buffer cleanup across signals, atexit, and normal termination.

---

## 4. Automated Testing

The automated test suite (`tests/test_ram.py` and `tests/test_update_flow.py`) includes **52 tests** validating:
- IEC byte formatting and boundary thresholds.
- Percentage bounds and zero-denominator handling.
- Full ANSI sequence and Unicode bidi character removal.
- Linux `/proc/meminfo` parsing and field 22 starttime parsing.
- macOS `vm_stat` regex parsing and unavailable commit state.
- Narrow and wide terminal responsive rendering and centered margins.
- RSA-2048 signature verification, digest validation, and AST semantic checks.
- CLI argument bounds, help overlays, and JSON stdout schema validation.

Run tests locally:
```bash
python3 -m unittest discover tests
```
