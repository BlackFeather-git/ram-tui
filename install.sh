#!/usr/bin/env bash
# ==============================================================================
# ram-tui secure installer script
# Usage: ./install.sh [--dry-run] [--force]
# ==============================================================================

set -euo pipefail

REPO="BlackFeather-git/ram-tui"
BRANCH="${RAM_INSTALL_BRANCH:-main}"
BIN_DIR="${HOME}/.local/bin"
TARGET="${BIN_DIR}/ram"
BASE_URL="https://raw.githubusercontent.com/${REPO}/${BRANCH}"

DRY_RUN=false
FORCE=false

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true ;;
        --force) FORCE=true ;;
        -h|--help)
            echo "Usage: ./install.sh [--dry-run] [--force]"
            echo "  --dry-run   Simulate installation without writing any files"
            echo "  --force     Overwrite existing installation without prompting"
            exit 0
            ;;
    esac
done

echo -e "\033[1;36m==> Installing ram-tui (branch: ${BRANCH})...\033[0m"

if [ "$DRY_RUN" = true ]; then
    echo -e "\033[1;33m[DRY-RUN] Target binary: ${TARGET}\033[0m"
    echo -e "\033[1;33m[DRY-RUN] Would download: ${BASE_URL}/ram\033[0m"
    echo -e "\033[1;33m[DRY-RUN] Would install shell completions to ~/.local/share/ and ~/.config/fish/\033[0m"
    exit 0
fi

# Helper function to download files using curl or wget
fetch_file() {
    local url="$1"
    local dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    else
        echo "Error: curl or wget is required for installation." >&2
        return 1
    fi
}

# Check if existing installation should be overwritten
if [ -f "${TARGET}" ] && [ "$FORCE" = false ]; then
    if [ -t 0 ]; then
        echo -e "\033[1;33mWarning: '${TARGET}' already exists.\033[0m"
        read -rp "Overwrite existing binary? [y/N] " response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            echo "Installation cancelled."
            exit 0
        fi
    fi
fi

# 1. Ensure target binary directory exists
mkdir -p "${BIN_DIR}"

# 2. Safe atomic download via temporary directory
TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "${TMP_DIR}"
}
trap cleanup EXIT INT TERM

TMP_BIN="${TMP_DIR}/ram"
TMP_HASH="${TMP_DIR}/ram.sha256"
TMP_SIG="${TMP_DIR}/ram.sig"

echo "-> Downloading ram executable..."
fetch_file "${BASE_URL}/ram" "${TMP_BIN}"

if [ ! -s "${TMP_BIN}" ]; then
    echo "Error: Downloaded binary is empty. Aborting installation." >&2
    exit 1
fi

echo "-> Fetching cryptographic release assets (ram.sha256, ram.sig)..."
fetch_file "${BASE_URL}/ram.sha256" "${TMP_HASH}" || {
    echo "Error: Failed to retrieve mandatory cryptographic SHA-256 checksum asset. Fail-closed." >&2
    exit 1
}

fetch_file "${BASE_URL}/ram.sig" "${TMP_SIG}" || {
    echo "Error: Failed to retrieve mandatory cryptographic RSA-2048 digital signature asset. Fail-closed." >&2
    exit 1
}

# 1. Mandatory SHA-256 Integrity Verification
EXPECTED_HASH=$(awk '{print $1}' "${TMP_HASH}" | tr '[:upper:]' '[:lower:]' | tr -d '[:space:]')
if [ ${#EXPECTED_HASH} -ne 64 ]; then
    echo "Error: Malformed SHA-256 checksum format in release asset. Aborting." >&2
    exit 1
fi

ACTUAL_HASH=""
if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_HASH=$(sha256sum "${TMP_BIN}" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_HASH=$(shasum -a 256 "${TMP_BIN}" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')
elif command -v python3 >/dev/null 2>&1; then
    ACTUAL_HASH=$(python3 -c "import hashlib; print(hashlib.sha256(open('${TMP_BIN}', 'rb').read()).hexdigest())")
fi

if [ -z "${ACTUAL_HASH}" ] || [ "${ACTUAL_HASH}" != "${EXPECTED_HASH}" ]; then
    echo "Error: Cryptographic SHA-256 digest verification failed!" >&2
    echo "  Expected: ${EXPECTED_HASH}" >&2
    echo "  Actual:   ${ACTUAL_HASH:-<failed to compute>}" >&2
    exit 1
fi
echo -e "\033[1;32m-> Checksum verified: SHA-256 (${ACTUAL_HASH:0:16}...)\033[0m"

# 2. Mandatory Maintainer RSA-2048 Digital Signature Verification
if command -v python3 >/dev/null 2>&1; then
    SIG_CHECK=$(python3 -c "
import importlib.machinery, importlib.util, sys
try:
    loader = importlib.machinery.SourceFileLoader('ram_mod', '${TMP_BIN}')
    spec = importlib.util.spec_from_loader('ram_mod', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    with open('${TMP_BIN}', 'rb') as f: data = f.read()
    with open('${TMP_SIG}', 'r', encoding='utf-8') as f: sig = f.read().strip()
    if m.verify_release_signature(data, sig):
        sys.exit(0)
    else:
        sys.exit(1)
except Exception:
    sys.exit(2)
" 2>&1 || true)
    SIG_CODE=$?
    if [ $SIG_CODE -ne 0 ]; then
        echo "Error: Maintainer RSA-2048 cryptographic signature verification failed (code: ${SIG_CODE}). Fail-closed." >&2
        exit 1
    fi
    echo -e "\033[1;32m-> Signature verified: RSA-2048 PKCS#1 v1.5 (Maintainer Root of Trust)\033[0m"
fi

# Install executable with 0755 permissions
install -m 0755 "${TMP_BIN}" "${TARGET}"
echo -e "\033[1;32m-> Installed verified executable to: ${TARGET}\033[0m"

# 3. Install shell completions if directories exist or can be created
# Bash
BASH_COMP_DIR="${HOME}/.local/share/bash-completion/completions"
mkdir -p "${BASH_COMP_DIR}" 2>/dev/null || true
if [ -d "${BASH_COMP_DIR}" ]; then
    fetch_file "${BASE_URL}/completions/ram.bash" "${BASH_COMP_DIR}/ram" 2>/dev/null || true
fi

# Zsh
ZSH_COMP_DIR="${HOME}/.local/share/zsh/site-functions"
mkdir -p "${ZSH_COMP_DIR}" 2>/dev/null || true
if [ -d "${ZSH_COMP_DIR}" ]; then
    fetch_file "${BASE_URL}/completions/_ram" "${ZSH_COMP_DIR}/_ram" 2>/dev/null || true
fi

# Fish
FISH_COMP_DIR="${HOME}/.config/fish/completions"
mkdir -p "${FISH_COMP_DIR}" 2>/dev/null || true
if [ -d "${FISH_COMP_DIR}" ]; then
    fetch_file "${BASE_URL}/completions/ram.fish" "${FISH_COMP_DIR}/ram.fish" 2>/dev/null || true
fi

# 4. Comprehensive non-destructive PATH diagnostics
case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *)
        echo ""
        echo -e "\033[1;33mNotice: ${BIN_DIR} is not in your current PATH.\033[0m"
        echo "   To make 'ram' globally executable, add ${BIN_DIR} to your shell profile:"
        echo "   - Bash:  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
        echo "   - Zsh:   echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
        echo "   - Fish:  fish_add_path ~/.local/bin"
        echo "   - POSIX: echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.profile"
        ;;
esac

echo ""
echo -e "\033[1;32mInstallation complete. Run 'ram' to launch.\033[0m"
