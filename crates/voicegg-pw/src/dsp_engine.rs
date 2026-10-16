//! Real-time in-process DSP audio engine for VoiceGG.
//!
//! Provides dedicated per-channel processing workers with parametric equalizers,
//! volume meters, and a comprehensive microphone voice processing suite (AI noise suppression,
//! noise gate, compressor, limiter). Also orchestrates low-latency microphone capture and
//! playback streaming.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use voicegg_core::{ChannelId, Preset};
use voicegg_dsp::{Compressor, Limiter, NoiseGate, NoiseSuppressor, ParametricEq, VUMeter};

/// Peak and RMS audio meter levels for a channel in dBFS.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChannelMeter {
    /// Peak level in dBFS.
    pub peak_dbfs: f32,
    /// RMS level in dBFS.
    pub rms_dbfs: f32,
}

impl Default for ChannelMeter {
    fn default() -> Self {
        Self {
            peak_dbfs: -100.0,
            rms_dbfs: -100.0,
        }
    }
}

/// A dedicated real-time DSP worker managing audio processing and metering for an individual channel.
pub struct DspChannelWorker {
    /// Associated channel identifier.
    pub channel: ChannelId,
    /// Active audio preset wrapped in a thread-safe read-write lock.
    pub preset: Arc<RwLock<Preset>>,
    /// Current peak and RMS volume meter reading.
    pub meter: Arc<RwLock<ChannelMeter>>,
    /// Active state flag.
    pub active: Arc<AtomicBool>,
    sample_rate: f32,
    eq: ParametricEq,
    eq_r: ParametricEq,
    vu_meter: VUMeter,
    // Microphone-specific DSP processors:
    noise_suppressor: Option<NoiseSuppressor>,
    noise_gate: Option<NoiseGate>,
    compressor: Option<Compressor>,
    limiter: Option<Limiter>,
}

impl DspChannelWorker {
    /// Creates a new channel worker for the specified channel and preset.
    #[must_use]
    pub fn new(channel: ChannelId, sample_rate: f32, preset: Preset) -> Self {
        let preset_arc = Arc::new(RwLock::new(preset.clone()));
        let meter_arc = Arc::new(RwLock::new(ChannelMeter::default()));
        let active_arc = Arc::new(AtomicBool::new(true));

        let mut eq = ParametricEq::new(sample_rate);
        eq.update_config(&preset.eq);

        let mut eq_r = ParametricEq::new(sample_rate);
        eq_r.update_config(&preset.eq);

        let vu_meter = VUMeter::new();

        let (noise_suppressor, noise_gate, compressor, limiter) = if channel == ChannelId::Mic {
            let mut ns = NoiseSuppressor::new();
            if let Some(ref nc) = preset.noise_canceller {
                ns.set_reduction_percent(nc.amount as u8);
                ns.set_vad_threshold(nc.vad_threshold);
                ns.set_enabled(nc.enabled);
            }

            let gate_cfg = preset.noise_gate.clone().unwrap_or_default();
            let gate = NoiseGate::new(sample_rate, &gate_cfg);

            let comp_cfg = preset.compressor.clone().unwrap_or_default();
            let comp = Compressor::new(sample_rate, &comp_cfg);

            let lim_cfg = preset.limiter.clone().unwrap_or_default();
            let lim = Limiter::new(sample_rate, lim_cfg.ceiling_db, lim_cfg.release_ms);

            (Some(ns), Some(gate), Some(comp), Some(lim))
        } else {
            let lim = preset.limiter.as_ref().and_then(|l| {
                if l.enabled {
                    Some(Limiter::new(sample_rate, l.ceiling_db, l.release_ms))
                } else {
                    None
                }
            });
            (None, None, None, lim)
        };

        Self {
            channel,
            preset: preset_arc,
            meter: meter_arc,
            active: active_arc,
            sample_rate,
            eq,
            eq_r,
            vu_meter,
            noise_suppressor,
            noise_gate,
            compressor,
            limiter,
        }
    }

    /// Updates the worker's active audio preset and recalculates DSP filter coefficients.
    pub fn update_preset(&mut self, preset: &Preset) {
        if let Ok(mut p) = self.preset.write() {
            *p = preset.clone();
        } else if let Err(p) = self.preset.write() {
            *p.into_inner() = preset.clone();
        }

        self.eq.update_config(&preset.eq);
        self.eq_r.update_config(&preset.eq);

        if self.channel == ChannelId::Mic {
            if let Some(ref mut ns) = self.noise_suppressor {
                if let Some(ref nc) = preset.noise_canceller {
                    ns.set_reduction_percent(nc.amount as u8);
                    ns.set_vad_threshold(nc.vad_threshold);
                    ns.set_enabled(nc.enabled);
                } else {
                    ns.set_enabled(false);
                }
            }
            if let Some(ref mut gate) = self.noise_gate {
                if let Some(ref g) = preset.noise_gate {
                    gate.update_config(g);
                }
            }
            if let Some(ref mut comp) = self.compressor {
                if let Some(ref c) = preset.compressor {
                    comp.update_config(c);
                }
            }
            if let Some(ref lim) = preset.limiter {
                self.limiter = Some(Limiter::new(self.sample_rate, lim.ceiling_db, lim.release_ms));
            }
        } else if let Some(ref lim) = preset.limiter {
            if lim.enabled {
                self.limiter = Some(Limiter::new(self.sample_rate, lim.ceiling_db, lim.release_ms));
            } else {
                self.limiter = None;
            }
        } else {
            self.limiter = None;
        }
    }

    /// Processes a buffer of mono audio samples in-place. Returns (Peak dBFS, RMS dBFS).
    pub fn process_mono_frame(&mut self, buffer: &mut [f32]) -> (f32, f32) {
        if !self.active.load(Ordering::Relaxed) || buffer.is_empty() {
            return (-100.0, -100.0);
        }

        if self.channel == ChannelId::Mic {
            if let Some(ref mut ns) = self.noise_suppressor {
                ns.process(buffer);
            }
            if let Some(ref mut gate) = self.noise_gate {
                gate.process_slice(buffer);
            }
            self.eq.process(buffer);
            if let Some(ref mut comp) = self.compressor {
                comp.process_slice(buffer);
            }
            if let Some(ref mut lim) = self.limiter {
                lim.process_slice(buffer);
            }
        } else {
            self.eq.process(buffer);
            if let Some(ref mut lim) = self.limiter {
                lim.process_slice(buffer);
            }
        }

        self.vu_meter.reset();
        self.vu_meter.process(buffer);
        let peak = self.vu_meter.peak_dbfs();
        let rms = self.vu_meter.rms_dbfs();

        if let Ok(mut m) = self.meter.write() {
            m.peak_dbfs = peak;
            m.rms_dbfs = rms;
        } else if let Err(m) = self.meter.write() {
            let mut inner = m.into_inner();
            inner.peak_dbfs = peak;
            inner.rms_dbfs = rms;
        }

        (peak, rms)
    }

    /// Processes stereo audio buffers (left and right channels) in-place. Returns (Peak dBFS, RMS dBFS).
    pub fn process_stereo_frame(&mut self, left: &mut [f32], right: &mut [f32]) -> (f32, f32) {
        if !self.active.load(Ordering::Relaxed) || (left.is_empty() && right.is_empty()) {
            return (-100.0, -100.0);
        }

        if self.channel == ChannelId::Mic {
            if let Some(ref mut ns) = self.noise_suppressor {
                ns.process(left);
                if !right.is_empty() {
                    let mut temp = right.to_vec();
                    ns.process(&mut temp);
                    right.copy_from_slice(&temp);
                }
            }
            if let Some(ref mut gate) = self.noise_gate {
                gate.process_slice(left);
                gate.process_slice(right);
            }
            self.eq.process(left);
            self.eq_r.process(right);
            if let Some(ref mut comp) = self.compressor {
                comp.process_slice(left);
                comp.process_slice(right);
            }
            if let Some(ref mut lim) = self.limiter {
                lim.process_slice(left);
                lim.process_slice(right);
            }
        } else {
            self.eq.process(left);
            self.eq_r.process(right);
            if let Some(ref mut lim) = self.limiter {
                lim.process_slice(left);
                lim.process_slice(right);
            }
        }

        self.vu_meter.reset();
        if !left.is_empty() {
            self.vu_meter.process(left);
        }
        if !right.is_empty() {
            self.vu_meter.process(right);
        }
        let peak = self.vu_meter.peak_dbfs();
        let rms = self.vu_meter.rms_dbfs();

        if let Ok(mut m) = self.meter.write() {
            m.peak_dbfs = peak;
            m.rms_dbfs = rms;
        } else if let Err(m) = self.meter.write() {
            let mut inner = m.into_inner();
            inner.peak_dbfs = peak;
            inner.rms_dbfs = rms;
        }

        (peak, rms)
    }

    /// Returns a copy of the current meter reading in dBFS.
    #[must_use]
    pub fn get_meter(&self) -> ChannelMeter {
        self.meter.read().map(|m| *m).unwrap_or_default()
    }

    /// Resets all internal DSP processor histories and meters.
    pub fn reset(&mut self) {
        self.eq.reset();
        self.eq_r.reset();
        self.vu_meter.reset();
        if let Some(ref mut ns) = self.noise_suppressor {
            ns.reset();
        }
        if let Some(ref mut gate) = self.noise_gate {
            gate.reset();
        }
        if let Some(ref mut comp) = self.compressor {
            comp.reset();
        }
        if let Some(ref mut lim) = self.limiter {
            lim.reset();
        }
    }

    /// Returns an immutable reference to the parametric equalizer.
    #[must_use]
    pub fn eq(&self) -> &ParametricEq {
        &self.eq
    }

    /// Returns a mutable reference to the parametric equalizer.
    pub fn eq_mut(&mut self) -> &mut ParametricEq {
        &mut self.eq
    }

    /// Returns a reference to the VU meter.
    #[must_use]
    pub fn vu_meter(&self) -> &VUMeter {
        &self.vu_meter
    }
}

struct MicWorkerHandle {
    capture_child: Option<Child>,
    playback_child: Option<Child>,
    thread: Option<JoinHandle<()>>,
}

/// Real-time in-process audio DSP engine coordinating all channel workers and microphone streams.
pub struct DspEngine {
    /// Active channel DSP workers mapped by channel ID.
    pub workers: HashMap<ChannelId, DspChannelWorker>,
    mic_worker_handle: Option<MicWorkerHandle>,
}

impl Default for DspEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DspEngine {
    /// Creates a new DSP engine initialized with default channel workers.
    #[must_use]
    pub fn new() -> Self {
        let sample_rate = 48000.0;
        let mut workers = HashMap::new();

        let channels = [
            (
                ChannelId::Mic,
                Preset::get_preset_by_id("mic_broadcast").unwrap_or_default(),
            ),
            (
                ChannelId::Game,
                Preset::get_preset_by_id("game_flat").unwrap_or_default(),
            ),
            (
                ChannelId::Chat,
                Preset::get_preset_by_id("chat_clear_voice").unwrap_or_default(),
            ),
            (
                ChannelId::Media,
                Preset::get_preset_by_id("media_podcast").unwrap_or_default(),
            ),
            (ChannelId::Aux, Preset::default()),
            (ChannelId::Master, Preset::default()),
        ];

        for (channel, preset) in channels {
            workers.insert(
                channel,
                DspChannelWorker::new(channel, sample_rate, preset),
            );
        }

        Self {
            workers,
            mic_worker_handle: None,
        }
    }

    /// Returns an instantaneous snapshot of volume meters across all channels.
    #[must_use]
    pub fn sample_meters(&self) -> HashMap<ChannelId, ChannelMeter> {
        let mut meters = HashMap::with_capacity(self.workers.len());
        for (&channel, worker) in &self.workers {
            meters.insert(channel, worker.get_meter());
        }
        meters
    }

    /// Updates the active audio preset for a specific channel.
    pub fn update_channel_preset(&mut self, channel: ChannelId, preset: &Preset) {
        if let Some(worker) = self.workers.get_mut(&channel) {
            worker.update_preset(preset);
        }
    }

    /// Spawns audio capture & playback workers for Mic using `parec` and `pacat` float32 streams
    /// (with an automated loopback buffer fallback if audio server tools are unavailable).
    pub fn spawn_mic_worker(&mut self, source_device: Option<&str>, target_sink: Option<&str>) {
        self.stop_mic_worker();

        let mic_worker = match self.workers.get(&ChannelId::Mic) {
            Some(w) => w,
            None => return,
        };

        let active = mic_worker.active.clone();
        let preset_arc = mic_worker.preset.clone();
        let meter_arc = mic_worker.meter.clone();
        active.store(true, Ordering::Relaxed);

        let mut cap_cmd = Command::new("parec");
        cap_cmd.args(["--format=float32le", "--rate=48000", "--channels=1", "--raw"]);
        if let Some(src) = source_device {
            if !src.is_empty() && src != "default" {
                cap_cmd.args(["--device", src]);
            }
        }
        cap_cmd.stdout(Stdio::piped()).stderr(Stdio::null());

        let mut play_cmd = Command::new("pacat");
        play_cmd.args(["--playback", "--format=float32le", "--rate=48000", "--channels=1", "--raw"]);
        if let Some(tgt) = target_sink {
            if !tgt.is_empty() && tgt != "default" {
                play_cmd.args(["--device", tgt]);
            }
        } else {
            play_cmd.args(["--device", ChannelId::Mic.pipewire_node_name()]);
        }
        play_cmd.stdin(Stdio::piped()).stderr(Stdio::null());

        let cap_res = cap_cmd.spawn();
        let play_res = play_cmd.spawn();

        match (cap_res, play_res) {
            (Ok(mut cap_child), Ok(mut play_child)) => {
                let mut cap_stdout = cap_child.stdout.take();
                let mut play_stdin = play_child.stdin.take();
                let active_thread = active;

                let thread = std::thread::Builder::new()
                    .name("voicegg-mic-worker".to_string())
                    .spawn(move || {
                        let initial_preset =
                            preset_arc.read().map(|p| p.clone()).unwrap_or_default();
                        let mut dsp =
                            DspChannelWorker::new(ChannelId::Mic, 48000.0, initial_preset);
                        let mut buf_bytes = [0u8; 480 * 4];
                        let mut samples = [0.0f32; 480];

                        while active_thread.load(Ordering::Relaxed) {
                            if let Ok(p) = preset_arc.try_read() {
                                dsp.update_preset(&p);
                            }

                            if let Some(ref mut reader) = cap_stdout {
                                if reader.read_exact(&mut buf_bytes).is_err() {
                                    break;
                                }
                                for (i, chunk) in buf_bytes.as_chunks::<4>().0.iter().enumerate() {
                                    samples[i] = f32::from_le_bytes(*chunk);
                                }

                                let (peak, rms) = dsp.process_mono_frame(&mut samples);
                                if let Ok(mut m) = meter_arc.write() {
                                    m.peak_dbfs = peak;
                                    m.rms_dbfs = rms;
                                }

                                if let Some(ref mut writer) = play_stdin {
                                    for (i, &s) in samples.iter().enumerate() {
                                        let b = s.to_le_bytes();
                                        buf_bytes[i * 4..i * 4 + 4].copy_from_slice(&b);
                                    }
                                    if writer.write_all(&buf_bytes).is_err() {
                                        break;
                                    }
                                    let _ = writer.flush();
                                }
                            } else {
                                break;
                            }
                        }
                    })
                    .ok();

                self.mic_worker_handle = Some(MicWorkerHandle {
                    capture_child: Some(cap_child),
                    playback_child: Some(play_child),
                    thread,
                });
            }
            (partial_cap, partial_play) => {
                if let Ok(mut c) = partial_cap {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                if let Ok(mut p) = partial_play {
                    let _ = p.kill();
                    let _ = p.wait();
                }

                tracing::warn!("parec or pacat unavailable; running loopback fallback for Mic DSP");
                let active_thread = active;
                let thread = std::thread::Builder::new()
                    .name("voicegg-mic-loopback".to_string())
                    .spawn(move || {
                        let initial_preset =
                            preset_arc.read().map(|p| p.clone()).unwrap_or_default();
                        let mut dsp =
                            DspChannelWorker::new(ChannelId::Mic, 48000.0, initial_preset);
                        let mut buffer = [0.0f32; 480];
                        while active_thread.load(Ordering::Relaxed) {
                            if let Ok(p) = preset_arc.try_read() {
                                dsp.update_preset(&p);
                            }
                            let (peak, rms) = dsp.process_mono_frame(&mut buffer);
                            if let Ok(mut m) = meter_arc.write() {
                                m.peak_dbfs = peak;
                                m.rms_dbfs = rms;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    })
                    .ok();

                self.mic_worker_handle = Some(MicWorkerHandle {
                    capture_child: None,
                    playback_child: None,
                    thread,
                });
            }
        }
    }

    /// Stops the microphone background capture and playback worker.
    pub fn stop_mic_worker(&mut self) {
        if let Some(mut handle) = self.mic_worker_handle.take() {
            if let Some(worker) = self.workers.get(&ChannelId::Mic) {
                worker.active.store(false, Ordering::Relaxed);
            }
            if let Some(mut cap) = handle.capture_child.take() {
                let _ = cap.kill();
                let _ = cap.wait();
            }
            if let Some(mut play) = handle.playback_child.take() {
                let _ = play.kill();
                let _ = play.wait();
            }
            if let Some(th) = handle.thread.take() {
                let _ = th.join();
            }
        }
    }

    /// Stops all channel workers, active processes, and background threads gracefully.
    pub fn stop_all(&mut self) {
        self.stop_mic_worker();
        for worker in self.workers.values() {
            worker.active.store(false, Ordering::Relaxed);
        }
    }
}

impl Drop for DspEngine {
    fn drop(&mut self) {
        self.stop_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_worker_creation_and_mono_processing() {
        let preset = Preset::get_preset_by_id("mic_broadcast").unwrap_or_default();
        let mut worker = DspChannelWorker::new(ChannelId::Mic, 48000.0, preset);

        // Process first frame to prime the 480-sample RNNoise delay pipeline
        let mut buffer = vec![0.5_f32; 480];
        worker.process_mono_frame(&mut buffer);

        // Second frame contains active signal exiting the delay pipeline and opening the gate
        buffer.fill(0.5);
        let (peak, rms) = worker.process_mono_frame(&mut buffer);

        assert!(peak > -100.0, "Expected peak > -100 dBFS, got {peak}");
        assert!(rms > -100.0, "Expected rms > -100 dBFS, got {rms}");

        let meter = worker.get_meter();
        assert_eq!(meter.peak_dbfs, peak);
        assert_eq!(meter.rms_dbfs, rms);
    }

    #[test]
    fn test_channel_worker_stereo_processing() {
        let preset = Preset::get_preset_by_id("game_flat").unwrap_or_default();
        let mut worker = DspChannelWorker::new(ChannelId::Game, 48000.0, preset);

        let mut left = vec![0.2_f32; 256];
        let mut right = vec![0.2_f32; 256];
        let (peak, rms) = worker.process_stereo_frame(&mut left, &mut right);

        assert!(peak > -100.0);
        assert!(rms > -100.0);
    }

    #[test]
    fn test_dsp_engine_lifecycle_and_presets() {
        let mut engine = DspEngine::new();

        assert!(engine.workers.contains_key(&ChannelId::Mic));
        assert!(engine.workers.contains_key(&ChannelId::Game));
        assert!(engine.workers.contains_key(&ChannelId::Chat));
        assert!(engine.workers.contains_key(&ChannelId::Media));
        assert!(engine.workers.contains_key(&ChannelId::Aux));

        let meters = engine.sample_meters();
        assert!(meters.contains_key(&ChannelId::Mic));

        let new_preset = Preset::get_preset_by_id("game_cs2").unwrap_or_default();
        engine.update_channel_preset(ChannelId::Game, &new_preset);

        let p = engine.workers.get(&ChannelId::Game).unwrap().preset.read().unwrap();
        assert_eq!(p.id, "game_cs2");
    }

    #[test]
    fn test_dsp_engine_spawn_and_stop_all() {
        let mut engine = DspEngine::new();
        engine.spawn_mic_worker(None, None);

        // Give worker thread a moment to initialize
        std::thread::sleep(std::time::Duration::from_millis(30));

        engine.stop_all();
        assert!(!engine.workers.get(&ChannelId::Mic).unwrap().active.load(Ordering::Relaxed));
    }
}
