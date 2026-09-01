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

---

## Release Key Custody & Verification

### Key Custody
* The RSA-2048 maintainer private signing key is stored in an encrypted offline storage medium held exclusively by maintainer `Raven <blackfeatheractual@proton.me>`.
* No private keys or unencrypted signing materials are stored on public CI servers or repository branches.

### Manual Verification of Release Assets
Users who wish to verify downloaded release artifacts manually can do so with standard command-line tools without any third-party Python packages:

1. **Verify SHA-256 Checksum**:
   ```bash
   sha256sum -c ram.sha256
   ```

2. **Verify Maintainer RSA-2048 Digital Signature (Zero Dependencies)**:
   ```bash
   python3 -c "
   import importlib.machinery, importlib.util, sys
   loader = importlib.machinery.SourceFileLoader('ram_mod', 'ram')
   spec = importlib.util.spec_from_loader('ram_mod', loader)
   m = importlib.util.module_from_spec(spec)
   loader.exec_module(m)
   with open('ram', 'rb') as f: data = f.read()
   with open('ram.sig', 'r') as f: sig = f.read().strip()
   valid = m.verify_release_signature(data, sig)
   print('Signature verification:', 'VALID (Authentic Release)' if valid else 'INVALID')
   sys.exit(0 if valid else 1)
   "
   ```

### Key Rotation & Revocation Procedure
* If a key rotation is scheduled, a transition release signed by both the old and new maintainer keys will be issued with a minimum 60-day deprecation window.
* In the event of a key compromise, immediate notification will be published on the GitHub Releases page, git tags will be signed with a hardware GPG key, and package manager maintainers (Homebrew, AUR, Scoop) will be requested to freeze automated pulls until an emergency patch release updates the embedded `RELEASE_PUBLIC_KEY_N`.

---

## Package Manager vs. In-Place Self-Updater Guidance

* **Standalone / User-Local Installs (`~/.local/bin/ram`, `%USERPROFILE%\.local\bin\ram.py`)**: The built-in self-updater (`ram --update`) is the recommended update path.
* **System / Package Manager Installs (`/usr/bin/ram`, Homebrew, Scoop, Arch AUR, Debian)**: `ram --update` automatically detects system-managed paths and fails closed to prevent desynchronizing the OS package database. Users on package-managed installations should update via their package manager (e.g. `paru -Syu ram-tui`, `brew upgrade ram-tui`, or `scoop update ram`). Use `--force` only if you explicitly wish to override this safety guard.
