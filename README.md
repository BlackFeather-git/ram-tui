# ram-tui

A lightweight, real-time terminal memory monitor for **Linux, macOS, and Windows** with native/best-effort memory breakdown and process grouping.

> **v0.3.2 — maintained hobby/open-source project**
>
> The project is intentionally dependency-free and conservative: no telemetry, network access, tracking, elevated privileges, or destructive system actions.

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

- Real-time memory monitoring.
- Linux `/proc` memory and process collection.
- Windows native memory/process APIs through `ctypes`.
- macOS `vm_stat`/`sysctl` memory collection and `ps` process fallback.
- Human-readable interactive terminal UI.
- Grouped processes or individual PIDs.
- Deterministic process sorting.
- Configurable refresh rate and process count.
- One-shot snapshot mode.
- Machine-readable JSON mode.
- Pause/resume without collecting new snapshots while paused.
- Monotonic refresh scheduling to reduce timing drift.
- Graceful handling of disappearing/inaccessible processes.
- Terminal control-character sanitization.
- Standard-library only.

## Supported platforms

| Platform | System memory | Process memory | Notes |
|---|---|---|---|
| Linux | `/proc/meminfo` | `/proc/<pid>/statm` | RSS is lightweight/best-effort; Linux documents `statm` RSS as potentially inaccurate. |
| macOS | `vm_stat`, `sysctl` | `ps` | Memory categories use macOS semantics and are not direct Linux equivalents. |
| Windows | `GlobalMemoryStatusEx` | Tool Help + `GetProcessMemoryInfo` | Some protected processes cannot be queried and are skipped. |

### Python

**Python 3.6+** is intentionally retained for compatibility with the project's original requirement.

No third-party Python packages are required.

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

### Windows

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

Invalid rates/counts are rejected instead of silently producing surprising output.

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

## JSON

`--json` always emits a single JSON document with no ANSI escape sequences, banners, progress messages, or terminal control codes.

Example shape:

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

### JSON semantics

- `memory.total`, `available`, `used`, and process `rss` are bytes.
- `commit_as` and `commit_limit` use the host platform's commit semantics.
- `cached` is a platform-specific best-effort cache/reclaimable estimate.
- `swap_used` is an actual swap-used value where the platform exposes one.
- On Windows, `swap_total` represents **pagefile capacity** while actual pagefile residency is not exposed by `GlobalMemoryStatusEx`; `swap_used` is therefore `0` and `swap_desc` explicitly says usage is unavailable.
- `valid=false` means the platform memory collector could not obtain a valid system-memory snapshot.

Consumers should treat platform-specific metrics as optional/best-effort rather than assuming Linux-equivalent semantics everywhere.

## Memory accounting notes

### Linux

Linux exposes many overlapping counters through `/proc/meminfo`. `MemAvailable` is an estimate of memory available to start applications without swapping, and it is preferred over simply using `MemFree`.

The displayed cached value combines page cache, buffers, and reclaimable slab as a practical display estimate. It is **not** intended to be an exact partition of total RAM.

Process RSS comes from `/proc/<pid>/statm`. Linux documents that its RSS value can be inaccurate because of kernel scalability optimizations; using `smaps`/`smaps_rollup` for every process would be considerably more expensive and would undermine the fast-refresh design.

### macOS

macOS memory accounting uses different categories from Linux. `vm_stat` provides high-level virtual-memory statistics, and the UI therefore describes its numbers as best-effort platform-native metrics rather than Linux-compatible definitions.

### Windows

Windows `GlobalMemoryStatusEx` provides physical-memory availability and paging-file/commit capacity. It does **not** directly expose actual pagefile residency. ram-tui therefore avoids manufacturing a fake "pagefile used" value.

Process working-set collection uses native Windows APIs. Protected or inaccessible processes may be omitted.

## Terminal and redirected output

Interactive mode uses terminal input only when both stdin and stdout are TTYs.

For scripts, pipes, CI, logs, or automation, prefer:

```bash
ram --once
ram --json
```

The application handles broken pipes and interrupted execution without attempting destructive cleanup.

If the terminal is very narrow, process names are truncated and the layout remains bounded.

If `NO_COLOR` is set or `TERM=dumb`, color is disabled.

## Troubleshooting

### Some processes are missing

This is expected when the operating system denies access to a process or the process exits between enumeration and inspection. ram-tui treats individual process failures as non-fatal.

### Numbers differ from another system monitor

Different tools use different definitions for available, cached, committed, compressed, shared, and resident memory. ram-tui deliberately uses platform-native sources instead of pretending those definitions are identical.

### Linux RSS values differ slightly

`/proc/<pid>/statm` is selected for speed. Exact `smaps`-derived RSS/PSS accounting is much more expensive.

### Windows pagefile usage is not shown

That is intentional. The standard `GlobalMemoryStatusEx` interface does not provide actual pagefile residency. The program reports pagefile capacity and labels the limitation instead of inventing a number.

## Development

Run syntax checks:

```bash
python -m py_compile ram
```

Run the standard-library test suite:

```bash
python -m unittest discover -s tests -v
```

Run a local smoke test:

```bash
python ram --version
python ram --help
python ram --once
python ram --json
```

Validate JSON:

```bash
python ram --json > snapshot.json
python -c "import json; json.load(open('snapshot.json')); print('valid JSON')"
```

The test suite uses mocked platform data for deterministic parsing/calculation tests and does not rely solely on the current machine's process table.

## Project structure

```text
ram-tui/
├── ram
├── README.md
├── AUDIT.md
└── tests/
    └── test_ram.py
```

The executable remains a single-file application to keep installation and contribution simple.

## Contributing

Keep changes focused and platform-conscious.

Before submitting a change:

1. Preserve the zero-external-dependency design unless a dependency is genuinely necessary.
2. Add deterministic tests for parsing, calculation, rendering, or CLI behavior.
3. Avoid changing existing flags/keybindings without documenting compatibility impact.
4. Do not add telemetry, networking, tracking, privilege escalation, or destructive actions.
5. Verify Linux behavior locally when possible and clearly identify platform behavior that could not be tested.
6. Update the README when user-visible behavior or platform limitations change.

## Attribution

Created and maintained by **BlackFeather**.

The project began as a hobby recreation of a terminal memory-monitoring interface after an exact implementation could not be found. It is intentionally published as open source so others can use, inspect, improve, and adapt it.

## License

MIT License.

See `LICENSE` if included in a distribution.
