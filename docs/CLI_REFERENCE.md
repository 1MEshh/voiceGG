# VoiceGG Command-Line Interface (CLI) Reference Manual

The `voicegg` command-line utility provides complete scriptable control over the VoiceGG audio daemon, volume levels, ChatMix balance, dynamic stream routing, and emergency recovery.

---

## 1. Global Syntax & Commands

```bash
voicegg [COMMAND]
```

### Available Top-Level Commands
| Command | Description |
|:---|:---|
| `voicegg status` | Prints real-time audio channels, ChatMix balance, active streams, and game detection status. |
| `voicegg ctl` | Performs state mutations (volume, mute, ChatMix, routing, mic monitoring, presets). |
| `voicegg panic` | Immediately restores physical hardware audio routing, unloading all virtual modules. |

---

## 2. Command Reference

### 2.1 `voicegg status`
Queries the background daemon via Unix domain socket and displays current status.

#### Syntax
```bash
voicegg status [--json]
```

#### Human-Readable Output Example:
```text
=== VoiceGG Status ===
ChatMix Balance: -20
Mic Monitoring: ON (60%)
Channels:
  - Master  : 100% [ACTIVE]
  - Game    :  85% [ACTIVE]
  - Chat    :  75% [ACTIVE]
  - Media   :  60% [ACTIVE]
  - Aux     : 100% [ACTIVE]
  - Mic     : 100% [ACTIVE]
Active Streams:
  - [74] Counter-Strike 2 (cs2) -> Game
  - [82] Discord (discord) -> Chat
  - [91] Spotify (spotify) -> Media
Active Game: Counter-Strike 2
```

#### Machine-Readable (JSON) Output Example:
```bash
voicegg status --json
```

---

### 2.2 `voicegg ctl volume`
Sets the volume of any VoiceGG virtual channel.

#### Syntax
```bash
voicegg ctl volume <channel> <percent>
```

#### Arguments
- `<channel>`: One of `master`, `game`, `chat`, `media`, `aux`, `mic`.
- `<percent>`: Integer between `0` and `150` (supports software amplification up to 150%).

#### Examples
```bash
voicegg ctl volume game 80
voicegg ctl volume chat 100
voicegg ctl volume master 50
```

---

### 2.3 `voicegg ctl mute`
Toggles, mutes, or unmutes an audio channel.

#### Syntax
```bash
voicegg ctl mute <channel> [state]
```

#### Arguments
- `<channel>`: Target channel (`master`, `game`, `chat`, `media`, `aux`, `mic`).
- `[state]` (optional, default: `toggle`):
  - `toggle`: Invert current mute state.
  - `on` / `true` / `1`: Force mute.
  - `off` / `false` / `0`: Force unmute.

#### Examples
```bash
voicegg ctl mute mic toggle
voicegg ctl mute master on
voicegg ctl mute chat off
```

---

### 2.4 `voicegg ctl chatmix`
Adjusts the balance fader between Game and Voice Chat audio.

#### Syntax
```bash
voicegg ctl chatmix <value>
```

#### Arguments
- `<value>`: Integer from `-100` (100% Game, 0% Chat) to `+100` (0% Game, 100% Chat). `0` represents equal balance.

#### Examples
```bash
# Prioritize game audio during a clutch round
voicegg ctl chatmix -30

# Reset balance to center
voicegg ctl chatmix 0

# Prioritize Discord comms
voicegg ctl chatmix 40
```

---

### 2.5 `voicegg ctl route`
Reroutes an active application process binary to a target virtual channel and persists the rule in `~/.config/voicegg/config.toml`.

#### Syntax
```bash
voicegg ctl route <binary_name> <channel>
```

#### Examples
```bash
voicegg ctl route cs2 game
voicegg ctl route discord chat
voicegg ctl route spotify media
voicegg ctl route firefox media
```

---

### 2.6 `voicegg ctl monitor`
Controls Live Microphone Monitoring (hardware sidetone loopback).

#### Syntax
```bash
voicegg ctl monitor [state] [--volume <0-100>]
```

#### Examples
```bash
# Toggle sidetone on/off
voicegg ctl monitor toggle

# Enable sidetone at 65% volume
voicegg ctl monitor on --volume 65

# Turn off sidetone
voicegg ctl monitor off
```

---

### 2.7 `voicegg ctl preset` & `cycle-preset`
Applies an EQ profile or cycles to the next preset on a channel.

#### Syntax
```bash
voicegg ctl preset <channel> <preset_id>
voicegg ctl cycle-preset [channel]
```

#### Preset IDs
- Game: `game_cs2`, `game_apex`, `game_valorant`, `game_r6`, `game_tarkov`, `game_overwatch`, `flat`
- Mic: `mic_broadcast`, `mic_clarity`, `mic_clean`

#### Examples
```bash
voicegg ctl preset game game_cs2
voicegg ctl preset mic mic_broadcast
voicegg ctl cycle-preset game
```

---

### 2.8 `voicegg panic`
Emergency recovery command that resets the entire PipeWire graph to hardware defaults.

#### Syntax
```bash
voicegg panic
```

---

## 3. Scripting Recipes & Automation

### 3.1 Clutched Game Audio Script
Create `~/.local/bin/clutch-mode.sh`:
```bash
#!/usr/bin/env bash
# Lower Discord volume and boost Game audio for 30 seconds during a clutch
voicegg ctl chatmix -60
voicegg ctl preset game game_cs2
sleep 30
voicegg ctl chatmix 0
```

### 3.2 Discord Mute Sync with System LED
```bash
#!/usr/bin/env bash
voicegg ctl mute mic toggle
MUTED=$(voicegg status --json | jq -r '.channels.mic.muted')
if [ "$MUTED" = "true" ]; then
    brightnessctl --device='platform::micmute' set 1
else
    brightnessctl --device='platform::micmute' set 0
fi
```
