#!/bin/bash
# ram-tui installer script
# Installs ram into ~/.local/bin

set -e

REPO="BlackFeather-git/ram-tui"
INSTALL_DIR="${HOME}/.local/bin"
EXECUTABLE="${INSTALL_DIR}/ram"

echo "⚡ Installing ram-tui..."

# Ensure target directory exists
mkdir -p "${INSTALL_DIR}"

# Download executable
curl -sSL "https://raw.githubusercontent.com/${REPO}/main/ram" -o "${EXECUTABLE}"
chmod +x "${EXECUTABLE}"

echo "✅ Successfully installed ram-tui to ${EXECUTABLE}"

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    echo ""
    echo "⚠️  Note: ${INSTALL_DIR} is not in your PATH."
    echo "   Add it to your shell configuration file (~/.bashrc or ~/.zshrc):"
    echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo ""
echo "🚀 Run 'ram' to start monitoring!"
