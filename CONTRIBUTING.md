# Contributing to ram-tui

Thank you for your interest in contributing to `ram-tui`! 🚀

Our core philosophy:
> *"Small enough to understand. Accurate enough to trust. Fast enough to leave running."*

---

## Guiding Principles

1. **Standard Library Only:** Zero external dependencies (no `psutil`, `rich`, `curses`, etc.).
2. **Cross-Platform Integrity:** Linux, macOS, and Windows support must be preserved.
3. **Mathematical Honesty:** If a platform doesn't support a metric (e.g. Commit limit on macOS), display `N/A` rather than guessing or fabricating numbers.
4. **Backward Compatibility:** CLI flags, hotkeys, and JSON output schema are public API contracts.

---

## Development Setup

Clone the repository and switch to the `beta` branch:

```bash
git clone https://github.com/BlackFeather-git/ram-tui.git
cd ram-tui
git checkout beta
```

Run the deterministic test suite:

```bash
python3 -m unittest tests/test_ram.py
```

Test CLI flags:

```bash
./ram --version
./ram --once
./ram --compact --theme catppuccin
./ram --json
```

---

## Pull Request Guidelines

1. Make sure all unit tests pass before submitting a PR.
2. If adding a feature, include unit test coverage in `tests/test_ram.py`.
3. Keep changes focused and minimal—avoid large architectural rewrites without prior discussion.
