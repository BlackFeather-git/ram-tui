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

echo -e "\033[1;36m⚡ Installing ram-tui (branch: ${BRANCH})...\033[0m"

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
        echo "❌ Error: curl or wget is required for installation." >&2
        return 1
    fi
}

# Check if existing installation should be overwritten
if [ -f "${TARGET}" ] && [ "$FORCE" = false ]; then
    if [ -t 0 ]; then
        echo -e "\033[1;33m⚠️  '${TARGET}' already exists.\033[0m"
        read -rp "   Overwrite existing binary? [y/N] " response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            echo "Installation cancelled."
            exit 0
        fi
    fi
fi

# 1. Ensure target binary directory exists
mkdir -p "${BIN_DIR}"

# 2. Safe atomic download via temporary file
TMP_BIN=$(mktemp)
cleanup() {
    rm -f "${TMP_BIN}"
}
trap cleanup EXIT INT TERM

echo "📦 Downloading ram executable..."
fetch_file "${BASE_URL}/ram" "${TMP_BIN}"

# Verify executable is non-empty
if [ ! -s "${TMP_BIN}" ]; then
    echo "❌ Error: Downloaded file is empty. Please check your network connection." >&2
    exit 1
fi

# Install executable with 0755 permissions
install -m 0755 "${TMP_BIN}" "${TARGET}"
echo -e "\033[1;32m✅ Installed executable to: ${TARGET}\033[0m"

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
        echo -e "\033[1;33m⚠️  Note: ${BIN_DIR} is not in your current PATH.\033[0m"
        echo "   To make 'ram' globally executable, add ${BIN_DIR} to your shell profile:"
        echo "   • Bash:  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
        echo "   • Zsh:   echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
        echo "   • Fish:  fish_add_path ~/.local/bin"
        echo "   • POSIX: echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.profile"
        ;;
esac

echo ""
echo -e "\033[1;32m🚀 Installation complete! Run 'ram' to launch.\033[0m"
