#!/bin/bash
set -e

echo "🗑️ Uninstalling CrossOver-rs..."

pkill -f "crossover" 2>/dev/null || true
rm -f "$HOME/.local/bin/crossover"
rm -f "$HOME/.local/share/applications/crossover.desktop"
rm -f "$HOME/.local/share/icons/hicolor/scalable/apps/crossover.svg"

echo "✅ CrossOver-rs has been completely uninstalled."
