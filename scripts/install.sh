#!/usr/bin/env bash
set -e

# VoiceGG Rootless Installer for Arch Linux & Wayland / Hyprland

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Building VoiceGG frontend..."
if [ -d "$ROOT_DIR/app" ]; then
    (cd "$ROOT_DIR/app" && npm run build)
fi

echo "==> Building VoiceGG release binaries..."
cd "$ROOT_DIR"
cargo build --release

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
SYSTEMD_DIR="$HOME/.config/systemd/user"

echo "==> Creating user directories..."
mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR" "$SYSTEMD_DIR"

echo "==> Installing binaries to $BIN_DIR..."
install -m 755 "$ROOT_DIR/target/release/voicegg-daemon" "$BIN_DIR/voicegg-daemon"
install -m 755 "$ROOT_DIR/target/release/voicegg" "$BIN_DIR/voicegg"
if [ -f "$ROOT_DIR/target/release/voicegg-gui" ]; then
    install -m 755 "$ROOT_DIR/target/release/voicegg-gui" "$BIN_DIR/voicegg-gui"
fi
install -m 755 "$ROOT_DIR/scripts/voicegg-launcher" "$BIN_DIR/voicegg-launcher"

echo "==> Installing desktop entry and icon..."
install -m 644 "$ROOT_DIR/packaging/voicegg.desktop" "$APP_DIR/voicegg.desktop"
install -m 644 "$ROOT_DIR/packaging/icons/voicegg.svg" "$ICON_DIR/voicegg.svg"

echo "==> Installing systemd user unit..."
install -m 644 "$ROOT_DIR/packaging/systemd/voicegg-daemon.service" "$SYSTEMD_DIR/voicegg-daemon.service"

echo "==> Updating desktop and icon caches..."
if command -v update-desktop-database > /dev/null 2>&1; then
    update-desktop-database "$APP_DIR" || true
fi
if command -v gtk-update-icon-cache > /dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "==> Reloading systemd user daemon..."
systemctl --user daemon-reload || true

echo ""
echo "============================================================"
echo "  VoiceGG successfully installed!"
echo "  You can now launch VoiceGG directly from Rofi, Wofi, or Hyprland."
echo "============================================================"
