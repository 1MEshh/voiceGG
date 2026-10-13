# Linux Desktop & Window Manager Integration Guide

VoiceGG is engineered for seamless, native integration with modern Linux desktop environments and Wayland compositors—specifically **Hyprland**, **Sway**, **River**, **KDE Plasma**, and **GNOME**.

This guide provides tested, copy-paste configurations for **Waybar**, **Hyprland hotkeys**, **SwayOSD on-screen display**, and **Rofi / Wofi interactive launchers**.

---

## 1. Waybar Integration

VoiceGG includes native JSON output support tailored for Waybar status bars.

### 1.1 Complete Waybar Module (`~/.config/waybar/config.jsonc`)

Add the `custom/voicegg` module into your modules list:

```jsonc
{
  "modules-right": [
    "custom/voicegg",
    "pulseaudio",
    "network",
    "clock"
  ],

  "custom/voicegg": {
    "format": "{}",
    "exec": "$HOME/.local/bin/waybar-voicegg.sh",
    "return-type": "json",
    "interval": 2,
    "on-click": "voicegg-launcher",
    "on-click-right": "voicegg ctl mute mic toggle",
    "on-click-middle": "voicegg ctl mute master toggle",
    "on-scroll-up": "voicegg ctl chatmix +5",
    "on-scroll-down": "voicegg ctl chatmix -5",
    "tooltip": true
  }
}
```

### 1.2 Dedicated ChatMix Slider Module (Optional)
If you prefer a distinct ChatMix status widget beside your volume slider:

```jsonc
  "custom/voicegg-chatmix": {
    "format": "🎮 {text} 💬",
    "exec": "voicegg status --chatmix-formatted",
    "interval": 1,
    "on-scroll-up": "voicegg ctl chatmix +5",
    "on-scroll-down": "voicegg ctl chatmix -5",
    "on-click-middle": "voicegg ctl chatmix 0",
    "tooltip": true
  }
```

### 1.3 Waybar CSS Styling (`~/.config/waybar/style.css`)

```css
/* VoiceGG Status Indicator */
#custom-voicegg {
    padding: 0 10px;
    margin: 3px 4px;
    background: #11151F;
    border: 1px solid #1F2637;
    border-radius: 6px;
    color: #F8FAFC;
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    font-weight: 600;
    transition: all 0.2s ease-in-out;
}

/* Hover state */
#custom-voicegg:hover {
    background: #161B26;
    border-color: #FF5200;
}

/* When Mic is Muted */
#custom-voicegg.mic-muted {
    background: #3B1219;
    border-color: #EF4444;
    color: #EF4444;
}

/* When Mic Monitoring (Sidetone) is Active */
#custom-voicegg.monitoring {
    border-color: #F59E0B;
    color: #F59E0B;
}

/* Daemon Offline */
#custom-voicegg.offline {
    background: #0B0E14;
    border-color: #1F2637;
    color: #64748B;
}
```

---

## 2. Hyprland Configuration (`~/.config/hypr/hyprland.conf`)

### 2.1 Window Rules
Ensure the VoiceGG control center opens centered and floating above games when invoked:

```ini
# VoiceGG GUI Window Rules
windowrulev2 = float, class:^(voicegg)$
windowrulev2 = size 1120 740, class:^(voicegg)$
windowrulev2 = center, class:^(voicegg)$
windowrulev2 = pin, class:^(voicegg)$, title:^(VoiceGG Pin)$
```

### 2.2 Global Hotkeys & Gaming Shortcuts
Bind essential audio actions directly to your keyboard:

```ini
# Toggle Microphone Mute (with instant audio feedback)
bind = SUPER, F9, exec, voicegg ctl mute mic toggle

# Adjust ChatMix balance (More Game vs More Discord)
bind = SUPER, F10, exec, voicegg ctl chatmix -10
bind = SUPER, F11, exec, voicegg ctl chatmix +10
bind = SUPER CTRL, F10, exec, voicegg ctl chatmix 0

# Toggle Live Mic Monitoring (Sidetone)
bind = SUPER, F8, exec, voicegg ctl monitor mic toggle

# Master Volume Controls
bind = SUPER, F12, exec, voicegg ctl volume master +5
bind = SUPER SHIFT, F12, exec, voicegg ctl volume master -5

# Emergency Audio Panic Reset (Instantly restore physical audio routing)
bind = SUPER CTRL, ESCAPE, exec, voicegg panic
```

---

## 3. Sway / i3 Configuration (`~/.config/sway/config`)

```ini
# VoiceGG Window Rules
for_window [app_id="voicegg"] floating enable, resize set 1120 px 740 px, move position center

# Keybindings
bindsym $mod+F8 exec voicegg ctl monitor mic toggle
bindsym $mod+F9 exec voicegg ctl mute mic toggle
bindsym $mod+F10 exec voicegg ctl chatmix -10
bindsym $mod+F11 exec voicegg ctl chatmix +10
bindsym $mod+Ctrl+Escape exec voicegg panic
```

---

## 4. SwayOSD / Desktop Notifications

If you use `swayosd` or standard notification daemons (`dunst`, `mako`, `swaync`), you can display slick on-screen overlays whenever ChatMix or Mute changes:

### Script: `~/.local/bin/voicegg-osd.sh`
```bash
#!/usr/bin/env bash
ACTION="$1"
VALUE="$2"

case "$ACTION" in
    chatmix)
        voicegg ctl chatmix "$VALUE"
        STATUS=$(voicegg status --chatmix-formatted)
        command -v notify-send >/dev/null && notify-send -h string:x-canonical-private-synchronous:voicegg-osd -i audio-volume-medium "ChatMix Balance" "$STATUS"
        ;;
    mute-mic)
        voicegg ctl mute mic toggle
        MUTED=$(voicegg status | jq -r '.channels.mic.muted')
        if [ "$MUTED" = "true" ]; then
            command -v notify-send >/dev/null && notify-send -u critical -h string:x-canonical-private-synchronous:voicegg-mic -i microphone-disabled "Microphone" "MUTED"
        else
            command -v notify-send >/dev/null && notify-send -u normal -h string:x-canonical-private-synchronous:voicegg-mic -i microphone "Microphone" "ACTIVE"
        fi
        ;;
esac
```

---

## 5. Rofi / Wofi Game Preset Selector

Create `~/.local/bin/rofi-voicegg-presets` to switch EQ profiles on the fly with a fuzzy finder:

```bash
#!/usr/bin/env bash
PRESETS="Counter-Strike 2\nApex Legends\nValorant\nRainbow Six Siege\nEscape from Tarkov\nOverwatch 2\nFlat / Neutral\nWarm Broadcast (Mic)"

CHOICE=$(echo -e "$PRESETS" | rofi -dmenu -p "VoiceGG Preset:" -theme-str 'window {width: 400px;}')

case "$CHOICE" in
    "Counter-Strike 2")     voicegg preset game cs2 ;;
    "Apex Legends")         voicegg preset game apex ;;
    "Valorant")             voicegg preset game valorant ;;
    "Rainbow Six Siege")    voicegg preset game r6 ;;
    "Escape from Tarkov")   voicegg preset game tarkov ;;
    "Overwatch 2")          voicegg preset game overwatch ;;
    "Flat / Neutral")       voicegg preset game flat ;;
    "Warm Broadcast (Mic)") voicegg preset mic broadcast ;;
esac
```

Bind this script to `SUPER, P` in your compositor config for instantaneous audio switching between matches!
