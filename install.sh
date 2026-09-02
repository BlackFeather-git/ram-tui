#!/usr/bin/env bash
# ram-tui v1.0.2 — Universal Installer Script
set -euo pipefail

REPO="BlackFeather-git/ram-tui"
INSTALL_DIR="${HOME}/.local/bin"

mkdir -p "${INSTALL_DIR}"

echo "==> Welcome to RAM-TUI v1.0.2!"
echo "==> Notice: RAM-TUI has officially transitioned from Python to a native Rust binary."
echo "==> Installing to ${INSTALL_DIR}..."

if command -v cargo >/dev/null 2>&1; then
    echo "Building native binary via Cargo..."
    cargo build --release -p cli
    cp -f target/release/ram "${INSTALL_DIR}/ram"
    cp -f target/release/ram-tui "${INSTALL_DIR}/ram-tui"
    chmod +x "${INSTALL_DIR}/ram" "${INSTALL_DIR}/ram-tui"
    echo "Successfully installed 'ram' and 'ram-tui' to ${INSTALL_DIR}"
    exit 0
fi

echo "Cargo not detected. Downloading precompiled release binary from GitHub..."
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${ARCH}" in
    x86_64|amd64) TARGET_ARCH="x86_64" ;;
    aarch64|arm64) TARGET_ARCH="aarch64" ;;
    *) echo "Unsupported architecture: ${ARCH}"; exit 1 ;;
esac

ASSET_URL="https://github.com/${REPO}/releases/latest/download/ram-tui-${OS}-${TARGET_ARCH}"
SHA_URL="${ASSET_URL}.sha256"

if ! curl -fsSL "${ASSET_URL}" -o "${INSTALL_DIR}/ram-tui"; then
    echo "Error: Failed to download binary asset from ${ASSET_URL}"
    echo "Please check https://github.com/${REPO}/releases/latest for available release assets."
    exit 1
fi

# Cryptographic SHA-256 integrity verification
if curl -fsSL "${SHA_URL}" -o "${INSTALL_DIR}/ram-tui.sha256" 2>/dev/null; then
    EXPECTED_SHA="$(awk '{print $1}' "${INSTALL_DIR}/ram-tui.sha256")"
    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL_SHA="$(sha256sum "${INSTALL_DIR}/ram-tui" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL_SHA="$(shasum -a 256 "${INSTALL_DIR}/ram-tui" | awk '{print $1}')"
    else
        ACTUAL_SHA=""
    fi
    if [ -n "${ACTUAL_SHA}" ] && [ -n "${EXPECTED_SHA}" ]; then
        if [ "${ACTUAL_SHA}" != "${EXPECTED_SHA}" ]; then
            echo "Error: Cryptographic SHA-256 checksum mismatch!"
            echo "Expected: ${EXPECTED_SHA}"
            echo "Actual:   ${ACTUAL_SHA}"
            rm -f "${INSTALL_DIR}/ram-tui" "${INSTALL_DIR}/ram-tui.sha256"
            exit 1
        fi
        echo "==> Cryptographic integrity verified: SHA-256 (${ACTUAL_SHA:0:16}...)"
    fi
    rm -f "${INSTALL_DIR}/ram-tui.sha256"
fi

chmod +x "${INSTALL_DIR}/ram-tui"
ln -sf "${INSTALL_DIR}/ram-tui" "${INSTALL_DIR}/ram"

echo "Installation complete! Run 'ram-tui' or 'ram' to launch."
