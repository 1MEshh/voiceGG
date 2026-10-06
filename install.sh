#!/usr/bin/env bash
# VoiceGG Rootless Installer for Arch Linux & Wayland / Hyprland
# Repo: https://github.com/1MEshh/voiceGG
# Can be run directly from cloned repo or piped via curl:
#   curl -fsSL https://raw.githubusercontent.com/1MEshh/voiceGG/main/install.sh | bash

set -euo pipefail

# 1. Safety check: never run as root
if [ "$(id -u)" -eq 0 ]; then
    echo "ERROR: Do NOT run this installer as root or with sudo!" >&2
    echo "VoiceGG is designed to run entirely within your user session." >&2
    exit 1
fi

echo "============================================================"
echo "          VoiceGG Installer (Arch Linux / Wayland)          "
echo "============================================================"
echo ""

# 2. Check for Arch Linux environment & PipeWire
if ! command -v pacman > /dev/null 2>&1; then
    echo "WARNING: pacman was not detected on this system." >&2
    echo "VoiceGG is officially developed and tuned for Arch Linux and Arch-based distros." >&2
fi

if ! command -v pipewire > /dev/null 2>&1; then
    echo "ERROR: PipeWire is not installed. VoiceGG requires a running PipeWire audio server." >&2
    exit 1
fi

# 3. Check build and runtime dependencies
NEEDED_PKGS=()
check_pkg() {
    local cmd="$1"
    local pkg="$2"
    if ! command -v "$cmd" > /dev/null 2>&1; then
        NEEDED_PKGS+=("$pkg")
    fi
}

check_pkg git git
check_pkg cargo rust
check_pkg npm nodejs
check_pkg pkg-config pkgconf

# Check for webkit2gtk-4.1 (Tauri 2 requirement)
if command -v pacman > /dev/null 2>&1; then
    if ! pacman -Qi webkit2gtk-4.1 > /dev/null 2>&1; then
        NEEDED_PKGS+=("webkit2gtk-4.1")
    fi
fi

if [ ${#NEEDED_PKGS[@]} -gt 0 ]; then
    echo "==> Missing required build packages: ${NEEDED_PKGS[*]}"
    echo ""
    read -r -p "Install missing packages now using 'sudo pacman -S --needed'? [y/N] " response </dev/tty || response="n"
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        sudo pacman -S --needed "${NEEDED_PKGS[@]}"
    else
        echo "Aborted. Please install dependencies manually and run the installer again:"
        echo "  sudo pacman -S --needed ${NEEDED_PKGS[*]}"
        exit 1
    fi
fi

# 4. Determine repository source (local or clone into temporary directory)
TEMP_DIR=""
cleanup() {
    if [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
}
trap cleanup EXIT

if [ -f "./Cargo.toml" ] && [ -d "./crates" ] && [ -d "./app" ]; then
    ROOT_DIR="$(pwd)"
    echo "==> Building from local workspace: $ROOT_DIR"
else
    TEMP_DIR="$(mktemp -d -t voicegg-build-XXXXXX)"
    echo "==> Cloning VoiceGG repository to temporary workspace..."
    REPO_URL="${VOICEGG_REPO_URL:-https://github.com/1MEshh/voiceGG.git}"
    REPO_REF="${VOICEGG_REF:-main}"
    git clone --depth 1 --branch "$REPO_REF" "$REPO_URL" "$TEMP_DIR"
    ROOT_DIR="$TEMP_DIR"
fi

cd "$ROOT_DIR"

# 5. Build release binaries
echo "==> Building VoiceGG release binaries (daemon & CLI)..."
cargo build --release -p voicegg-cli -p voicegg-daemon

echo "==> Building VoiceGG GUI release binary (with embedded frontend)..."
(
    cd "$ROOT_DIR/app"
    if [ ! -d "node_modules" ]; then
        npm ci --prefer-offline 2>/dev/null || npm install
    fi
    npx tauri build --no-bundle
)

# 6. Installation directories
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
SYSTEMD_DIR="$HOME/.config/systemd/user"
MANIFEST_DIR="$HOME/.local/share/voicegg"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR" "$SYSTEMD_DIR" "$MANIFEST_DIR"

echo "==> Installing binaries to $BIN_DIR..."
install -m 755 "$ROOT_DIR/target/release/voicegg-daemon" "$BIN_DIR/voicegg-daemon"
install -m 755 "$ROOT_DIR/target/release/voicegg" "$BIN_DIR/voicegg"
if [ -f "$ROOT_DIR/target/release/voicegg-gui" ]; then
    install -m 755 "$ROOT_DIR/target/release/voicegg-gui" "$BIN_DIR/voicegg-gui"
fi
install -m 755 "$ROOT_DIR/scripts/voicegg-launcher" "$BIN_DIR/voicegg-launcher"

# Install desktop entry, icon, and systemd unit
echo "==> Installing desktop entry and assets..."
install -m 644 "$ROOT_DIR/packaging/voicegg.desktop" "$APP_DIR/voicegg.desktop"
install -m 644 "$ROOT_DIR/packaging/icons/voicegg.svg" "$ICON_DIR/voicegg.svg"
install -m 644 "$ROOT_DIR/packaging/systemd/voicegg-daemon.service" "$SYSTEMD_DIR/voicegg-daemon.service"

# Install uninstall script for user convenience
install -m 755 "$ROOT_DIR/scripts/uninstall.sh" "$BIN_DIR/voicegg-uninstall"

# 7. Write installation manifest for safe uninstallation
cat <<EOF > "$MANIFEST_DIR/installed-files"
$BIN_DIR/voicegg-daemon
$BIN_DIR/voicegg
$BIN_DIR/voicegg-gui
$BIN_DIR/voicegg-launcher
$BIN_DIR/voicegg-uninstall
$APP_DIR/voicegg.desktop
$ICON_DIR/voicegg.svg
$SYSTEMD_DIR/voicegg-daemon.service
EOF

# 8. Update desktop and icon caches
echo "==> Updating desktop and icon caches..."
if command -v update-desktop-database > /dev/null 2>&1; then
    update-desktop-database "$APP_DIR" || true
fi
if command -v gtk-update-icon-cache > /dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

# 9. Reload systemd user daemon
systemctl --user daemon-reload || true

echo ""
echo "============================================================"
echo "  VoiceGG successfully installed!"
echo ""
echo "  How to use:"
echo "    - Launch GUI:          voicegg-launcher (or search in Rofi/Wofi)"
echo "    - Command Line Status: voicegg status"
echo "    - Restore Audio/Panic: voicegg panic"
echo "    - Safe Uninstall:      voicegg-uninstall"
echo "============================================================"
