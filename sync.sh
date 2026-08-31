#!/usr/bin/env bash
# ==============================================================================
# ram-tui Automated Dual-Sync Engine (GitHub + Google Drive)
# Usage: ./sync.sh ["commit message"]
# ==============================================================================
set -e

REPO_DIR="/home/raven/Projects/ram-tui"
cd "$REPO_DIR"

# 1. Extract current version & branch
VERSION=$(grep -o '__version__ = "[^"]*"' ram | cut -d'"' -f2 || echo "dev")
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "beta")
COMMIT_MSG="${1:-"chore(release): sync $VERSION updates"}"

echo -e "\033[1;36m[1/3] 🧪 Running test suite...\033[0m"
python3 -m unittest discover tests -q

echo -e "\033[1;35m[2/3] 🐙 Pushing to GitHub (branch: $BRANCH)...\033[0m"
git add .
if ! git diff-index --quiet HEAD --; then
    git commit -m "$COMMIT_MSG"
fi
git pull --rebase origin "$BRANCH" 2>/dev/null || true
git push origin "$BRANCH"

echo -e "\033[1;32m[3/3] ☁️ Syncing to Google Drive (gdrive:ram-tui)...\033[0m"
rclone copy "$REPO_DIR" gdrive:ram-tui \
    --exclude ".git/**" \
    --exclude "__pycache__/**" \
    --exclude "*.pyc" \
    --fast-list

echo -e "\033[1;32m✨ SUCCESS! Dual-sync complete:\033[0m"
echo -e "   🐙 GitHub: https://github.com/BlackFeather-git/ram-tui/tree/$BRANCH"
echo -e "   ☁️ GDrive: https://drive.google.com/open?id=11fLmfBDeusQJUjMFH77yqX9FmvwEyEUj"
