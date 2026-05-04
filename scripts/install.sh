#!/bin/bash
# dmgr v2 install script
# Installs the GUI binary, polkit helper, polkit policy, and desktop entry.
# Run from the repository root: sudo ./scripts/install.sh
set -e

PREFIX="${PREFIX:-/usr}"
BIN_DIR="$PREFIX/bin"
POLKIT_DIR="$PREFIX/share/polkit-1/actions"
APPS_DIR="$PREFIX/share/applications"

echo "=== dmgr v2 — Installation ==="
echo "Prefix: $PREFIX"
echo ""

# Build release binaries
echo "[1/3] Building release binaries..."
cargo build --release -p dmgr-gui -p dmgr-polkit-helper
echo "      Done."

# Install binaries
echo "[2/3] Installing binaries to $BIN_DIR ..."
install -Dm755 target/release/dmgr              "$BIN_DIR/dmgr"
install -Dm755 target/release/dmgr-polkit-helper "$BIN_DIR/dmgr-polkit-helper"
echo "      dmgr            -> $BIN_DIR/dmgr"
echo "      dmgr-polkit-helper -> $BIN_DIR/dmgr-polkit-helper"

# Install resources
echo "[3/3] Installing resources..."
install -Dm644 resources/org.dmgr.DeviceManager.policy \
    "$POLKIT_DIR/org.dmgr.DeviceManager.policy"
install -Dm644 resources/dmgr.desktop \
    "$APPS_DIR/dmgr.desktop"
echo "      polkit policy   -> $POLKIT_DIR/org.dmgr.DeviceManager.policy"
echo "      desktop entry   -> $APPS_DIR/dmgr.desktop"

echo ""
echo "=== Installation complete ==="
echo ""
echo "Launch the device manager:"
echo "  dmgr"
echo ""
echo "For privilege escalation (bind/unbind drivers, write sysfs),"
echo "pkexec and the polkit policy are already installed."
