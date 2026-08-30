# ram-tui v0.4.0 — Maintainer Architectural Audit

Date: 2026-08-31  
Maintainer: BlackFeather (https://github.com/BlackFeather-git/ram-tui)  
Repository State: `v0.4.0`

---

## 1. Executive Summary

`ram-tui` is a single-file, zero-external-dependency, real-time terminal memory monitor supporting **Linux, macOS, and Windows**. It provides dynamic visual usage tracks, memory breakdown, process grouping, and machine-readable JSON exports.

This document serves as the formal technical blueprint detailing cross-platform memory accounting, kernel API choices, security controls, and known operating-system trade-offs.

---

## 2. Platform Architecture & Data Sources

| Platform | System Memory Source | Process Inspection | Architecture Notes |
|---|---|---|---|
| **Linux** | `/proc/meminfo`, `/proc/swaps` | `/proc/<pid>/statm`, `/proc/<pid>/comm` | Direct kernel `/proc` parsing with starttime-keyed cache to prevent PID-reuse race conditions. |
| **macOS** | `vm_stat`, `sysctl` (`hw.memsize`, `vm.swapusage`) | `ps -axo pid,rss,comm` | Native page-size calculation & regex swap extraction. Commit limit is marked unsupported/NA as macOS does not enforce Linux/Windows commit limits. |
| **Windows** | `GlobalMemoryStatusEx`, `GetPerformanceInfo` (PSAPI) | Tool Help API (`CreateToolhelp32Snapshot`) + `GetProcessMemoryInfo` | Pure `ctypes` Win32 API calls (no `tasklist` subprocess overhead). Pagefile capacity reported separately from physical RAM. |

---

## 3. Memory Semantics & Accounting Contracts

### Linux
- **Used**: Computed as `MemTotal - MemAvailable`. `MemAvailable` is the Linux kernel's official estimate of memory usable without swapping.
- **Commit**: `Committed_AS` vs `CommitLimit`. Represents total virtual address space currently allocated under memory overcommit policies (`/proc/sys/vm/overcommit_memory`).
- **Cached**: Sum of `Cached + Buffers + SReclaimable`. Represents memory immediately reclaimable by the kernel under pressure.
- **Swap**: Dynamic distinction between compressed in-RAM swap (`zram`) and disk partitions via `/proc/swaps`.

### macOS
- **Used**: `Total - (Free + Inactive + Speculative) * PageSize`.
- **Cached**: `(Inactive + Speculative) * PageSize`.
- **Commit**: Explicitly unsupported (`N/A`) because macOS Virtual Memory does not expose a global commit charge or commit limit equivalent to Linux/Windows.
- **Swap**: Extracted via `sysctl vm.swapusage` distinguishing total allocated swap files from active swap usage.

### Windows
- **Used**: `TotalPhysical - AvailPhysical` via `GlobalMemoryStatusEx`.
- **Commit**: System-wide `CommitTotal * PageSize` vs `CommitLimit * PageSize` via `GetPerformanceInfo` (PSAPI / Kernel32).
- **Swap / Pagefile**: Reported as system paging file capacity (`CommitLimit - TotalPhysical`). Actual pagefile residency is not claimed because Windows APIs do not expose exact per-file resident byte counters.

---

## 4. Performance & Scheduling

1. **Monotonic Deadline Scheduler**: The main TUI loop uses `time.monotonic()` target deadlines rather than cumulative `time.sleep()`. This guarantees zero refresh drift even when process inspection takes several milliseconds.
2. **PID Name Cache with Starttime Isolation**: Linux process names are cached using `(pid, start_time)` tuples, cutting `/proc` disk reads by >70% while completely preventing stale-name display on fast PID reuse.
3. **Bounded Top-N Selection**: Process tables use `heapq.nlargest` ($O(N \log K)$) rather than full in-memory sorting of all system processes.

---

## 5. Security & Terminal Integrity

- **ANSI / Control-Character Sanitization**: Process names are untrusted strings. `sanitize_text()` filters out ASCII control codes (0–31, 127), raw ANSI escape sequences, and Unicode directional overrides (e.g. `\u202e`) to prevent terminal injection attacks.
- **Privilege Requirements**: Zero root or administrative privileges required. Restricted or disappearing processes are handled gracefully without application crashes.
- **Zero Network / Zero Telemetry**: Pure local standard-library execution.

---

## 6. Automated Testing

The project includes a deterministic test suite (`tests/test_ram.py`) validating:
- IEC byte formatting and boundary thresholds.
- Percentage bounds and zero-denominator handling.
- ANSI and Bidirectional Unicode character stripping.
- Linux `/proc/meminfo` missing-field resilience and ZRAM detection.
- macOS `vm_stat` regex parsing and unavailable commit state.
- CLI argument bounds (`--rate 20..2000`, `--count 1..10000`).
- JSON output schema and type stability.

Run tests via:
```bash
python3 -m unittest tests/test_ram.py
```
