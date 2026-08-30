# ⚡ RAM - Real-Time Dynamic Memory TUI

A high-precision, low-latency (100ms) dynamic terminal memory monitor designed for Linux with native ZRAM detection, smooth ANSI rendering, and process resident set aggregation.

---

## 🚀 Features

- **Exact Screenshot Aesthetics**: Clean typography, colored gradient usage bars, and mini process graphs.
- **100ms Live Refresh Rate**: Sub-millisecond direct `/proc` kernel parsing with zero flickering.
- **ZRAM Awareness**: Automatically detects whether swap is ZRAM (compressed in RAM) or physical disk.
- **Process Aggregation**: Groups multi-instance applications (e.g. `brave (12)`, `firefox (8)`, `gjs (3)`) with accurate combined Resident Set Size (RSS).
- **Interactive Controls**:
  - `q` / `Ctrl+C`: Quit
  - `Space` / `p`: Pause / Resume live stream
  - `+` / `-`: Increase / Decrease refresh speed (from 20ms to 2000ms)
  - `1` / `2`: Toggle between Grouped process view and Individual PID view
- **CLI Options**:
  - `ram`: Live interactive 100ms TUI monitor
  - `ram --once` / `ram -1`: Single-shot snapshot
  - `ram -r 50`: Custom refresh rate in milliseconds
  - `ram -n 12`: Display top 12 processes instead of 8

---

## 📁 Installation

The binary is located at `/home/raven/Projects/ram-tui/ram` and symlinked to `~/.local/bin/ram`.

You can launch it anytime from any terminal by typing:
```bash
ram
```
