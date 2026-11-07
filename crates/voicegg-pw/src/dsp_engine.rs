//! Real-time in-process DSP audio engine for VoiceGG.
//!
//! Provides dedicated per-channel processing workers with parametric equalizers,
//! volume meters, 3D spatial soundstage virtualization, smart volume normalization,
//! and a comprehensive microphone voice processing suite (AI noise suppression,
//! noise gate, compressor, limiter). Also orchestrates low-latency microphone capture and
//! per-channel stereo playback streaming.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use voicegg_core::{ChannelId, CompressorConfig, Preset, SmartVolumeConfig};
use voicegg_dsp::{
    Compressor, Limiter, NoiseGate, NoiseSuppressor, ParametricEq, SpatialProcessor, VUMeter,
};

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

/// Routing, volume fader, and dual-bus mix state for a channel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChannelRoutingState {
    /// Volume fader level (0 ..= 150, with 100 representing 0 dBFS unity gain).
    pub volume: u8,
    /// Whether channel is muted.
    pub muted: bool,
    /// Whether audio is routed to the headphone / personal monitoring mix.
    pub personal_mix: bool,
    /// Whether audio is routed to the stream audience mix.
    pub stream_mix: bool,
}

impl Default for ChannelRoutingState {
    fn default() -> Self {
        Self {
            volume: 100,
            muted: false,
            personal_mix: true,
            stream_mix: true,
        }
    }
}

fn smart_volume_to_compressor_config(sv: Option<&SmartVolumeConfig>) -> CompressorConfig {
    if let Some(cfg) = sv {
        let level = match cfg.mode.to_ascii_lowercase().as_str() {
            "quiet" => 0.4_f32,
            "loud" => 1.0_f32,
            _ => 0.7_f32, // "balanced"
        };
        CompressorConfig {
            enabled: cfg.enabled,
            threshold_db: -18.0 - (level * 12.0),
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 120.0,
            makeup_gain_db: level * 6.0,
            knee_db: 4.0,
        }
    } else {
        CompressorConfig::default()
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
    /// Instant when the last audio frame was processed.
    pub last_frame_instant: Arc<RwLock<Instant>>,
    sample_rate: f32,
    eq: ParametricEq,
    eq_r: ParametricEq,
    vu_meter: VUMeter,
    // Playback-specific DSP processors:
    spatial: Option<SpatialProcessor>,
    smart_volume: Option<Compressor>,
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
        let last_frame_instant = Arc::new(RwLock::new(Instant::now()));

        let mut eq = ParametricEq::new(sample_rate);
        eq.update_config(&preset.eq);

        let mut eq_r = ParametricEq::new(sample_rate);
        eq_r.update_config(&preset.eq);

        let vu_meter = VUMeter::new();

        let (spatial, smart_volume, noise_suppressor, noise_gate, compressor, limiter) =
            if channel == ChannelId::Mic {
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

                (None, None, Some(ns), Some(gate), Some(comp), Some(lim))
            } else {
                let mut sp = SpatialProcessor::new(sample_rate, 0.5, 0.5);
                if let Some(ref sp_cfg) = preset.spatial {
                    sp.set_params(0.5, f32::from(sp_cfg.immersion_pct) / 100.0);
                    sp.set_enabled(sp_cfg.enabled);
                } else {
                    sp.set_enabled(false);
                }

                let sv_cfg = smart_volume_to_compressor_config(preset.smart_volume.as_ref());
                let sv = Compressor::new(sample_rate, &sv_cfg);

                let lim = preset.limiter.as_ref().and_then(|l| {
                    if l.enabled {
                        Some(Limiter::new(sample_rate, l.ceiling_db, l.release_ms))
                    } else {
                        None
                    }
                });
                (Some(sp), Some(sv), None, None, None, lim)
            };

        Self {
            channel,
            preset: preset_arc,
            meter: meter_arc,
            active: active_arc,
            last_frame_instant,
            sample_rate,
            eq,
            eq_r,
            vu_meter,
            spatial,
            smart_volume,
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
                self.limiter = Some(Limiter::new(
                    self.sample_rate,
                    lim.ceiling_db,
                    lim.release_ms,
                ));
            }
        } else {
            if let Some(ref mut sp) = self.spatial {
                if let Some(ref sp_cfg) = preset.spatial {
                    sp.set_params(0.5, f32::from(sp_cfg.immersion_pct) / 100.0);
                    sp.set_enabled(sp_cfg.enabled);
                } else {
                    sp.set_enabled(false);
                }
            } else if let Some(ref sp_cfg) = preset.spatial {
                let mut sp = SpatialProcessor::new(
                    self.sample_rate,
                    0.5,
                    f32::from(sp_cfg.immersion_pct) / 100.0,
                );
                sp.set_enabled(sp_cfg.enabled);
                self.spatial = Some(sp);
            }

            let sv_cfg = smart_volume_to_compressor_config(preset.smart_volume.as_ref());
            if let Some(ref mut sv) = self.smart_volume {
                sv.update_config(&sv_cfg);
            } else {
                self.smart_volume = Some(Compressor::new(self.sample_rate, &sv_cfg));
            }

            if let Some(ref lim) = preset.limiter {
                if lim.enabled {
                    self.limiter = Some(Limiter::new(
                        self.sample_rate,
                        lim.ceiling_db,
                        lim.release_ms,
                    ));
                } else {
                    self.limiter = None;
                }
            } else {
                self.limiter = None;
            }
        }
    }

    /// Processes a buffer of mono audio samples in-place. Returns (Peak dBFS, RMS dBFS).
    pub fn process_mono_frame(&mut self, buffer: &mut [f32]) -> (f32, f32) {
        if !self.active.load(Ordering::Relaxed) || buffer.is_empty() {
            return (-100.0, -100.0);
        }

        if let Ok(mut t) = self.last_frame_instant.write() {
            *t = Instant::now();
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
            if let Some(ref mut sv) = self.smart_volume {
                sv.process_slice(buffer);
            }
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

    /// Processes stereo audio buffers (left and right channels) in-place using stereo-linked dynamics.
    /// Returns (Peak dBFS, RMS dBFS).
    pub fn process_stereo_frame(&mut self, left: &mut [f32], right: &mut [f32]) -> (f32, f32) {
        if !self.active.load(Ordering::Relaxed) || (left.is_empty() && right.is_empty()) {
            return (-100.0, -100.0);
        }

        if let Ok(mut t) = self.last_frame_instant.write() {
            *t = Instant::now();
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
                comp.process_stereo(left, right);
            }
            if let Some(ref mut lim) = self.limiter {
                lim.process_stereo(left, right);
            }
        } else {
            self.eq.process(left);
            self.eq_r.process(right);
            if let Some(ref mut sp) = self.spatial {
                sp.process_stereo(left, right);
            }
            if let Some(ref mut sv) = self.smart_volume {
                sv.process_stereo(left, right);
            }
            if let Some(ref mut lim) = self.limiter {
                lim.process_stereo(left, right);
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
        if let Some(ref mut sp) = self.spatial {
            sp.reset();
        }
        if let Some(ref mut sv) = self.smart_volume {
            sv.reset();
        }
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

struct StreamWorkerHandle {
    children: Arc<Mutex<Vec<Child>>>,
    thread: Option<JoinHandle<()>>,
}

/// Real-time in-process audio DSP engine coordinating all channel workers and streams.
pub struct DspEngine {
    /// Active channel DSP workers mapped by channel ID.
    pub workers: HashMap<ChannelId, DspChannelWorker>,
    /// Per-channel routing and mix states.
    pub routing_state: Arc<RwLock<HashMap<ChannelId, ChannelRoutingState>>>,
    /// Whether streamer mode dual-bus is enabled.
    pub streamer_mode: Arc<AtomicBool>,
    mic_worker_handle: Option<StreamWorkerHandle>,
    playback_worker_handles: HashMap<ChannelId, StreamWorkerHandle>,
}

impl Default for DspEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DspEngine {
    /// Creates a new DSP engine initialized with default channel workers and routing states.
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

        let mut routing = HashMap::new();
        for (channel, preset) in channels {
            workers.insert(
                channel,
                DspChannelWorker::new(channel, sample_rate, preset),
            );
            routing.insert(channel, ChannelRoutingState::default());
        }

        Self {
            workers,
            routing_state: Arc::new(RwLock::new(routing)),
            streamer_mode: Arc::new(AtomicBool::new(false)),
            mic_worker_handle: None,
            playback_worker_handles: HashMap::new(),
        }
    }

    /// Sets the full routing state for a specific channel.
    pub fn set_channel_routing(&self, channel: ChannelId, routing: ChannelRoutingState) {
        if let Ok(mut map) = self.routing_state.write() {
            map.insert(channel, routing);
        }
    }

    /// Sets the volume fader level (0 ..= 150) for a channel.
    pub fn set_channel_volume(&self, channel: ChannelId, volume: u8) {
        if let Ok(mut map) = self.routing_state.write() {
            let entry = map.entry(channel).or_default();
            entry.volume = volume;
        }
    }

    /// Sets the mute status for a channel.
    pub fn set_channel_mute(&self, channel: ChannelId, muted: bool) {
        if let Ok(mut map) = self.routing_state.write() {
            let entry = map.entry(channel).or_default();
            entry.muted = muted;
        }
    }

    /// Sets personal headphone mix routing for a channel.
    pub fn set_personal_mix(&self, channel: ChannelId, enabled: bool) {
        if let Ok(mut map) = self.routing_state.write() {
            let entry = map.entry(channel).or_default();
            entry.personal_mix = enabled;
        }
    }

    /// Sets stream audience mix routing for a channel.
    pub fn set_stream_mix(&self, channel: ChannelId, enabled: bool) {
        if let Ok(mut map) = self.routing_state.write() {
            let entry = map.entry(channel).or_default();
            entry.stream_mix = enabled;
        }
    }

    /// Sets dynamic streamer dual-bus mode.
    pub fn set_streamer_mode(&self, enabled: bool) {
        self.streamer_mode.store(enabled, Ordering::Relaxed);
    }

    /// Returns an instantaneous snapshot of volume meters across all channels with decay for idle channels.
    #[must_use]
    pub fn sample_meters(&self) -> HashMap<ChannelId, ChannelMeter> {
        let now = Instant::now();
        let routing_guard = self.routing_state.read().ok();
        let is_master_muted = routing_guard
            .as_ref()
            .and_then(|r| r.get(&ChannelId::Master))
            .map(|s| s.muted)
            .unwrap_or(false);

        let mut meters = HashMap::with_capacity(self.workers.len());
        let mut sum_lin_peak = 0.0_f32;
        let mut sum_lin_rms = 0.0_f32;

        for (&channel, worker) in &self.workers {
            let elapsed = worker
                .last_frame_instant
                .read()
                .map(|t| now.saturating_duration_since(*t))
                .unwrap_or(Duration::from_secs(1));

            let ch_routing = routing_guard
                .as_ref()
                .and_then(|r| r.get(&channel))
                .copied()
                .unwrap_or_default();

            let mut m = worker.get_meter();

            // If channel is idle (>80ms without frames), smoothly decay meters towards -100 dBFS
            if elapsed > Duration::from_millis(80) {
                let decay_step = 6.0_f32;
                m.peak_dbfs = (m.peak_dbfs - decay_step).max(-100.0);
                m.rms_dbfs = (m.rms_dbfs - decay_step).max(-100.0);
                if let Ok(mut guard) = worker.meter.write() {
                    *guard = m;
                }
            }

            // Mic mute forces mic meter to -100 dBFS
            if channel == ChannelId::Mic && ch_routing.muted {
                m.peak_dbfs = -100.0;
                m.rms_dbfs = -100.0;
            }

            meters.insert(channel, m);

            // Master meter: true acoustic energy sum of active personal-mix playback channels
            if matches!(
                channel,
                ChannelId::Game | ChannelId::Chat | ChannelId::Media | ChannelId::Aux
            ) && ch_routing.personal_mix
                && !ch_routing.muted
                && !is_master_muted
            {
                if m.peak_dbfs > -99.0 {
                    sum_lin_peak += 10.0_f32.powf(m.peak_dbfs / 10.0);
                }
                if m.rms_dbfs > -99.0 {
                    sum_lin_rms += 10.0_f32.powf(m.rms_dbfs / 10.0);
                }
            }
        }

        let master_peak = if sum_lin_peak > 0.0 {
            (10.0 * sum_lin_peak.log10()).clamp(-100.0, 0.0)
        } else {
            -100.0
        };
        let master_rms = if sum_lin_rms > 0.0 {
            (10.0 * sum_lin_rms.log10()).clamp(-100.0, 0.0)
        } else {
            -100.0
        };

        meters.insert(
            ChannelId::Master,
            ChannelMeter {
                peak_dbfs: master_peak,
                rms_dbfs: master_rms,
            },
        );

        meters
    }

    /// Updates the active audio preset for a specific channel.
    pub fn update_channel_preset(&mut self, channel: ChannelId, preset: &Preset) {
        if let Some(worker) = self.workers.get_mut(&channel) {
            worker.update_preset(preset);
        }
    }

    /// Spawns stereo audio capture & playback workers for a playback channel (`Game`, `Chat`, `Media`, `Aux`)
    /// with supervisor auto-restart, post-DSP channel fader multiplier, master limiter, personal mix,
    /// and dynamic stream mix.
    pub fn spawn_playback_worker(
        &mut self,
        channel: ChannelId,
        monitor_source: &str,
        target_sink: &str,
    ) {
        self.stop_playback_worker(channel);

        let worker = match self.workers.get(&channel) {
            Some(w) => w,
            None => return,
        };

        let active = worker.active.clone();
        let preset_arc = worker.preset.clone();
        let meter_arc = worker.meter.clone();
        let routing_state_arc = self.routing_state.clone();
        let streamer_mode_arc = self.streamer_mode.clone();
        active.store(true, Ordering::Relaxed);

        let children_holder = Arc::new(Mutex::new(Vec::new()));
        let children_for_thread = children_holder.clone();

        let monitor_source_str = monitor_source.to_string();
        let target_sink_str = target_sink.to_string();

        let thread = std::thread::Builder::new()
            .name(format!("voicegg-{channel}-worker"))
            .spawn(move || {
                let initial_preset = preset_arc.read().map(|p| p.clone()).unwrap_or_default();
                let mut last_preset = initial_preset.clone();
                let mut dsp = DspChannelWorker::new(channel, 48000.0, initial_preset);
                let mut master_limiter = Limiter::new(48000.0, -0.5, 20.0);

                // 480 stereo frames (10ms at 48kHz) = 960 f32 samples = 3840 bytes
                let mut buf_bytes = [0u8; 960 * 4];
                let silence_bytes = [0u8; 960 * 4];
                let mut left = [0.0f32; 480];
                let mut right = [0.0f32; 480];

                while active.load(Ordering::Relaxed) {
                    let mut cap_cmd = Command::new("parec");
                    cap_cmd.args([
                        "--format=float32le",
                        "--rate=48000",
                        "--channels=2",
                        "--raw",
                        "--latency-msec=5",
                        "--device",
                        &monitor_source_str,
                        "--property=media.role=filter",
                        &format!("--property=application.name=VoiceGG {channel} Capture"),
                        "--property=node.passive=true",
                    ]);
                    cap_cmd.stdout(Stdio::piped()).stderr(Stdio::null());

                    let mut play_cmd = Command::new("pacat");
                    play_cmd.args([
                        "--playback",
                        "--format=float32le",
                        "--rate=48000",
                        "--channels=2",
                        "--raw",
                        "--latency-msec=5",
                        "--device",
                        &target_sink_str,
                        "--property=media.role=filter",
                        &format!("--property=application.name=VoiceGG {channel} DSP"),
                        "--property=node.passive=true",
                    ]);
                    play_cmd.stdin(Stdio::piped()).stderr(Stdio::null());

                    let cap_res = cap_cmd.spawn();
                    let play_res = play_cmd.spawn();

                    let (mut cap_child, mut play_child) = match (cap_res, play_res) {
                        (Ok(c), Ok(p)) => (c, p),
                        (cap_r, play_r) => {
                            if let Ok(mut c) = cap_r {
                                let _ = c.kill();
                                let _ = c.wait();
                            }
                            if let Ok(mut p) = play_r {
                                let _ = p.kill();
                                let _ = p.wait();
                            }
                            tracing::warn!(
                                "parec/pacat unavailable or failed for playback channel {channel}; backing off"
                            );
                            std::thread::sleep(Duration::from_millis(200));
                            continue;
                        }
                    };

                    let mut cap_stdout = cap_child.stdout.take();
                    let mut play_stdin = play_child.stdin.take();

                    {
                        if let Ok(mut guard) = children_for_thread.lock() {
                            guard.clear();
                            guard.push(cap_child);
                            guard.push(play_child);
                        }
                    }

                    let mut stream_writer: Option<std::process::ChildStdin> = None;

                    // Inner I/O stream processing loop
                    while active.load(Ordering::Relaxed) {
                        if let Ok(p) = preset_arc.try_read() {
                            if *p != last_preset {
                                dsp.update_preset(&p);
                                last_preset = p.clone();
                            }
                        }

                        if let Some(ref mut reader) = cap_stdout {
                            if reader.read_exact(&mut buf_bytes).is_err() {
                                break;
                            }
                        } else {
                            break;
                        }

                        let chunks = buf_bytes.as_chunks::<4>().0;
                        for i in 0..480 {
                            left[i] = f32::from_le_bytes(chunks[i * 2]);
                            right[i] = f32::from_le_bytes(chunks[i * 2 + 1]);
                        }

                        // 1. Process DSP
                        let (peak, rms) = dsp.process_stereo_frame(&mut left, &mut right);
                        if let Ok(mut m) = meter_arc.write() {
                            m.peak_dbfs = peak;
                            m.rms_dbfs = rms;
                        }

                        // 2. Query channel routing state
                        let routing = routing_state_arc
                            .read()
                            .ok()
                            .and_then(|m| m.get(&channel).copied())
                            .unwrap_or_default();

                        // 3. Post-DSP channel fader volume multiplier & master brickwall limiter
                        let fader_gain = if routing.muted {
                            0.0
                        } else {
                            routing.volume as f32 / 100.0
                        };
                        for i in 0..480 {
                            left[i] *= fader_gain;
                            right[i] *= fader_gain;
                        }
                        master_limiter.process_stereo(&mut left, &mut right);

                        // 4. Encode fader-scaled audio buffer
                        for i in 0..480 {
                            let bl = left[i].to_le_bytes();
                            let br = right[i].to_le_bytes();
                            let base = i * 8;
                            buf_bytes[base..base + 4].copy_from_slice(&bl);
                            buf_bytes[base + 4..base + 8].copy_from_slice(&br);
                        }

                        // 5. Personal mix output (to voicegg_sink_master)
                        // If personal_mix is false or channel muted, write silence to maintain clock
                        let personal_buf = if routing.personal_mix && !routing.muted {
                            &buf_bytes
                        } else {
                            &silence_bytes
                        };

                        if let Some(ref mut writer) = play_stdin {
                            if writer.write_all(personal_buf).is_err() {
                                break;
                            }
                            let _ = writer.flush();
                        } else {
                            break;
                        }

                        // 6. Stream mix output (to voicegg_source_stream_mix when streamer_mode is active)
                        let is_streamer = streamer_mode_arc.load(Ordering::Relaxed);
                        if is_streamer {
                            if stream_writer.is_none() {
                                let mut stream_cmd = Command::new("pacat");
                                stream_cmd.args([
                                    "--playback",
                                    "--format=float32le",
                                    "--rate=48000",
                                    "--channels=2",
                                    "--raw",
                                    "--latency-msec=5",
                                    "--device",
                                    "voicegg_stream_mix_sink",
                                    "--property=media.role=filter",
                                    &format!("--property=application.name=VoiceGG {channel} StreamMix"),
                                    "--property=node.passive=true",
                                ]);
                                stream_cmd.stdin(Stdio::piped()).stderr(Stdio::null());
                                if let Ok(mut s_child) = stream_cmd.spawn() {
                                    stream_writer = s_child.stdin.take();
                                    if let Ok(mut guard) = children_for_thread.lock() {
                                        guard.push(s_child);
                                    }
                                }
                            }

                            if let Some(ref mut sw) = stream_writer {
                                let stream_buf = if routing.stream_mix && !routing.muted {
                                    &buf_bytes
                                } else {
                                    &silence_bytes
                                };
                                if sw.write_all(stream_buf).is_err() {
                                    stream_writer = None;
                                } else {
                                    let _ = sw.flush();
                                }
                            }
                        } else if stream_writer.is_some() {
                            stream_writer = None;
                        }
                    }

                    // Clean up processes on EOF or disconnection before retry
                    if let Ok(mut guard) = children_for_thread.lock() {
                        for child in guard.iter_mut() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                        guard.clear();
                    }

                    if !active.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            })
            .ok();

        self.playback_worker_handles.insert(
            channel,
            StreamWorkerHandle {
                children: children_holder,
                thread,
            },
        );
    }

    /// Stops a single playback channel worker and terminates its child processes.
    pub fn stop_playback_worker(&mut self, channel: ChannelId) {
        if let Some(mut handle) = self.playback_worker_handles.remove(&channel) {
            if let Some(worker) = self.workers.get(&channel) {
                worker.active.store(false, Ordering::Relaxed);
            }
            if let Ok(mut guard) = handle.children.lock() {
                for child in guard.iter_mut() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                guard.clear();
            }
            if let Some(th) = handle.thread.take() {
                let _ = th.join();
            }
        }
    }

    /// Stops all active playback channel workers (`Game`, `Chat`, `Media`, `Aux`).
    pub fn stop_playback_workers(&mut self) {
        let channels: Vec<ChannelId> = self.playback_worker_handles.keys().copied().collect();
        for ch in channels {
            self.stop_playback_worker(ch);
        }
    }

    /// Spawns audio capture & playback workers for Mic using `parec` and `pacat` float32 streams
    /// with supervisor auto-restart, mute silence guard, and dynamic stream mix.
    pub fn spawn_mic_worker(&mut self, source_device: Option<&str>, target_sink: Option<&str>) {
        self.stop_mic_worker();

        let mic_worker = match self.workers.get(&ChannelId::Mic) {
            Some(w) => w,
            None => return,
        };

        let active = mic_worker.active.clone();
        let preset_arc = mic_worker.preset.clone();
        let meter_arc = mic_worker.meter.clone();
        let routing_state_arc = self.routing_state.clone();
        let streamer_mode_arc = self.streamer_mode.clone();
        active.store(true, Ordering::Relaxed);

        let children_holder = Arc::new(Mutex::new(Vec::new()));
        let children_for_thread = children_holder.clone();

        let source_device_opt = source_device.map(|s| s.to_string());
        let target_sink_str = target_sink.unwrap_or("voicegg_mic_sink").to_string();

        let thread = std::thread::Builder::new()
            .name("voicegg-mic-worker".to_string())
            .spawn(move || {
                let initial_preset = preset_arc.read().map(|p| p.clone()).unwrap_or_default();
                let mut last_preset = initial_preset.clone();
                let mut dsp = DspChannelWorker::new(ChannelId::Mic, 48000.0, initial_preset);
                let mut mic_safety_limiter = Limiter::new(48000.0, -0.5, 20.0);

                let mut buf_bytes = [0u8; 480 * 4];
                let silence_bytes = [0u8; 480 * 4];
                let mut samples = [0.0f32; 480];
                let mut mic_stereo_bytes = [0u8; 480 * 2 * 4];
                let silence_stereo_bytes = [0u8; 480 * 2 * 4];

                while active.load(Ordering::Relaxed) {
                    let mut cap_cmd = Command::new("parec");
                    cap_cmd.args([
                        "--format=float32le",
                        "--rate=48000",
                        "--channels=1",
                        "--raw",
                        "--latency-msec=5",
                        "--property=media.role=filter",
                        "--property=application.name=VoiceGG Mic Capture",
                        "--property=node.passive=true",
                    ]);
                    if let Some(ref src) = source_device_opt {
                        if !src.is_empty() && src != "default" {
                            cap_cmd.args(["--device", src]);
                        }
                    }
                    cap_cmd.stdout(Stdio::piped()).stderr(Stdio::null());

                    let mut play_cmd = Command::new("pacat");
                    play_cmd.args([
                        "--playback",
                        "--format=float32le",
                        "--rate=48000",
                        "--channels=1",
                        "--raw",
                        "--latency-msec=5",
                        "--device",
                        &target_sink_str,
                        "--property=media.role=filter",
                        "--property=application.name=VoiceGG Mic DSP",
                        "--property=node.passive=true",
                    ]);
                    play_cmd.stdin(Stdio::piped()).stderr(Stdio::null());

                    let cap_res = cap_cmd.spawn();
                    let play_res = play_cmd.spawn();

                    let (mut cap_child, mut play_child) = match (cap_res, play_res) {
                        (Ok(c), Ok(p)) => (c, p),
                        (cap_r, play_r) => {
                            if let Ok(mut c) = cap_r {
                                let _ = c.kill();
                                let _ = c.wait();
                            }
                            if let Ok(mut p) = play_r {
                                let _ = p.kill();
                                let _ = p.wait();
                            }
                            tracing::warn!(
                                "parec/pacat unavailable or failed for Mic; running fallback"
                            );
                            let mut buffer = [0.0f32; 480];
                            while active.load(Ordering::Relaxed) {
                                if let Ok(p) = preset_arc.try_read() {
                                    if *p != last_preset {
                                        dsp.update_preset(&p);
                                        last_preset = p.clone();
                                    }
                                }
                                let (peak, rms) = dsp.process_mono_frame(&mut buffer);
                                if let Ok(mut m) = meter_arc.write() {
                                    m.peak_dbfs = peak;
                                    m.rms_dbfs = rms;
                                }
                                std::thread::sleep(Duration::from_millis(10));
                            }
                            break;
                        }
                    };

                    let mut cap_stdout = cap_child.stdout.take();
                    let mut play_stdin = play_child.stdin.take();

                    {
                        if let Ok(mut guard) = children_for_thread.lock() {
                            guard.clear();
                            guard.push(cap_child);
                            guard.push(play_child);
                        }
                    }

                    let mut stream_writer: Option<std::process::ChildStdin> = None;

                    while active.load(Ordering::Relaxed) {
                        if let Ok(p) = preset_arc.try_read() {
                            if *p != last_preset {
                                dsp.update_preset(&p);
                                last_preset = p.clone();
                            }
                        }

                        if let Some(ref mut reader) = cap_stdout {
                            if reader.read_exact(&mut buf_bytes).is_err() {
                                break;
                            }
                        } else {
                            break;
                        }

                        for (i, chunk) in buf_bytes.as_chunks::<4>().0.iter().enumerate() {
                            samples[i] = f32::from_le_bytes(*chunk);
                        }

                        // 1. Process DSP
                        let (peak, rms) = dsp.process_mono_frame(&mut samples);
                        if let Ok(mut m) = meter_arc.write() {
                            m.peak_dbfs = peak;
                            m.rms_dbfs = rms;
                        }

                        // 2. Query routing state
                        let routing = routing_state_arc
                            .read()
                            .ok()
                            .and_then(|m| m.get(&ChannelId::Mic).copied())
                            .unwrap_or_default();

                        // 3. Post-DSP fader multiplier & safety limiter
                        let fader_gain = if routing.muted {
                            0.0
                        } else {
                            routing.volume as f32 / 100.0
                        };
                        for s in samples.iter_mut() {
                            *s *= fader_gain;
                        }
                        mic_safety_limiter.process_slice(&mut samples);

                        // 4. Encode mono samples
                        for (i, &s) in samples.iter().enumerate() {
                            let b = s.to_le_bytes();
                            buf_bytes[i * 4..i * 4 + 4].copy_from_slice(&b);
                        }

                        // 5. Write to voicegg_source_mic (silence if muted)
                        let play_buf = if routing.muted {
                            &silence_bytes
                        } else {
                            &buf_bytes
                        };
                        if let Some(ref mut writer) = play_stdin {
                            if writer.write_all(play_buf).is_err() {
                                break;
                            }
                            let _ = writer.flush();
                        } else {
                            break;
                        }

                        // 6. Write to stream mix if active
                        let is_streamer = streamer_mode_arc.load(Ordering::Relaxed);
                        if is_streamer {
                            if stream_writer.is_none() {
                                let mut stream_cmd = Command::new("pacat");
                                stream_cmd.args([
                                    "--playback",
                                    "--format=float32le",
                                    "--rate=48000",
                                    "--channels=2",
                                    "--raw",
                                    "--latency-msec=5",
                                    "--device",
                                    "voicegg_stream_mix_sink",
                                    "--property=media.role=filter",
                                    "--property=application.name=VoiceGG Mic StreamMix",
                                    "--property=node.passive=true",
                                ]);
                                stream_cmd.stdin(Stdio::piped()).stderr(Stdio::null());
                                if let Ok(mut s_child) = stream_cmd.spawn() {
                                    stream_writer = s_child.stdin.take();
                                    if let Ok(mut guard) = children_for_thread.lock() {
                                        guard.push(s_child);
                                    }
                                }
                            }

                            if let Some(ref mut sw) = stream_writer {
                                let mic_stream_buf = if routing.stream_mix && !routing.muted {
                                    for (i, &s) in samples.iter().enumerate() {
                                        let b = s.to_le_bytes();
                                        let base = i * 8;
                                        mic_stereo_bytes[base..base + 4].copy_from_slice(&b);
                                        mic_stereo_bytes[base + 4..base + 8].copy_from_slice(&b);
                                    }
                                    &mic_stereo_bytes
                                } else {
                                    &silence_stereo_bytes
                                };
                                if sw.write_all(mic_stream_buf).is_err() {
                                    stream_writer = None;
                                } else {
                                    let _ = sw.flush();
                                }
                            }
                        } else if stream_writer.is_some() {
                            stream_writer = None;
                        }
                    }

                    if let Ok(mut guard) = children_for_thread.lock() {
                        for child in guard.iter_mut() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                        guard.clear();
                    }

                    if !active.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            })
            .ok();

        self.mic_worker_handle = Some(StreamWorkerHandle {
            children: children_holder,
            thread,
        });
    }

    /// Stops the microphone background capture and playback worker.
    pub fn stop_mic_worker(&mut self) {
        if let Some(mut handle) = self.mic_worker_handle.take() {
            if let Some(worker) = self.workers.get(&ChannelId::Mic) {
                worker.active.store(false, Ordering::Relaxed);
            }
            if let Ok(mut guard) = handle.children.lock() {
                for child in guard.iter_mut() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                guard.clear();
            }
            if let Some(th) = handle.thread.take() {
                let _ = th.join();
            }
        }
    }

    /// Stops all channel workers, active processes, and background threads gracefully.
    pub fn stop_all(&mut self) {
        self.stop_mic_worker();
        self.stop_playback_workers();
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
    use voicegg_core::{SmartVolumeConfig, SpatialConfig};

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
    fn test_channel_worker_stereo_processing_with_spatial_and_smart_volume() {
        let mut preset = Preset::get_preset_by_id("game_flat").unwrap_or_default();
        preset.spatial = Some(SpatialConfig {
            enabled: true,
            immersion_pct: 75,
        });
        preset.smart_volume = Some(SmartVolumeConfig {
            enabled: true,
            mode: "loud".to_string(),
        });
        let mut worker = DspChannelWorker::new(ChannelId::Game, 48000.0, preset);

        let mut left = vec![0.2_f32; 256];
        let mut right = vec![-0.1_f32; 256];
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

        let p = engine
            .workers
            .get(&ChannelId::Game)
            .unwrap()
            .preset
            .read()
            .unwrap();
        assert_eq!(p.id, "game_cs2");
    }

    #[test]
    fn test_dsp_engine_spawn_and_stop_all() {
        let mut engine = DspEngine::new();
        engine.spawn_mic_worker(None, None);

        // Give worker thread a moment to initialize
        std::thread::sleep(Duration::from_millis(30));

        engine.stop_all();
        assert!(!engine
            .workers
            .get(&ChannelId::Mic)
            .unwrap()
            .active
            .load(Ordering::Relaxed));
    }

    #[test]
    fn test_dsp_engine_routing_state_and_faders() {
        let engine = DspEngine::new();

        engine.set_channel_volume(ChannelId::Game, 120);
        engine.set_channel_mute(ChannelId::Chat, true);
        engine.set_personal_mix(ChannelId::Media, false);
        engine.set_stream_mix(ChannelId::Aux, false);
        engine.set_streamer_mode(true);

        assert!(engine.streamer_mode.load(Ordering::Relaxed));

        let r = engine.routing_state.read().unwrap();
        assert_eq!(r.get(&ChannelId::Game).unwrap().volume, 120);
        assert!(r.get(&ChannelId::Chat).unwrap().muted);
        assert!(!r.get(&ChannelId::Media).unwrap().personal_mix);
        assert!(!r.get(&ChannelId::Aux).unwrap().stream_mix);
    }

    #[test]
    fn test_dsp_engine_sample_meters_idle_decay_and_mute() {
        let engine = DspEngine::new();

        // Simulate Mic with signal
        {
            let mic_worker = engine.workers.get(&ChannelId::Mic).unwrap();
            let mut m = mic_worker.meter.write().unwrap();
            m.peak_dbfs = -12.0;
            m.rms_dbfs = -16.0;
        }

        // When not muted, sample_meters reports the level
        let meters = engine.sample_meters();
        assert_eq!(meters.get(&ChannelId::Mic).unwrap().peak_dbfs, -12.0);

        // Mute Mic -> meter clamped to -100.0 dBFS
        engine.set_channel_mute(ChannelId::Mic, true);
        let meters_muted = engine.sample_meters();
        assert_eq!(meters_muted.get(&ChannelId::Mic).unwrap().peak_dbfs, -100.0);
        assert_eq!(meters_muted.get(&ChannelId::Mic).unwrap().rms_dbfs, -100.0);

        // Simulate idle playback channel (>80ms without frames)
        {
            let game_worker = engine.workers.get(&ChannelId::Game).unwrap();
            let mut m = game_worker.meter.write().unwrap();
            m.peak_dbfs = -10.0;
            m.rms_dbfs = -14.0;
            let mut t = game_worker.last_frame_instant.write().unwrap();
            *t = Instant::now() - Duration::from_millis(200);
        }

        let meters_decayed = engine.sample_meters();
        let game_meter = meters_decayed.get(&ChannelId::Game).unwrap();
        assert!(
            game_meter.peak_dbfs < -10.0,
            "Expected meter decay on idle, got {}",
            game_meter.peak_dbfs
        );
    }

    #[test]
    fn test_dsp_engine_master_acoustic_energy_sum() {
        let engine = DspEngine::new();

        // Game at -6.0 dBFS
        {
            let game = engine.workers.get(&ChannelId::Game).unwrap();
            let mut m = game.meter.write().unwrap();
            m.peak_dbfs = -6.0;
            m.rms_dbfs = -9.0;
            *game.last_frame_instant.write().unwrap() = Instant::now();
        }

        let meters = engine.sample_meters();
        let master = meters.get(&ChannelId::Master).unwrap();
        assert!((master.peak_dbfs - (-6.0)).abs() < 0.1);

        // Now activate Chat also at -6.0 dBFS
        // Power sum of two -6 dBFS signals is -6 + 3 = -3 dBFS
        {
            let chat = engine.workers.get(&ChannelId::Chat).unwrap();
            let mut m = chat.meter.write().unwrap();
            m.peak_dbfs = -6.0;
            m.rms_dbfs = -9.0;
            *chat.last_frame_instant.write().unwrap() = Instant::now();
        }

        let meters_combined = engine.sample_meters();
        let master_combined = meters_combined.get(&ChannelId::Master).unwrap();
        assert!(
            (master_combined.peak_dbfs - (-3.0)).abs() < 0.1,
            "Expected acoustic sum ~ -3.0 dBFS, got {}",
            master_combined.peak_dbfs
        );

        // If personal_mix is disabled for Chat, Chat is excluded from Master meter
        engine.set_personal_mix(ChannelId::Chat, false);
        let meters_filtered = engine.sample_meters();
        let master_filtered = meters_filtered.get(&ChannelId::Master).unwrap();
        assert!((master_filtered.peak_dbfs - (-6.0)).abs() < 0.1);
    }
}
