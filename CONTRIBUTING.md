# Contributing to ram-tui

Thank you for your interest in contributing to `ram-tui`!

---

## 1. Design Principles & Invariants

* **Zero Unnecessary Dependencies**: Use lightweight, audited standard library or ecosystem primitives.
* **Sub-Millisecond Execution**: Memory collection and rendering loops must complete in <1.0ms.
* **100% Rust Memory Safety**: Avoid unnecessary `unsafe` blocks. FFI code in `collector_linux` must have strict bounds checks.
* **Zero Emojis**: Maintain a clean, professional, high-density terminal interface.
* **Cross-Platform Parity**: Features should maintain semantic parity across Linux, macOS, and Windows where possible.

---

## 2. Workspace Architecture

The project is structured as a modular Cargo workspace:

* **`core_render`**: UTF-8 terminal cell-width calculation, TrueColor ANSI interpolation, IEC unit formatting, sparkline generation, and differential frame buffering.
* **`collector_linux`**: Single-pass procfs `/proc/meminfo` parser, candidate-gated `/proc/<pid>/smaps_rollup` PSS/USS engine, Cgroups v2/v1 container detector, and macOS Mach / Windows PSAPI native subsystems.
* **`ui`**: 13 TrueColor theme palettes, responsive layout budgeting, interactive theme picker modal, and raw terminal management.
* **`cli`**: Binary targets (`ram` and `ram-tui`), CLI flag parsing via `clap`, JSON snapshot export, and interactive event loop.

---

## 3. Development Workflow

### Prerequisites
* Rust 1.70.0+ (stable toolchain)
* Linux, macOS, or Windows

### Building the Project
```bash
cargo build --workspace
```

### Running Tests
```bash
cargo test --workspace
```

### Running Linter
```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

### Running Benchmarks
```bash
cargo bench
```

---

## 4. Pull Request Guidelines

1. **Keep Commits Clean & Focused**: Use clear commit messages adhering to standard conventions.
2. **Include Unit Tests**: Any new telemetry collection or rendering feature must include unit tests.
3. **Preserve Documentation**: Update `CHANGELOG.md` and relevant markdown documents for any user-facing changes.
