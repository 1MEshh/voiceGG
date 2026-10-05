# VoiceGG

VoiceGG is a high-performance, open-source audio routing, mixer, and DSP effect suite designed specifically for Arch-based Linux distributions running PipeWire and modern Wayland compositors (such as Hyprland and Sway) or desktop environments.

Inspired by advanced gaming audio suites, VoiceGG gives you full control over per-application audio routing, parametric multi-band equalizers, intelligent noise cancellation, and a unified mixer interface—built entirely with native performance and privacy in mind.

---

## Features

- **PipeWire Native**: Zero hacky virtual drivers; operates directly within the native PipeWire graph with minimal latency.
- **5-Channel Mixing Architecture**: Dedicated channels for **Game**, **Chat**, **Media**, **Aux**, and **Mic**, plus a **Master** bus and **Streamer Mix**.
- **Intuitive App Routing**: Drag-and-drop running audio streams between channels, with automatic persistence across reboots.
- **Parametric 10-Band EQ**: Full biquad equalizer with interactive curve adjustments, filter types (peak, shelving, pass filters), and instant presets.
- **AI Noise Cancellation**: Local, offline noise suppression on microphone input with 0–100% wet/dry attenuation.
- **Game Detection & Auto-Profiles**: Automatically applies tailored audio curves when supported games (CS2, Apex Legends, Overwatch, etc.) are launched.
- **ChatMix & Quick Faders**: Hardware-free software ChatMix balancing game and voice communication seamlessly.
- **Custom Theming**: Per-channel accent colors, dark/light themes, gradient VU meters, and pywal / desktop color scheme integration.
- **Headless Daemon Architecture**: Runs as a lightweight `systemd --user` service with an optional high-polish UI and a rich CLI for Hyprland keybindings.

---

## Architecture Overview

```
voicegg/
├── crates/
│   ├── voicegg-core/       # Core types, channels, presets, serialization
│   ├── voicegg-dsp/        # Pure DSP algorithms (biquad EQ, limiter, gate, noise cancellation)
│   ├── voicegg-pw/         # PipeWire graph management (virtual sinks, links, metadata)
│   ├── voicegg-ipc/        # Unix domain socket protocol and IPC client/server
│   ├── voicegg-daemon/     # Background service managing the graph and DSP processing
│   └── voicegg-cli/        # Command-line control interface for scripting and keybindings
└── app/                    # Modern desktop UI (Tauri 2 + Svelte 5)
```

---

## Getting Started

### Requirements
- Arch Linux or Arch-based distribution (EndeavourOS, CachyOS, Manjaro, etc.)
- PipeWire with WirePlumber (0.5+)
- Rust (1.80+) & Cargo

### Building from Source

```bash
git clone https://github.com/your-username/voicegg.git
cd voicegg
cargo build --release
```

---

## Hyprland Integration Example

Bind ChatMix and Mic mute directly to hotkeys in `hyprland.conf`:

```ini
# Toggle mic mute
bind = SUPER, F9, exec, voicegg ctl mute mic toggle

# Adjust ChatMix balance
bind = SUPER, F10, exec, voicegg ctl chatmix -10
bind = SUPER, F11, exec, voicegg ctl chatmix +10
```

---

## License

VoiceGG is released under the **GNU General Public License v3.0 (GPL-3.0)**. See [LICENSE](LICENSE) for details.
