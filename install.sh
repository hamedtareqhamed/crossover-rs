#!/bin/bash
set -e

REPO_URL="https://github.com/hamedtareqhamed/crossover-rs.git"
TARBALL_URL="https://github.com/hamedtareqhamed/crossover-rs/archive/refs/heads/main.tar.gz"
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

# Determine if running from within the repository or as a standalone script (e.g. via wget / curl)
SCRIPT_DIR=""
if [ -n "${BASH_SOURCE[0]}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
fi

CLEANUP_DIR=""
if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/Cargo.toml" ] && [ -d "$SCRIPT_DIR/assets" ]; then
    SRC_DIR="$SCRIPT_DIR"
elif [ -f "./Cargo.toml" ] && [ -d "./assets" ]; then
    SRC_DIR="$(pwd)"
else
    echo "📦 Downloading source repository from GitHub..."
    TMP_DIR=$(mktemp -d /tmp/crossover-rs-XXXXXX)
    CLEANUP_DIR="$TMP_DIR"
    trap 'if [ -n "$CLEANUP_DIR" ] && [ -d "$CLEANUP_DIR" ]; then rm -rf "$CLEANUP_DIR"; fi' EXIT INT TERM

    if command -v git >/dev/null 2>&1; then
        git clone --depth 1 "$REPO_URL" "$TMP_DIR"
    elif command -v curl >/dev/null 2>&1; then
        curl -fsSL "$TARBALL_URL" | tar -xz -C "$TMP_DIR" --strip-components=1
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$TARBALL_URL" | tar -xz -C "$TMP_DIR" --strip-components=1
    else
        echo "❌ Error: Neither git, curl, nor wget is installed."
        exit 1
    fi
    SRC_DIR="$TMP_DIR"
fi

cd "$SRC_DIR"

# 1. Install Binary
if [ -f "./target/release/$APP_NAME" ]; then
    echo "⚡ Found compiled release binary, installing..."
    cp "./target/release/$APP_NAME" "$BIN_DIR/$APP_NAME"
elif [ -f "./$APP_NAME" ]; then
    echo "⚡ Found local binary, installing..."
    cp "./$APP_NAME" "$BIN_DIR/$APP_NAME"
else
    if ! command -v cargo >/dev/null 2>&1; then
        echo "❌ Error: Rust & Cargo are required to compile CrossOver-rs."
        echo "👉 You can install Rust easily by running:"
        echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    echo "⚙️ Building release binary with Cargo (this may take a minute)..."
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
Comment=Ultra-lightweight Linux Crosshair Overlay with System Tray (Wayland & X11)
Exec=$BIN_DIR/$APP_NAME -d
Icon=crossover
Terminal=false
Type=Application
Categories=Game;Utility;
Keywords=crosshair;overlay;aim;fps;gaming;tray;
Actions=Toggle;Interactive;Quit;

[Desktop Action Toggle]
Name=Toggle Visibility
Exec=$BIN_DIR/$APP_NAME --toggle

[Desktop Action Interactive]
Name=Interactive Terminal Controller
Exec=sh -c "$BIN_DIR/$APP_NAME -i"

[Desktop Action Quit]
Name=Quit CrossOver
Exec=$BIN_DIR/$APP_NAME --quit
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
