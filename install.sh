#!/bin/bash
# install.sh - Install claude-deck binary

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="claude-deck"

echo "Building claude-deck in release mode..."
cargo build --release

echo "Installing to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
else
    sudo cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
fi

echo "Installed $BINARY_NAME to $INSTALL_DIR"

# Verify installation
if command -v "$BINARY_NAME" &> /dev/null; then
    echo "✓ Installation successful!"
    echo ""
    "$BINARY_NAME" --version
    echo ""
    echo "Usage:"
    echo "  claude-deck           # Run with device, spawns claude"
    echo "  claude-deck --attach  # Device only, keystrokes to focused window"
    echo "  claude-deck --status  # Check if device is connected"
    echo ""
    echo "To start on login:"
    echo "  claude-deck --install-autostart"
else
    echo "✗ Installation may have failed - binary not found in PATH"
    exit 1
fi
