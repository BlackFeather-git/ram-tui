# ram-tui

> *"Small enough to understand. Accurate enough to trust. Fast enough to leave running."*  
> *"I made this because I wanted it and it is heavily vibe coded! If it is useful for you please feel free to use it however you want."*

A clean, real-time memory monitor for your terminal.

Most system monitors are either bloated full-screen dashboards or bare-bones CLI outputs without context. `ram-tui` gives you a fast, zero-dependency TUI with dynamic usage bars, ZRAM detection, and aggregated process rankings across Linux, macOS, and Windows.

Single file. Standard library only. No installation headaches.

---

## Preview

```text
RAM USAGE — my-laptop  Sun 21:47:21
[██████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 13.9%

Used    4.31 GB    Available  26.70 GB   Total   31.01 GB
Commit  9.18 GB / 46.51 GB  (20% of commit limit)
Cached  8.43 GB    (cache + reclaimable estimate)
Swap    1.55 MB / 31.01 GB   (zram swap)

TOP 8 PROCESSES BY RESIDENT SET
brave (12)             2.1 GB  ████████████████  6.7%
gnome-shell          657.2 MB  █████░░░░░░░░░░░  2.1%
code (6)             384.6 MB  ███░░░░░░░░░░░░░  1.2%

These 8 account for  3.79 GB (12% of installed RAM)
```

> **Note:** Process RSS measures resident memory pages, not strictly exclusive physical RAM. Shared libraries may cause the sum of process RSS values to differ from total system memory usage.

## Features

- **Zero dependencies**: Pure Python 3.6+ standard library.
- **Cross-platform**: Native `/proc` parsing on Linux, `sysctl`/`vm_stat` on macOS, and Win32 APIs via `ctypes` on Windows (`GetPerformanceInfo` & Tool Help API).
- **Accurate memory contracts**: Platform-native metrics with honest semantics (see [AUDIT.md](AUDIT.md)).
- **Real-time & smooth**: Monotonic deadline scheduler (default 100ms) with zero timing drift.
- **PID-reuse safe**: Starttime-keyed cache on Linux prevents stale process names on rapid PID reuse.
- **Process grouping**: Automatically groups multi-process apps (e.g. `brave (12)`) or displays individual PIDs with `1`/`2` hotkeys.
- **Sanitized rendering**: Strips terminal control codes and Unicode bidi overrides to prevent ANSI injection.
- **Automation ready**: Output one-time snapshots with `--once` or machine-readable JSON with `--json`.

## Supported Platforms

| Platform | System Memory Source | Process Inspection | Notes |
|---|---|---|---|
| **Linux** | `/proc/meminfo`, `/proc/swaps` | `/proc/<pid>/statm` | Native ZRAM detection & starttime-keyed PID cache. |
| **macOS** | `vm_stat`, `sysctl` | `ps` | Native page-size calculation & regex swap extraction. Commit is marked N/A. |
| **Windows** | `GlobalMemoryStatusEx`, `GetPerformanceInfo` | Tool Help + `GetProcessMemoryInfo` | Pure `ctypes` Win32 API calls (no `tasklist` subprocess overhead). |

## Installation

### Linux & macOS

```bash
git clone https://github.com/BlackFeather-git/ram-tui.git
cd ram-tui

mkdir -p ~/.local/bin
cp ram ~/.local/bin/ram
chmod +x ~/.local/bin/ram
```
*(Make sure `~/.local/bin` is in your `$PATH`)*

**System-wide:**
```bash
sudo cp ram /usr/local/bin/ram
sudo chmod +x /usr/local/bin/ram
```

### Windows (PowerShell / Command Prompt)

```powershell
git clone https://github.com/BlackFeather-git/ram-tui.git
cd ram-tui
python ram
```

## Usage

**Interactive TUI:**
```bash
ram
```

**Single Snapshot:**
```bash
ram --once
```

**Show Individual PIDs:**
```bash
ram --no-group
```

**Faster 50ms Refresh:**
```bash
ram --rate 50
```

**Show Top 16 Processes:**
```bash
ram --count 16
```

**JSON Output:**
```bash
ram --json
```

## Hotkeys

| Key | Action |
|---|---|
| `q` / `Ctrl+C` | Quit |
| `Space` / `p` | Pause / resume live updates |
| `+` / `=` | Increase refresh rate (down to 20 ms) |
| `-` / `_` | Decrease refresh rate (up to 2000 ms) |
| `1` | Group processes by executable name |
| `2` | Show individual process PIDs |

## JSON Mode

`--json` outputs a clean, machine-parseable JSON payload:

```json
{
  "timestamp": "2026-08-31T00:15:00+05:30",
  "hostname": "my-laptop",
  "os": "Linux",
  "version": "0.4.3",
  "memory": {
    "total": 33554432000,
    "available": 28689039360,
    "used": 4865392640,
    "commit_as": 9634304000,
    "commit_limit": 50331648000,
    "cached": 9069694976,
    "swap_used": 1638400,
    "swap_total": 33554432000,
    "swap_desc": "zram swap",
    "valid": true
  },
  "top_processes": [
    {
      "name": "brave",
      "rss": 2254857830,
      "count": 12
    }
  ]
}
```

## Tests

Run the built-in deterministic test suite:

```bash
python3 -m unittest tests/test_ram.py
```

## License

[MIT](LICENSE)
