#!/bin/bash
set -e

echo "=== dmgr Uninstallation ==="

PREFIX="${PREFIX:-/usr}"

echo "Removing binaries..."
rm -f "$PREFIX/bin/dmgr-daemon"
rm -f "$PREFIX/bin/dmgr-polkit-helper"

echo "Removing resources..."
rm -f "$PREFIX/share/applications/dmgr.desktop"
rm -f "$PREFIX/share/applications/dmgr-daemon.desktop"
rm -f "$PREFIX/lib/systemd/user/dmgr-daemon.service" 2>/dev/null || true
rm -f "/usr/lib/systemd/user/dmgr-daemon.service" 2>/dev/null || true
rm -f "$PREFIX/share/polkit-1/actions/org.dmgr.DeviceManager.policy"

echo "Removing QML and QuickShell modules..."
rm -rf "$PREFIX/share/dmgr"
rm -rf "/usr/share/quickshell/modules/dmgr" 2>/dev/null || true
rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/quickshell" 2>/dev/null || true

echo "Removing icons..."
for icon in usb pci audio input block gpu network dmgr; do
    rm -f "$PREFIX/share/icons/hicolor/scalable/apps/$icon.svg"
done

echo ""
echo "=== Uninstallation complete ==="
echo ""
echo "Stop and disable the daemon first:"
echo "  systemctl --user disable --now dmgr-daemon"
