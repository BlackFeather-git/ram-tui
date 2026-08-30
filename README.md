# ram-tui

A lightweight, real-time dynamic terminal memory monitor for Linux with native ZRAM detection and grouped process breakdown.

```text
RAM USAGE — my-laptop  Sun 21:47:21
[██████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 13.9%

Used    4.31 GB    Available  26.70 GB   Total   31.01 GB
Commit  9.18 GB / 46.51 GB  (20% of commit limit)
Cached  8.43 GB    (reclaimable on demand)
Swap    1.55 MB / 31.01 GB   (zram - compressed, costs CPU not disk)

TOP 8 PROCESSES BY RESIDENT SET
brave (12)             2.1 GB  ████████████████   6.7%
gnome-shell          657.2 MB  █████░░░░░░░░░░░   2.1%
code (6)             384.6 MB  ███░░░░░░░░░░░░░   1.2%
Xwayland             197.7 MB  █░░░░░░░░░░░░░░░   0.6%
kitty                160.9 MB  █░░░░░░░░░░░░░░░   0.5%

These 8 account for  3.79 GB (12% of installed RAM)
```

## Installation

### User install (no root needed)
```bash
git clone https://github.com/BlackFeather-git/ram-tui.git
cd ram-tui
mkdir -p ~/.local/bin
cp ram ~/.local/bin/
```
*(Make sure `~/.local/bin` is in your `$PATH`)*

### System-wide install
```bash
sudo cp ram /usr/local/bin/
```

## Usage

Simply run:
```bash
ram
```

### Keybindings
| Key | Action |
| --- | --- |
| `q` / `Ctrl+C` | Quit |
| `Space` / `p` | Pause / resume live view |
| `+` / `-` | Increase / decrease refresh rate (20ms – 2000ms) |
| `1` / `2` | Toggle grouped processes vs individual PIDs |

### Options
```text
-r, --rate <ms>    Refresh rate in milliseconds (default: 100)
-n, --count <N>    Number of top processes to display (default: 8)
-1, --once         Output a single snapshot and exit
--no-group         Show individual process PIDs instead of grouping
```

## Requirements
- Linux with `/proc` filesystem
- Python 3.6+ (zero external dependencies)

## License
MIT
