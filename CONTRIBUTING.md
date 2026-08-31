# Contributing to ram-tui

Thank you for your interest in contributing to `ram-tui`.

`ram-tui` is a minimalist, ultra-low-latency terminal memory monitor engineered for accuracy, speed, and safety.

---

## Core Guarantees & Non-Negotiables

Before submitting a pull request, ensure your proposed changes adhere strictly to the project's core invariants:

1. **Zero External Dependencies**:
   * The runtime must remain 100% standard library (`argparse`, `ctypes`, `subprocess`, `ast`, `hmac`, `unicodedata`).
   * No `pip` requirements, third-party packages, or wheel dependencies are accepted.

2. **Cross-Platform Parity**:
   * Features must function deterministically across **Linux**, **macOS**, and **Windows**.
   * Linux telemetry uses `/proc`. macOS telemetry uses `sysctl`/`vm_stat`. Windows telemetry uses Win32 PSAPI ctypes.

3. **Sub-Millisecond Performance**:
   * The frame rendering cycle must maintain sub-millisecond execution (<1ms/frame) with <0.6% CPU footprint.
   * Process parsing and width caching must remain $O(N)$ with LRU fast paths.

4. **Terminal & 2D Geometry Safety**:
   * All rendered text lines must strictly clamp within terminal bounds (`cols` and `rows`).
   * No line wrapping, screen stutter, or scrollback pollution.

5. **Cryptographic Integrity**:
   * All update logic must maintain fail-closed RSA-2048 and SHA-256 validation.

---

## Development Setup

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/BlackFeather-git/ram-tui.git
   cd ram-tui
   ```

2. **Run the Test Suite**:
   ```bash
   python3 -m unittest discover tests
   ```

3. **Validate Bytecode Compilation**:
   ```bash
   python3 -m compileall ram
   ```

4. **Smoke-Test Local Invocations**:
   ```bash
   python3 ram --version
   python3 ram --once
   python3 ram --compact --once
   python3 ram --mini --once
   python3 ram --tiny --once
   python3 ram --json --once
   ```

---

## Pull Request Guidelines

1. **Branch Off `test`**:
   * Feature branches and pull requests should target the `test` branch (active development area).

2. **Add Unit Tests**:
   * All bug fixes and feature additions must include corresponding unit tests in `tests/test_ram.py` or `tests/test_update_flow.py`.

3. **Maintain Zero-Emoji Policy**:
   * All code, terminal outputs, error messages, and documentation must have zero emojis.
