# ram-tui

> *"Small enough to understand. Accurate enough to trust. Fast enough to leave running."*  
> *"I made this because I wanted it - and now anyone can depend on it."*

`ram-tui` is a lightweight, cross-platform terminal memory monitor written in Python for **Linux, macOS, and Windows**. It provides real-time, best-effort memory statistics using platform-native data sources, process resident-memory ranking, optional process grouping, configurable refresh rates, one-shot snapshots, and machine-readable JSON output.

> [!NOTE]
> Metrics may differ between Linux, macOS, and Windows because each operating system exposes memory information differently (e.g. Linux `MemAvailable` vs macOS inactive/speculative memory vs Windows commit/pagefile limits). `ram-tui` queries each platform's native subsystem for the most accurate local representation.

---

## What it shows

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

Process RSS is a resident-memory measurement, not a claim of unique physical memory ownership. Shared pages can therefore make the sum of process RSS values exceed the system's physical-memory usage.

## Features

- Real-time memory monitoring with zero external dependencies.
- Linux `/proc` memory and process collection.
- Windows native memory/process APIs through `ctypes` (`GlobalMemoryStatusEx` & Tool Help API).
- macOS `vm_stat`/`sysctl` memory collection and `ps` process ranking.
- Human-readable interactive terminal UI with colored gradient bars.
- Grouped processes (e.g. `brave (12)`) or individual PIDs.
- Deterministic process sorting.
- Configurable refresh rate (20ms – 2000ms) and process count.
- One-shot snapshot mode (`--once`) and clean JSON mode (`--json`).
- Pause/resume without collecting new snapshots while paused.
- Monotonic refresh scheduling to eliminate timing drift.
- Terminal control-character sanitization to prevent ANSI injection.
- Standard-library only (Python 3.6+).

## Supported platforms

| Platform | System memory | Process memory | Notes |
|---|---|---|---|
| Linux | `/proc/meminfo` | `/proc/<pid>/statm` | RSS is lightweight/best-effort; Linux documents `statm` RSS as potentially inaccurate. |
| macOS | `vm_stat`, `sysctl` | `ps` | Memory categories use macOS semantics and are not direct Linux equivalents. |
| Windows | `GlobalMemoryStatusEx` | Tool Help + `GetProcessMemoryInfo` | Some protected processes cannot be queried and are skipped. |

## Installation

### Linux & macOS

```bash
git clone https://github.com/BlackFeather-git/ram-tui.git
cd ram-tui

mkdir -p ~/.local/bin
cp ram ~/.local/bin/ram
chmod +x ~/.local/bin/ram
```

Make sure `~/.local/bin` is in your `PATH`.

System-wide:

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

The project uses the Windows API through Python's standard-library `ctypes`; it does not require PowerShell commands such as `tasklist`.

## Usage

Interactive mode:

```bash
ram
```

One snapshot:

```bash
ram --once
```

Individual PIDs:

```bash
ram --no-group
```

Faster refresh:

```bash
ram --rate 50
```

Show more processes:

```bash
ram --count 16
```

JSON snapshot:

```bash
ram --json
```

JSON plus individual processes:

```bash
ram --json --no-group --count 16
```

## CLI

```text
-r, --rate <ms>      Refresh interval in milliseconds (20–2000, default: 100)
-n, --count <N>      Number of top processes (1–10000, default: 8)
-1, --once           Output one snapshot and exit
--json               Output one JSON snapshot and exit
--no-group           Show individual process PIDs instead of grouping
-v, --version        Show version number
```

Invalid rates/counts are rejected with clear error messages.

## Keybindings

| Key | Action |
|---|---|
| `q` / `Ctrl+C` | Quit |
| `Space` / `p` | Pause / resume |
| `+` / `=` | Faster refresh, down to 20 ms |
| `-` / `_` | Slower refresh, up to 2000 ms |
| `1` | Group processes by executable name |
| `2` | Show individual PIDs |

While paused, the program stops collecting new snapshots. The displayed frame remains visible.

## JSON Mode

`--json` emits a clean, machine-readable JSON document with no ANSI escape sequences:

```json
{
  "timestamp": "2026-08-30T23:30:00+05:30",
  "hostname": "my-laptop",
  "os": "Linux",
  "version": "0.3.2",
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

## Running Tests

Run the built-in deterministic test suite:

```bash
python3 -m unittest tests/test_ram.py
```

## Requirements

- Python 3.6+ on Linux, macOS, or Windows
- Zero external dependencies (standard library only)

## License

[MIT](LICENSE)
