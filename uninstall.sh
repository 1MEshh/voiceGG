#!/usr/bin/env bash
# VoiceGG Rootless Safe Uninstaller
# Completely removes VoiceGG and restores default PipeWire audio state.
# Can be run via:
#   curl -fsSL https://raw.githubusercontent.com/1MEshh/voiceGG/main/uninstall.sh | bash

set -euo pipefail

if [ "$(id -u)" -eq 0 ]; then
    echo "ERROR: Do NOT run this uninstaller as root or with sudo!" >&2
    exit 1
fi

echo "============================================================"
echo "                VoiceGG Uninstaller                         "
echo "============================================================"
echo ""

# 1. Restore PipeWire routing to defaults
echo "==> Restoring default system audio routing..."
if command -v voicegg > /dev/null 2>&1; then
    voicegg panic > /dev/null 2>&1 || true
fi

# 2. Stop and disable systemd user daemon
echo "==> Stopping VoiceGG daemon and background services..."
systemctl --user stop voicegg-daemon.service > /dev/null 2>&1 || true
systemctl --user disable voicegg-daemon.service > /dev/null 2>&1 || true
pkill -x voicegg-daemon > /dev/null 2>&1 || true
pkill -x voicegg-gui > /dev/null 2>&1 || true

# 3. Remove installed files according to manifest or safe fallback list
MANIFEST="$HOME/.local/share/voicegg/installed-files"
FILES_TO_REMOVE=()

if [ -f "$MANIFEST" ]; then
    while IFS= read -r line || [ -n "$line" ]; do
        [ -z "$line" ] && continue
        FILES_TO_REMOVE+=("$line")
    done < "$MANIFEST"
else
    # Fallback to known default install locations
    FILES_TO_REMOVE=(
        "$HOME/.local/bin/voicegg-daemon"
        "$HOME/.local/bin/voicegg"
        "$HOME/.local/bin/voicegg-gui"
        "$HOME/.local/bin/voicegg-launcher"
        "$HOME/.local/bin/voicegg-uninstall"
        "$HOME/.local/share/applications/voicegg.desktop"
        "$HOME/.local/share/icons/hicolor/scalable/apps/voicegg.svg"
        "$HOME/.config/systemd/user/voicegg-daemon.service"
    )
fi

echo "==> Removing installed files..."
for file in "${FILES_TO_REMOVE[@]}"; do
    # Strict safety guard: never remove anything outside $HOME/.local or $HOME/.config
    case "$file" in
        "$HOME/.local/"*|"$HOME/.config/systemd/user/"*)
            if [ -f "$file" ] || [ -L "$file" ]; then
                rm -f "$file"
                echo "    Removed: $file"
            fi
            ;;
        *)
            echo "    SKIPPED unsafe path: $file" >&2
            ;;
    esac
done

# Remove manifest directory and runtime socket
rm -rf "$HOME/.local/share/voicegg" 2>/dev/null || true
rm -rf "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/voicegg" 2>/dev/null || true

# 4. Handle configuration and custom presets
CONFIG_DIR="$HOME/.config/voicegg"
PURGE=false

if [ "${1:-}" = "--purge" ]; then
    PURGE=true
elif [ -d "$CONFIG_DIR" ]; then
    echo ""
    read -r -p "Delete saved presets and configuration in $CONFIG_DIR? [y/N] " response </dev/tty || response="n"
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        PURGE=true
    fi
fi

if [ "$PURGE" = true ] && [ -d "$CONFIG_DIR" ]; then
    echo "==> Purging configuration in $CONFIG_DIR..."
    rm -rf "$CONFIG_DIR"
else
    if [ -d "$CONFIG_DIR" ]; then
        echo "==> Preserved configuration directory ($CONFIG_DIR)."
    fi
fi

# 5. Update desktop caches
echo "==> Refreshing desktop and icon databases..."
if command -v update-desktop-database > /dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" || true
fi
if command -v gtk-update-icon-cache > /dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi
systemctl --user daemon-reload || true

echo ""
echo "============================================================"
echo "  VoiceGG has been completely and safely uninstalled."
echo "  All default PipeWire audio routing has been restored."
echo "============================================================"
