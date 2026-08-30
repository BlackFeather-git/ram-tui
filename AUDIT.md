# ram-tui v0.3.1 — Maintainer Audit

Date: 2026-08-30  
Auditor: Lead-maintainer review  
Repository snapshot reviewed: `ram`, `README.md`

## A. Repository summary

The supplied repository snapshot contains two files: the executable Python script `ram` and `README.md`. The README describes a zero-external-dependency terminal memory monitor for Linux, macOS and Windows, with interactive mode, one-shot mode, JSON mode, process grouping, and adjustable refresh/count controls. fileciteturn0file0L24-L33 fileciteturn0file0L47-L73

- Entry point: `ram`.
- Architecture in v0.3.1: one monolithic Python module.
- Platform collection:
  - Linux: `/proc/meminfo`, `/proc/swaps`, `/proc/<pid>/statm`, `/proc/<pid>/comm`, `/proc/<pid>/cmdline`.
  - Windows: `ctypes` for system memory, `tasklist /FO CSV /NH` for processes.
  - macOS: `sysctl`, `vm_stat`, `ps`.
- Runtime dependencies: Python standard library only.
- CLI: `-r/--rate`, `-n/--count`, `-1/--once`, `--json`, `--no-group`, `-v/--version`.
- Interactive controls: quit, pause/resume, refresh-rate adjustment, grouping toggle.
- JSON output: one snapshot containing timestamp, hostname, OS, memory, and top processes.
- Tests: no test suite was present in the supplied snapshot.
- README support claim: Python 3.6+ and Linux/macOS/Windows. fileciteturn0file0L40-L44 fileciteturn0file0L54-L73

## B. Correctness audit

### Confirmed bugs

1. **Windows "swap used" calculation is not a valid pagefile-usage metric — High-priority bug.**
   `GlobalMemoryStatusEx` exposes total/available physical memory and total/available page-file-backed commit capacity, but the implementation derives `swap_used` as `commit_as - physical_used`. That quantity is not actual pagefile residency. Microsoft documents the fields as physical memory and paging-file/commit-related values, not a direct pagefile-residency counter.

2. **Windows pagefile capacity is conflated with swap usage — High-priority bug.**
   The UI labels the derived values as swap/pagefile usage even though the underlying API does not supply that exact measurement.

3. **macOS "used" memory is an approximation — Medium-priority documentation/correctness issue.**
   `total - (free + inactive + speculative)` is a useful display approximation, but it is not semantically identical to Linux `MemTotal - MemAvailable`. It needs to be presented as best-effort.

4. **Linux cached value is an aggregate estimate — Medium-priority documentation issue.**
   `Cached + Buffers + SReclaimable` combines counters whose meanings overlap in Linux memory accounting. The kernel documentation explicitly warns that `/proc/meminfo` counters overlap and do not necessarily sum to total memory.

5. **Fallback memory snapshot uses `total=1` — Medium-priority correctness issue.**
   Returning a fabricated total allows the UI to look numeric when collection has actually failed. This can make an unavailable metric appear valid.

6. **Negative process-count behavior is incorrect — Medium-priority CLI bug.**
   Python slicing with a negative count silently produces an unintended list rather than rejecting invalid input.

7. **Zero process-count behavior is inconsistent — Medium-priority CLI bug.**
   `--count 0` is accepted and produces no processes instead of rejecting an invalid configuration.

8. **Refresh-rate validation is incomplete — Medium-priority CLI bug.**
   Values below 20 ms are silently clamped rather than rejected, while the documented keybinding range is 20–2000 ms.

9. **Interactive scheduling uses `time.sleep()` from the start of each loop — Performance/reliability improvement.**
   Collection and rendering time are added to the refresh interval, causing refresh drift. A monotonic deadline scheduler is preferable.

10. **Windows process enumeration shells out to `tasklist` — Portability/performance improvement.**
    It is not shell-injection-prone because the command is passed as an argument list, but it introduces an avoidable external-process dependency and parsing overhead every refresh.

11. **Process names are not sanitized before terminal rendering — High-priority security/display bug.**
    Process names are system-derived but untrusted input. ANSI/control characters should not be allowed to reach the terminal renderer.

12. **JSON mode inherits invalid count behavior — Medium-priority CLI bug.**
    JSON mode slices the process list before validating the count.

13. **JSON mode has no explicit schema/version field — Low-priority maintainability improvement.**
    A version field makes machine-readable consumers better able to reason about changes.

14. **Terminal clear sequence is stronger than necessary — Low-priority terminal improvement.**
    `ESC[H ESC[J` is valid on common terminals but a consistent home+clear strategy should be documented and kept isolated.

15. **Exception handling is overly broad in several collection paths — Maintainability improvement.**
    Broad `except Exception` blocks prevent crashes but also hide programming errors. Expected OS/race errors should be handled specifically where practical.

### Confirmed positive design choices

- Linux process collection already avoids a shell.
- PID name caching reduces repeated name reads.
- Process disappearance is generally handled by catching `FileNotFoundError`/permission errors.
- `GlobalMemoryStatusEx` is the correct Windows system-memory API family.
- The project has no network, telemetry, tracking, or third-party runtime dependency.

## C. Cross-platform audit

### Linux

- `/proc/meminfo` is correctly treated as optional-field data rather than a fixed schema.
- `MemAvailable` is the appropriate primary availability metric when present.
- `/proc/<pid>/statm` is lightweight, but Linux documents its RSS field as potentially inaccurate; exact `smaps`-based measurement would be substantially more expensive.
- `/proc/swaps` is suitable for distinguishing zram from disk-backed swap on typical Linux systems.
- Permission/race failures during process inspection should remain per-process failures rather than snapshot failures.

### macOS

- `hw.memsize` is appropriate for physical RAM size.
- `vm_stat` page-size parsing is necessary and is retained.
- macOS memory semantics differ from Linux; labels should avoid implying exact Linux-equivalent "available" or "cached" semantics.
- `ps` remains a pragmatic process fallback because the `kinfo_proc` ABI is architecture/release-sensitive; replacing it with guessed ctypes layouts would risk correctness.

### Windows

- `GlobalMemoryStatusEx` is the appropriate standard-library-accessible native API for system memory.
- Process enumeration can be moved from `tasklist` to Tool Help + `GetProcessMemoryInfo`, avoiding a subprocess and reducing parsing overhead.
- Access-denied processes must simply be skipped.
- Unicode process names should use the wide-character Windows APIs.

## D. Performance audit

Main v0.3.1 costs:

- One `/proc` directory listing per refresh.
- Up to several file opens per Linux process when names are not cached.
- Full process enumeration and sorting every refresh.
- A subprocess for Windows process enumeration.
- A subprocess for macOS process enumeration.
- Full terminal redraw every refresh.
- Refresh drift from sleep-after-work scheduling.

The largest safe wins are:
1. validate/clamp configuration once;
2. use monotonic scheduling;
3. avoid unnecessary redraw while paused;
4. keep Linux name caching;
5. replace Windows `tasklist` with native process enumeration;
6. keep process sorting deterministic;
7. avoid expensive exact RSS sources such as `smaps` for the default fast monitor.

## E. Reliability audit

Required improvements:
- explicit CLI validation;
- graceful broken-pipe handling;
- safe terminal restoration;
- narrow-terminal-safe rendering;
- deterministic sorting;
- explicit unavailable memory state;
- no single process failure may abort a snapshot;
- one-shot and JSON modes must not depend on terminal state.

## F. Security audit

- No shell injection was found in the existing command invocations because subprocess arguments are passed as lists rather than shell strings.
- No network access, telemetry, temporary files, elevated privileges, or destructive process actions are present.
- Terminal escape/control-character sanitization was missing and is corrected.
- Process names are treated as untrusted display data.
- The dependency footprint remains standard-library only.

## Classification summary

| Class | Findings |
|---|---|
| High-priority bug | Windows pagefile/swap semantics; terminal control-character injection |
| Medium-priority bug | Invalid counts; refresh validation; fabricated fallback totals; macOS/Linux metric semantics |
| Performance improvement | Native Windows process enumeration; monotonic scheduling; reduced paused work |
| Portability improvement | Native Windows APIs; explicit terminal fallback behavior |
| Maintainability improvement | Normalized process/memory helpers; deterministic sorting; targeted exception handling |
| Documentation improvement | Metric semantics, limitations, JSON schema, testing and troubleshooting |
| Optional future feature | Additional platform-native metrics, richer process metadata, configurable output themes |

## Sources used for verification

- Linux kernel `/proc/meminfo` documentation: `MemAvailable` is an estimate and multiple memory counters overlap.
- Linux `proc_pid_statm(5)`: the RSS field is documented as potentially inaccurate; accurate `smaps`/`smaps_rollup` data is much slower.
- Microsoft `MEMORYSTATUSEX`/`GlobalMemoryStatusEx`: documents physical memory, paging-file totals/availability, and commit-related semantics.
- Microsoft Tool Help / `Process32First`/`Process32Next`: documents native process enumeration.
- Microsoft `GetProcessMemoryInfo`: documents native working-set retrieval and access requirements.
- Apple `vm_stat` and `sysctlbyname`: documents macOS virtual-memory statistics and `hw.memsize`.

This audit deliberately distinguishes exact native counters from best-effort estimates; no unsupported metric is presented as exact.
