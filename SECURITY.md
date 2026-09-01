# Security Policy

## Supported Versions

| Version | Supported |
|:---|:---|
| `1.0.x` (Rust Native) | Yes |
| `< 1.0.0` (Python Legacy) | Security Fixes Only |

---

## 1. Threat Model & Security Invariants

`ram-tui` is designed to be a completely self-contained system telemetry monitor with zero attack surface:

* **Zero Network Telemetry**: `ram-tui` never transmits metrics, telemetry, or user system data over the network.
* **Least-Privilege Procurement**: Requires no `root` / `sudo` privileges for standard memory monitoring or user process telemetry.
* **Privileged Directory & TOCTOU Protection**: In-place updates verify physical file identity, target parent directory permissions, and reject symlink substitutions.
* **Strict Input & Output Sanitization**: Process names, hostnames, and CLI arguments are sanitized against ANSI escape sequences, ASCII control codes, and Unicode directional overrides (Bidi spoofing).
* **PID Reuse Mitigation**: Process cache keys include kernel starttime (`/proc/<pid>/stat` field 22) to prevent PID recycling race conditions.
* **Signal Safety**: Process termination hotkey (`x`/`K`) implements an explicit confirmation gate (`[y/N]`) before dispatching `SIGTERM`.

---

## 2. Reporting a Vulnerability

If you discover a potential security issue in `ram-tui`, please report it privately:

* **Email**: `Raven (BlackFeather) <blackfeatheractual@proton.me>`
* **PGP Key**: Available upon request or via maintainer profile.

Please include:
1. Description of the vulnerability.
2. Steps to reproduce or proof-of-concept.
3. Affected operating system and terminal environment.

We aim to acknowledge reports within 24 hours and coordinate responsible disclosure.
