# Security Policy

## Supported Versions

| Version | Supported |
|:---|:---|
| 0.6.x | Yes |
| < 0.6.0 | No |

## Reporting a Vulnerability

If you discover a security vulnerability or integrity flaw in `ram-tui` (including the kernel telemetry collectors, terminal engine, or cryptographic self-updater), please report it responsibly:

* **Maintainer Contact**: `Raven <blackfeatheractual@proton.me>`
* **PGP / Encryption**: Available upon request via protonmail.

Please include:
1. Description of the vulnerability and attack vector.
2. Minimal reproduction steps or proof-of-concept payload.
3. Affected operating systems and Python versions.

Security reports receive an initial response within 24 hours. Validated issues are remediated and released with high priority.

---

## Threat Model & Security Architecture

`ram-tui` implements a strict zero-dependency, defense-in-depth model:

### 1. Cryptographic Root of Trust (RSA-2048 PKCS#1 v1.5)
* Release binaries are mathematically verified against an embedded maintainer RSA-2048 public key before any execution or replacement.
* Signature verification is implemented in pure standard-library arithmetic with strict representative bounds (`0 < s < n`) and constant-time digest comparison (`hmac.compare_digest`).

### 2. Dual Integrity Layer (SHA-256 + RSA Signature)
* Every download payload is verified against both its published SHA-256 digest (`ram.sha256`) and the maintainer digital signature (`ram.sig`).

### 3. AST Semantic Source Validation
* Downloaded updates are inspected using Python standard library `ast.parse()` to verify top-level `__version__` declarations and `if __name__ == "__main__":` entry blocks, preventing comment or docstring spoofing.

### 4. Privilege & TOCTOU Protection
* The self-updater verifies that the binary and its parent directory are genuine physical paths and not swapped for symlinks prior to atomic replacement.
* Updates executed as root (`geteuid() == 0`) fail closed with `PermissionError` if situated in insecure world-writable directories (`st_mode & 0o002`).

### 5. Process Lock & PID Reuse Protection
* Background update checks employ inter-process locking with Linux process starttime keying (field 22 of `/proc/[pid]/stat`) to prevent stale lock eviction races caused by PID wraparound.
