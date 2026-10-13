# Competitive Gaming Presets & Acoustic Tuning Guide

VoiceGG features an acoustic engine tuned specifically for competitive esports and studio-grade voice broadcasting. This guide details the acoustic theory, frequency breakdown, and exact 10-band parametric EQ curves calibrated for each competitive title.

---

## 1. Acoustic Theory for Esports Audio

Competitive gaming audio is fundamentally an exercise in **signal-to-noise ratio management**. In games like *Counter-Strike 2*, *Apex Legends*, or *Escape from Tarkov*, critical positional audio cues (such as enemy footfalls, weapon reloads, scope clicks, and grenade pin pulls) are constantly masked by low-frequency noise (artillery rumble, HE grenade explosions, airstrikes, and vehicle engines).

```
   Frequency Band         Acoustic Characteristic                 Esports Function
──────────────────────────────────────────────────────────────────────────────────────────────────
   20 Hz – 60 Hz          Sub-Bass                                Explosion rumble, air turbines (CUT)
   60 Hz – 250 Hz         Bass / Chest                            Gunshot body, vehicle engines (REDUCE)
   250 Hz – 500 Hz        Low-Midrange                            Mud / Boxiness, room resonance (CUT)
   500 Hz – 2000 Hz       Midrange                                Footsteps on gravel/dirt, sliding (BOOST)
   2000 Hz – 4000 Hz      Upper Mids / Presence                   Footsteps on metal/wood, foliage (PEAK BOOST)
   4000 Hz – 8000 Hz      Brilliance / Treble                     Pin pulls, reload clicks, defuse cues (BOOST)
   8000 Hz – 20000 Hz     Air                                     Cymbal sizzle, ear fatigue (TAME)
```

### The Fletcher-Munson Equal-Loudness Curve
The human ear is naturally most sensitive between **2 kHz and 5 kHz**—coincidentally where spoken consonants and tactical metal/wood clicks reside. By slightly attenuating distracting sub-bass (< 80 Hz) and focusing energy within this high-intelligibility window, players can discern subtle enemy movements at twice the distance without turning up their overall headphone volume to hearing-damaging levels.

---

## 2. Competitive Game Profiles Breakdown

### 🎯 Counter-Strike 2 (Footstep & Defuse Isolation)
* **Goal**: Isolate footstep impacts across Mirage, Inferno, and Nuke while preventing HE grenade concussions from deafening the player.
* **Key Cues**: Footsteps on metal catwalks (2.5 kHz), bomb plant & defuse beeps (3.8 kHz), weapon reload/scope clicks (5.2 kHz).

| Band | Type | Frequency | Q Factor | Gain (dB) | Purpose |
|:---:|:---:|:---:|:---:|:---:|:---|
| 0 | Low Shelf | 32 Hz | 0.71 | -6.0 dB | Eliminate sub-audible grenade rumble |
| 1 | Peaking | 64 Hz | 1.00 | -4.0 dB | Tame low-end explosion resonance |
| 2 | Peaking | 125 Hz | 1.20 | -2.0 dB | Clean up bass mask |
| 3 | Peaking | 250 Hz | 1.40 | -3.0 dB | Remove boxy room reverberation |
| 4 | Peaking | 500 Hz | 1.20 | +1.5 dB | Bring out footfall ground contact |
| 5 | Peaking | 1,000 Hz | 1.10 | +3.5 dB | Solidify wood and stone footsteps |
| 6 | Peaking | 2,000 Hz | 1.30 | +6.0 dB | Peak boost for metal stairs & footsteps |
| 7 | Peaking | 4,000 Hz | 1.40 | +5.0 dB | Bomb defuse click & scope zoom cue |
| 8 | Peaking | 8,000 Hz | 1.20 | +2.0 dB | Reload mechanism rattle clarity |
| 9 | High Shelf | 16,000 Hz | 0.71 | -1.5 dB | Tame high-frequency flashbang ringing |

---

### ⚡ Apex Legends (Movement & Slide Audio)
* **Goal**: Overcome loud ultimate abilities (Gibraltar / Bangalore strikes) and elevate sliding sounds on metal structures in World's Edge and Olympus.
* **Key Cues**: Crouch-slide friction (800 Hz – 1.8 kHz), zip-line attachment clicks (3.2 kHz), shield crack audio (4.5 kHz).

| Band | Type | Frequency | Q Factor | Gain (dB) | Purpose |
|:---:|:---:|:---:|:---:|:---:|:---|
| 0 | Low Shelf | 32 Hz | 0.71 | -5.0 dB | Filter out chaotic ultimate bass rumble |
| 1 | Peaking | 64 Hz | 1.10 | -3.0 dB | Reduce explosion wash |
| 2 | Peaking | 125 Hz | 1.20 | -1.5 dB | Clean up low-end engine noise |
| 3 | Peaking | 250 Hz | 1.30 | -2.5 dB | Cut muddy reverberation |
| 4 | Peaking | 500 Hz | 1.00 | +2.0 dB | Boost crouch-slide body |
| 5 | Peaking | 1,000 Hz | 1.20 | +4.0 dB | Elevate running steps on terrain |
| 6 | Peaking | 2,000 Hz | 1.40 | +5.5 dB | Zipline grab and metal footfalls |
| 7 | Peaking | 4,000 Hz | 1.30 | +4.5 dB | Shield battery / cell charge cues |
| 8 | Peaking | 8,000 Hz | 1.10 | +2.5 dB | Shield crack shatter intelligibility |
| 9 | High Shelf | 16,000 Hz | 0.71 | 0.0 dB | Natural air retention |

---

### 🛡️ Valorant (HRTF Precision & Spike Audio)
* **Goal**: Complement Riot's native HRTF binaural engine with sharp directional presence and distinct Spike interaction audio.
* **Key Cues**: Step transitions between gravel/wood/tile (1.2 kHz – 2.8 kHz), Spike plant/half-defuse sound (4 kHz – 5.5 kHz).

| Band | Type | Frequency | Q Factor | Gain (dB) | Purpose |
|:---:|:---:|:---:|:---:|:---:|:---|
| 0 | Low Shelf | 32 Hz | 0.71 | -4.0 dB | Tighten bottom end |
| 1 | Peaking | 64 Hz | 1.00 | -2.0 dB | Control ability rumble |
| 2 | Peaking | 125 Hz | 1.10 | -1.0 dB | Transparent mid-bass |
| 3 | Peaking | 250 Hz | 1.20 | -2.0 dB | Remove room boxiness |
| 4 | Peaking | 500 Hz | 1.10 | +1.0 dB | Footstep body |
| 5 | Peaking | 1,000 Hz | 1.20 | +3.0 dB | Stone surface footfalls |
| 6 | Peaking | 2,000 Hz | 1.30 | +5.0 dB | Wood & tile step presence |
| 7 | Peaking | 4,000 Hz | 1.40 | +5.5 dB | Spike plant & tap notification sound |
| 8 | Peaking | 8,000 Hz | 1.10 | +2.0 dB | Ability cast click cues |
| 9 | High Shelf | 16,000 Hz | 0.71 | -1.0 dB | Smooth highs |

---

### 🪖 Escape from Tarkov (Foliage & Step Detection)
* **Goal**: Maximize survival audio across woods and dorms by boosting bush rustles and wood creaks while mitigating loud rain and wind.
* **Key Cues**: Bush and branch brushes (3.5 kHz – 6 kHz), floorboard creaking in dorms (1.8 kHz), distant ADS (aim-down-sights) fabric rustle (4.2 kHz).

| Band | Type | Frequency | Q Factor | Gain (dB) | Purpose |
|:---:|:---:|:---:|:---:|:---:|:---|
| 0 | Low Shelf | 32 Hz | 0.71 | -7.0 dB | Severe cut to heavy ambient wind rumble |
| 1 | Peaking | 64 Hz | 1.20 | -5.0 dB | Tame rain low-frequency thump |
| 2 | Peaking | 125 Hz | 1.20 | -3.0 dB | Clear out heavy armor movement thuds |
| 3 | Peaking | 250 Hz | 1.30 | -3.5 dB | Eliminate indoor room hollow resonance |
| 4 | Peaking | 500 Hz | 1.10 | +1.5 dB | Enhance gravel running audio |
| 5 | Peaking | 1,000 Hz | 1.20 | +3.5 dB | Ground step impact definition |
| 6 | Peaking | 2,000 Hz | 1.40 | +6.5 dB | Dorm floorboard creaks & metal stairs |
| 7 | Peaking | 4,000 Hz | 1.50 | +6.0 dB | Bush rustle, barbed wire, ADS cloth sound |
| 8 | Peaking | 8,000 Hz | 1.20 | +3.0 dB | Grenade pin drop and magazine check |
| 9 | High Shelf | 16,000 Hz | 0.71 | -2.0 dB | Tame loud ambient rain sizzle |

---

### 🔫 Rainbow Six Siege (Destruction & Rappel Acoustics)
* **Goal**: Discern sound propagation through breached walls, floorboard punch-holes, and exterior rappel ropes.
* **Key Cues**: Barbed wire movement (3 kHz), wooden barricade splintering (1.5 kHz – 2.5 kHz), rappel rope swing (2.2 kHz).

| Band | Type | Frequency | Q Factor | Gain (dB) | Purpose |
|:---:|:---:|:---:|:---:|:---:|:---|
| 0 | Low Shelf | 32 Hz | 0.71 | -5.0 dB | Dampen breach charge explosion bass |
| 1 | Peaking | 64 Hz | 1.00 | -3.0 dB | Tighten shotgun blast echoes |
| 2 | Peaking | 125 Hz | 1.10 | -1.5 dB | Clean up drone motor hum |
| 3 | Peaking | 250 Hz | 1.20 | -2.0 dB | Cut wall hollow cavity resonance |
| 4 | Peaking | 500 Hz | 1.00 | +1.0 dB | Footstep ground resonance |
| 5 | Peaking | 1,000 Hz | 1.20 | +3.0 dB | Barricade wood hit cue |
| 6 | Peaking | 2,000 Hz | 1.30 | +5.5 dB | Barbed wire & drone roll sounds |
| 7 | Peaking | 4,000 Hz | 1.40 | +5.0 dB | Rappel hook attachment & footsteps |
| 8 | Peaking | 8,000 Hz | 1.10 | +2.0 dB | Defuser electronic prompt audio |
| 9 | High Shelf | 16,000 Hz | 0.71 | 0.0 dB | Crisp soundstage |

---

## 3. Microphone & Voice Profiles

### 🎙️ Warm Broadcast (Radio & Streamer Presence)
Provides a rich, authoritative broadcast voice similar to an Shure SM7B through an analog preamp:
- **Bass Depth (120 Hz)**: +3.0 dB low-shelf boost for full proximity warmth.
- **Mud Cut (350 Hz)**: -3.5 dB narrow cut to remove cheap room hollow resonance.
- **Vocal Presence (3.2 kHz)**: +4.0 dB boost for crisp consonant intelligibility.
- **Air (12 kHz)**: +2.0 dB high-shelf boost for studio sparkle.

### 🗣️ Crystal Clear Comms (Esports / Discord Clutch)
Engineered for maximum voice clarity in noisy in-game voice channels:
- **Highpass Filter**: Cuts everything below 100 Hz.
- **Presence Boost (2.5 kHz – 4 kHz)**: +5.0 dB emphasis so your callouts cut through loud gunfights.
- **Aggressive Hysteresis Gate**: Instantly mutes background fan noise and keyboard clicks.

---

## 4. Custom Preset Management & CLI Automation

Presets are stored as standard JSON structures in `~/.config/voicegg/presets.json` and can be imported, exported, or hot-swapped via the CLI:

### Exporting Your Active Preset
```bash
voicegg export game my_custom_cs2.json
```

### Importing a Shared Pro Gamer Preset
```bash
voicegg import game my_custom_cs2.json
```

### Switching Presets via Keybinds (Hyprland / Sway)
```ini
bind = SUPER SHIFT, F1, exec, voicegg preset game cs2
bind = SUPER SHIFT, F2, exec, voicegg preset game apex
bind = SUPER SHIFT, F3, exec, voicegg preset game valorant
bind = SUPER SHIFT, F4, exec, voicegg preset game flat
```
