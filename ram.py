#!/usr/bin/env python3
"""
ram-tui v1.0.0-rc.1 — Polyglot launcher & migration bridge.

Executes the native high-performance Rust engine directly, or coordinates
local compilation / installation.
"""

import os
import shutil
import subprocess
import sys

__version__ = "1.0.0-rc.1"


def find_native_binary():
    """Locate the compiled native Rust binary across common search paths."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    candidates = [
        os.path.join(script_dir, "target", "release", "ram-tui"),
        os.path.join(script_dir, "target", "release", "ram"),
        os.path.expanduser("~/.local/bin/ram-tui"),
        os.path.expanduser("~/.local/bin/ram"),
        os.path.expanduser("~/.cargo/bin/ram-tui"),
        os.path.expanduser("~/.cargo/bin/ram"),
        "/usr/local/bin/ram-tui",
        "/usr/local/bin/ram",
        "/usr/bin/ram-tui",
        "/usr/bin/ram",
    ]
    for path in candidates:
        if os.path.isfile(path) and os.access(path, os.X_OK):
            return path
    return None


def main():
    binary = find_native_binary()
    if binary:
        try:
            os.execv(binary, [binary] + sys.argv[1:])
        except Exception:
            subprocess.run([binary] + sys.argv[1:])
            sys.exit(0)

    # If binary not built yet, build via cargo
    script_dir = os.path.dirname(os.path.abspath(__file__))
    cargo_toml = os.path.join(script_dir, "Cargo.toml")
    if os.path.isfile(cargo_toml) and shutil.which("cargo"):
        sys.stderr.write("Compiling native ram-tui binary via Cargo...\n")
        res = subprocess.run(
            ["cargo", "build", "--release", "-p", "cli"],
            cwd=script_dir,
        )
        if res.returncode == 0:
            new_bin = os.path.join(script_dir, "target", "release", "ram-tui")
            if os.path.isfile(new_bin):
                os.execv(new_bin, [new_bin] + sys.argv[1:])

    sys.stderr.write(
        "ram-tui v1.0.0-rc.1: native binary not found. Run 'cargo build --release' or './install.sh'.\n"
    )
    sys.exit(1)


if __name__ == "__main__":
    main()
