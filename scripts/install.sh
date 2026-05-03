#!/bin/bash
set -e

echo "=== dmgr Installation ==="

PREFIX="${PREFIX:-/usr}"
BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/dmgr"
POLKIT_DIR="$PREFIX/share/polkit-1/actions"
QUICKSHELL_DIR="${QUICKSHELL_DIR:-/usr/share/quickshell/modules/dmgr}"
SYSTEMD_USER_DIR="${SYSTEMD_USER_DIR:-/usr/lib/systemd/user}"
APPS_DIR="$PREFIX/share/applications"
ICONS_DIR="$PREFIX/share/icons/hicolor/scalable/apps"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/quickshell"

echo "Building Rust components..."
cargo build --release

echo "Installing binaries..."
install -Dm755 target/release/dmgr-daemon "$BIN_DIR/dmgr-daemon"
install -Dm755 target/release/dmgr-polkit-helper "$BIN_DIR/dmgr-polkit-helper"

echo "Installing resources..."
install -Dm644 resources/dmgr.desktop "$APPS_DIR/dmgr.desktop"
install -Dm644 resources/dmgr-daemon.desktop "$APPS_DIR/dmgr-daemon.desktop"
install -Dm644 resources/dmgr-daemon.service "$SYSTEMD_USER_DIR/dmgr-daemon.service"
install -Dm644 resources/org.dmgr.DeviceManager.policy "$POLKIT_DIR/org.dmgr.DeviceManager.policy"

echo "Installing QML module..."
mkdir -p "$SHARE_DIR/qml"
cp -r qml/* "$SHARE_DIR/qml/"

echo "Installing QuickShell module..."
mkdir -p "$QUICKSHELL_DIR"
cp -r quickshell/* "$QUICKSHELL_DIR/" 2>/dev/null || true
cp resources/quickshell/metadata.json "$QUICKSHELL_DIR/"
cp -r qml "$QUICKSHELL_DIR/"

echo "Installing icons..."
for icon in qml/icons/*.svg; do
    name=$(basename "$icon" .svg)
    install -Dm644 "$icon" "$ICONS_DIR/$name.svg"
done

echo ""
echo "=== Installation complete ==="
echo ""
echo "Start the daemon:"
echo "  systemctl --user enable --now dmgr-daemon"
echo ""
echo "Launch the standalone app:"
echo "  qml6 /usr/share/dmgr/qml/dmgr-standalone.qml"
echo ""
echo "Or use the CLI:"
echo "  pip install ./cli"
echo "  dmgr list"
