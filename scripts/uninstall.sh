#!/usr/bin/env bash
set -e

# VoiceGG Rootless Uninstaller

echo "==> Restoring system audio routing..."
if command -v voicegg > /dev/null 2>&1; then
    voicegg panic > /dev/null 2>&1 || true
fi

echo "==> Stopping VoiceGG daemon..."
systemctl --user stop voicegg-daemon.service > /dev/null 2>&1 || true
systemctl --user disable voicegg-daemon.service > /dev/null 2>&1 || true
pkill -f voicegg-daemon > /dev/null 2>&1 || true

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
SYSTEMD_DIR="$HOME/.config/systemd/user"

echo "==> Removing installed files..."
rm -f "$BIN_DIR/voicegg-daemon"
rm -f "$BIN_DIR/voicegg"
rm -f "$BIN_DIR/voicegg-gui"
rm -f "$BIN_DIR/voicegg-launcher"
rm -f "$APP_DIR/voicegg.desktop"
rm -f "$ICON_DIR/voicegg.svg"
rm -f "$SYSTEMD_DIR/voicegg-daemon.service"
rm -rf "$XDG_RUNTIME_DIR/voicegg" 2>/dev/null || true

if [ "$1" == "--purge" ]; then
    echo "==> Purging configuration files..."
    rm -rf "$HOME/.config/voicegg"
fi

echo "==> Updating desktop database..."
if command -v update-desktop-database > /dev/null 2>&1; then
    update-desktop-database "$APP_DIR" || true
fi
systemctl --user daemon-reload || true

echo ""
echo "============================================================"
echo "  VoiceGG successfully uninstalled."
echo "  System audio routing has been restored to default."
echo "============================================================"
