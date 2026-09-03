# ram-tui — Maintainer Architectural & Security Audit Log

Date: 2026-09-02  
Maintainer: Raven (BlackFeather) https://github.com/BlackFeather-git/ram-tui  
Latest Verified State: `v1.0.3` (Maintenance Release: Sanitized Hostname, Fail-Closed Checksums, Windows Swap Telemetry & Theme Modal Border Fix)  
Historical Reference: Initial audit completed at `v0.4.3`, hardened through `v0.5.x`–`v0.7.0` (Python), and fully re-architected in Rust for `v1.0.0`–`v1.0.3`.

---

## 1. Executive Summary

`ram-tui v1.0.3` is a standalone, ultra-low-overhead terminal memory monitor and process telemetry engine written in pure, native Rust. It delivers sub-millisecond execution latency, a 2.2MB stripped binary footprint, and deep kernel telemetry (PSS, USS, Cgroups v2/v1 container detection).

This document details the architectural integrity, resolution of confirmed audit findings (`C-001` through `C-405`), memory safety guarantees, zero-allocation render loop, and multi-platform native subsystems.

---

## 2. Platform Architecture & Data Sources

| Platform | System Memory Source | Process Inspection | Architecture Notes |
|:---|:---|:---|:---|
| **Linux** | `/proc/meminfo`, `/proc/swaps`, `/sys/block/zram*`, Cgroups v2/v1 | `/proc/<pid>/statm`, `/proc/<pid>/comm`, `/proc/<pid>/smaps_rollup` (PSS/USS) | Single-pass zero-subprocess procfs scanning. Fast RSS candidate pre-filtering ensures `smaps_rollup` page table inspections complete in <0.5ms. |
| **macOS (Darwin)** | Mach kernel `host_statistics64` (`HOST_VM_INFO64`), `sysctlbyname` (`hw.memsize`, `vm.swapusage`) | `proc_listpids` (`PROC_ALL_PIDS`), `proc_pidinfo` (`PROC_PIDTASKINFO`) | Pure Mach kernel FFI via `libc`. Zero subprocess fork/exec overhead. Direct page-to-byte sizing with non-negative arithmetic bounds. |
| **Windows** | Win32 `GlobalMemoryStatusEx` (`MEMORYSTATUSEX`) | PSAPI `K32EnumProcesses` + `K32GetProcessMemoryInfo` | Direct Win32 API FFI. Captures physical Working Set Size (RSS) and Private Commit (USS) with robust `INVALID_HANDLE_VALUE` sentinel checks. |

---

## 3. Resolution of Findings & Architectural Hardening

1. **`C-401` (Deep Kernel Telemetry: PSS & USS)**: Integrated `/proc/<pid>/smaps_rollup` parsing to report true proportional memory consumption (PSS) and private dirty/clean memory (USS), eliminating shared-library overcounting in multi-process workloads.
2. **`C-402` (Container Boundary Detection)**: Implemented Linux Cgroups v2 (`/sys/fs/cgroup/memory.max`) and Cgroups v1 limit detection, automatically reflecting container memory constraints in Docker, Podman, and Kubernetes.
3. **`C-403` (Flicker-Free Differential Frame Buffer)**: Developed double-buffered frame diffing in `core_render::framebuf::FrameBuffer`, emitting only modified rows per frame to eliminate terminal flicker and reduce I/O overhead.
4. **`C-404` (Auto-Ranging Dynamic Sparkline)**: Implemented dynamic spread calculation ($\Delta = \max - \min$) over the 60-second historical window in `core_render::sparkline::render_sparkline`, rendering responsive fluid wave glyphs (` ▂▃▄▅▆▇█`).
5. **`C-405` (Bounded Cursor & Interactive Filter Stability)**: Bounded `selected_idx` strictly to `filtered_procs.len().saturating_sub(1)` and separated search typing state (`search_active`) from locked filter state (`search_query`).
6. **`C-301` (Atomic In-Place Updater)**: Implemented atomic temporary file replacement (`.ram-update-*.tmp` + `fsync` + `chmod 755` + `std::fs::rename`) with strict SHA-256 digest validation.
7. **`C-303` (TOCTOU & Symlink Guard)**: Prevents symlink hijacking by validating the physical binary path and directory write permissions prior to atomic replacement.
8. **`C-003` (PID-Reuse Safety)**: Keyed process starttime identity from field 22 of `/proc/<pid>/stat` to guard against PID wraparound during prolonged monitoring sessions.
9. **`C-008` (Unicode & ANSI Sanitization)**: `sanitize_text()` strips ANSI escape sequences, ASCII control characters, and Unicode directional overrides (Bidi controls).
10. **`C-009` (Idempotent Terminal Restore)**: `TerminalManager` guarantees cleanup of raw mode, alternate screen buffers (`\x1b[?1049l`), and cursor visibility (`\x1b[?25h`) across normal exit, panic hooks, and termination signals.

---

## 4. Automated Testing & Verification Suite

The test suite across the Cargo workspace consists of **71 automated tests**:
* **`collector` (32 unit tests)**: Validates `/proc/meminfo`, zram/disk swap detection, Cgroups v2/v1 boundary parsing, PID-reuse starttime tracking, RSS/PSS/USS aggregation, RSS pre-sort candidate enrichment, and process hierarchy grouping.
* **`core_render` (26 unit tests)**: Validates Unicode cell-width calculation (CJK, ZWJ, combining characters), TrueColor RGB gradient interpolation, IEC byte boundary formatting, frame-buffer row diffing, civil timestamp conversion, sanitized system hostname procurement, and leap-year calendar algorithms.
* **`ui` (6 unit tests)**: Validates 13 TrueColor theme palettes, mode cycling, monochrome fallback formatting, and pixel-perfect theme selector modal box alignment across all palettes.
* **`cli` (7 integration tests)**: Validates `--once` execution, CLI help documentation, sorting arguments (`--sort pss|uss|rss|name`), process count range enforcement (`-n 1..=10000`), JSON telemetry schema conformance, `--spark` rolling trend flag, and zero-emoji regression invariant.

---

## 5. Verification Commands

Run full workspace tests:
```bash
cargo test --workspace
```

Run strict clippy linter:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```
