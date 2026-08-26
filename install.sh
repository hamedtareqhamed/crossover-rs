#!/bin/bash
set -e

APP_NAME="crossover"
BIN_DIR="$HOME/.local/bin"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_SVG_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
ICON_PNG_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"

echo "🎯 Installing CrossOver-rs (GPL-3.0)..."

# Ensure target directories exist
mkdir -p "$BIN_DIR"
mkdir -p "$DESKTOP_DIR"
mkdir -p "$ICON_SVG_DIR"
mkdir -p "$ICON_PNG_DIR"

# 1. Install Binary
if [ -f "./target/release/$APP_NAME" ]; then
    cp "./target/release/$APP_NAME" "$BIN_DIR/$APP_NAME"
elif [ -f "./$APP_NAME" ]; then
    cp "./$APP_NAME" "$BIN_DIR/$APP_NAME"
else
    echo "⚙️ Building release binary with Cargo..."
    cargo build --release
    cp "./target/release/$APP_NAME" "$BIN_DIR/$APP_NAME"
fi
chmod +x "$BIN_DIR/$APP_NAME"

# 2. Install Icons
if [ -f "./assets/crossover.svg" ]; then
    cp "./assets/crossover.svg" "$ICON_SVG_DIR/crossover.svg"
fi
if [ -f "./assets/crossover.png" ]; then
    cp "./assets/crossover.png" "$ICON_PNG_DIR/crossover.png"
fi

# 3. Install Desktop Entry
cat << DESKTOPEOF > "$DESKTOP_DIR/crossover.desktop"
[Desktop Entry]
Name=CrossOver
GenericName=Crosshair Overlay
Comment=Ultra-lightweight Linux Crosshair Overlay (Wayland & X11)
Exec=$BIN_DIR/$APP_NAME
Icon=crossover
Terminal=true
Type=Application
Categories=Game;Utility;
Keywords=crosshair;overlay;aim;fps;gaming;
DESKTOPEOF

chmod +x "$DESKTOP_DIR/crossover.desktop"

# Check if PATH contains ~/.local/bin
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️ Note: $HOME/.local/bin is not in your PATH."
    echo "Add this to your ~/.bashrc or ~/.zshrc:"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
fi

echo "✨ CrossOver-rs installed successfully!"
echo "🚀 Run 'crossover' in your terminal or launch it from your Application Menu."
