#!/usr/bin/env bash
# ==============================================================================
# ram-tui secure installer script
# Installs executable and shell completions into user directories
# ==============================================================================

set -euo pipefail

REPO="BlackFeather-git/ram-tui"
BRANCH="${RAM_INSTALL_BRANCH:-main}"
BIN_DIR="${HOME}/.local/bin"
TARGET="${BIN_DIR}/ram"
BASE_URL="https://raw.githubusercontent.com/${REPO}/${BRANCH}"

echo -e "\033[1;36m⚡ Installing ram-tui (branch: ${BRANCH})...\033[0m"

# 1. Ensure target binary directory exists
mkdir -p "${BIN_DIR}"

# 2. Safe atomic download via temporary file
TMP_BIN=$(mktemp)
cleanup() {
    rm -f "${TMP_BIN}"
}
trap cleanup EXIT INT TERM

echo "📦 Downloading ram executable..."
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${BASE_URL}/ram" -o "${TMP_BIN}"
elif command -v wget >/dev/null 2>&1; then
    wget -qO "${TMP_BIN}" "${BASE_URL}/ram"
else
    echo "❌ Error: curl or wget is required for installation." >&2
    exit 1
fi

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
    curl -fsSL "${BASE_URL}/completions/ram.bash" -o "${BASH_COMP_DIR}/ram" 2>/dev/null || true
fi

# Zsh
ZSH_COMP_DIR="${HOME}/.local/share/zsh/site-functions"
mkdir -p "${ZSH_COMP_DIR}" 2>/dev/null || true
if [ -d "${ZSH_COMP_DIR}" ]; then
    curl -fsSL "${BASE_URL}/completions/_ram" -o "${ZSH_COMP_DIR}/_ram" 2>/dev/null || true
fi

# Fish
FISH_COMP_DIR="${HOME}/.config/fish/completions"
mkdir -p "${FISH_COMP_DIR}" 2>/dev/null || true
if [ -d "${FISH_COMP_DIR}" ]; then
    curl -fsSL "${BASE_URL}/completions/ram.fish" -o "${FISH_COMP_DIR}/ram.fish" 2>/dev/null || true
fi

# 4. Non-destructive PATH check
case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *)
        echo ""
        echo -e "\033[1;33m⚠️  Note: ${BIN_DIR} is not in your current PATH.\033[0m"
        echo "   To make 'ram' available globally, add this to your ~/.bashrc or ~/.zshrc:"
        echo -e "   \033[1mexport PATH=\"\$HOME/.local/bin:\$PATH\"\033[0m"
        ;;
esac

echo ""
echo -e "\033[1;32m🚀 Installation complete! Run 'ram' to launch.\033[0m"
