# VoiceGG PipeWire Diagnostic & Troubleshooting Playbook

This playbook covers common audio edge cases, diagnostic procedures, distro-specific plugin installations, and recovery workflows for VoiceGG and PipeWire.

---

## 1. Quick Diagnostic Checklist

If you experience unexpected audio behavior, run these two diagnostic commands first:

```bash
# 1. Check VoiceGG daemon status and IPC connectivity
voicegg status

# 2. Check PipeWire real-time node scheduler and Xruns (buffer overruns)
pw-top
```

Confirm that the PipeWire user services are active:
```bash
systemctl --user status pipewire pipewire-pulse wireplumber
```

---

## 2. Common Issues & Solutions

### 2.1 "Noise Remover is unavailable: install librnnoise_ladspa"

#### Symptom:
The Noise Remover toggle in the Mic tab displays a warning badge indicating that `librnnoise_ladspa.so` is missing.

#### Cause:
VoiceGG conducts an honest scan of system LADSPA/LV2 plugin directories (`/usr/lib/ladspa`, `/usr/lib64/ladspa`, `/usr/local/lib/ladspa`, `~/.ladspa`). If the native RNNoise neural network filter is not installed on the system, VoiceGG disables the filter rather than silently failing.

#### Solution: Install `noise-suppression-for-voice`

* **Arch Linux / Manjaro / EndeavourOS**:
  ```bash
  yay -S noise-suppression-for-voice
  # or using paru:
  paru -S noise-suppression-for-voice
  ```

* **Fedora**:
  ```bash
  sudo dnf install ladspa-noise-suppression-for-voice
  ```

* **Ubuntu 24.04+ / Debian 13+**:
  ```bash
  sudo apt install ladspa-noise-suppression-for-voice
  ```

* **openSUSE Tumbleweed**:
  ```bash
  sudo zypper in ladspa-noise-suppression-for-voice
  ```

* **Manual User-Space Installation (Any Distro)**:
  Download `librnnoise_ladspa.so` from the upstream [werman/noise-suppression-for-voice](https://github.com/werman/noise-suppression-for-voice) release, create `~/.ladspa/`, and place the `.so` inside:
  ```bash
  mkdir -p ~/.ladspa
  cp librnnoise_ladspa.so ~/.ladspa/
  systemctl --user restart voicegg-daemon
  ```

---

### 2.2 Live Microphone Monitoring Sounds Delayed (Echo)

#### Symptom:
When enabling Voice Monitoring (Sidetone), you hear a perceptible delay or double-voice echo.

#### Cause:
Your PipeWire audio server quantum buffer may be set to a high default (e.g., 1024 or 2048 samples, ~21ms to 42ms), or your USB audio interface is operating in a high-latency power-saving mode.

#### Solution:
1. Ensure your PipeWire quantum is locked to low latency:
   ```bash
   pw-metadata -n settings 0 clock.force-quantum 256
   ```
2. In VoiceGG, check that your physical microphone is connected to a high-speed USB port (avoid unpowered USB 2.0 hubs).
3. If using Bluetooth headphones, ensure you are using the **FastStream** or **AptX-LL** codec rather than high-latency SBC or LDAC.

---

### 2.3 Audio Crackling or Pops During Heavy Gaming (Xruns)

#### Symptom:
Occasional audio stuttering, clicks, or pops during intense gameplay.

#### Cause:
PipeWire is experiencing buffer underruns (Xruns) because the user account lacks real-time thread scheduling permissions (`rtprio`).

#### Solution:
1. Verify if your user belongs to the `audio` or `realtime` group:
   ```bash
   groups
   ```
2. If not, add your user and configure real-time limits:
   ```bash
   sudo usermod -aG audio $USER
   ```
3. Create `/etc/security/limits.d/99-realtime-audio.conf` with:
   ```ini
   @audio - rtprio 95
   @audio - memlock unlimited
   @audio - nice -19
   ```
4. Log out and log back in to apply limits.

---

### 2.4 Application Audio Not Moving When Dragged in Mixer

#### Symptom:
Dragging Spotify or Discord into the Media or Chat channel does not redirect the audio stream.

#### Cause:
Another patchbay (e.g., EasyEffects or Helvum) or a custom WirePlumber script is actively locking stream routing.

#### Solution:
1. Inspect WirePlumber stream state:
   ```bash
   wpctl status
   ```
2. Clear conflicting manual loopbacks in PipeWire:
   ```bash
   pw-cli dump | grep -i loopback
   ```
3. Use the VoiceGG CLI to force-route the stream:
   ```bash
   voicegg route discord chat
   ```

---

### 2.5 Emergency Panic Recovery (Instant Audio Restore)

#### Symptom:
Audio graph deadlock, no sound coming out of headphones, or unintended feedback loop.

#### Infallible Recovery:
Run the VoiceGG Panic command:

```bash
voicegg panic
```

What `voicegg panic` does immediately:
1. Terminates all VoiceGG filter-chain child processes.
2. Unloads all VoiceGG virtual sinks (`voicegg_sink_*`) and loopback modules.
3. Queries your physical hardware soundcard via `wpctl`.
4. Reroutes all active audio streams back to the physical hardware default.
5. Fully restores native audio playback in **under 500 milliseconds**.
