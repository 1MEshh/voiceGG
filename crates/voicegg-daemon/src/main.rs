#![forbid(unsafe_code)]

//! VoiceGG Background Daemon.
//!
//! Owns the PipeWire audio graph, runs the game detection engine,
//! auto-routes application streams, and serves client IPC requests.

mod game_detector;

use anyhow::Result;
use game_detector::GameDetector;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use voicegg_core::{ChannelId, VoiceggConfig};
use voicegg_ipc::{IpcRequest, IpcResponse, IpcServer, RequestHandler, SystemStatus};
use voicegg_pw::GraphManager;

struct DaemonHandler {
    graph: Arc<Mutex<GraphManager>>,
    config: Arc<RwLock<VoiceggConfig>>,
    active_game: Arc<RwLock<Option<String>>>,
    dirty: Arc<AtomicBool>,
    mic_monitor_state: Arc<Mutex<(bool, u8)>>,
    shutdown_tx: tokio::sync::mpsc::UnboundedSender<()>,
    client_pids: Arc<RwLock<HashSet<u32>>>,
}

impl RequestHandler for DaemonHandler {
    fn handle_request(&self, req: IpcRequest) -> IpcResponse {
        match req {
            IpcRequest::GetStatus => {
                let cfg = {
                    let lock = self.config.read().unwrap();
                    lock.clone()
                };
                let (devices, streams, caps) = {
                    let gm = self.graph.lock().unwrap();
                    (
                        gm.list_devices().unwrap_or_default(),
                        gm.list_active_streams().unwrap_or_default(),
                        gm.get_capabilities(),
                    )
                };
                let (mic_mon_enabled, mic_mon_vol) = *self.mic_monitor_state.lock().unwrap();
                let game = self.active_game.read().unwrap().clone();
                IpcResponse::Status(Box::new(SystemStatus {
                    config: cfg,
                    devices,
                    streams,
                    active_game: game,
                    capabilities: caps,
                    mic_monitor_enabled: mic_mon_enabled,
                    mic_monitor_volume: mic_mon_vol,
                }))
            }
            IpcRequest::SetVolume { channel, volume } => {
                let volume = volume.min(150);
                let effective_vol = {
                    let mut cfg = self.config.write().unwrap();
                    cfg.volumes.insert(channel, volume);
                    cfg.effective_volume(channel)
                };
                {
                    let gm = self.graph.lock().unwrap();
                    let _ = gm.set_channel_volume(channel, effective_vol);
                    gm.dsp_engine.set_channel_volume(channel, effective_vol);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!(
                    "Volume for {} set to {}% (effective: {}%)",
                    channel,
                    volume,
                    effective_vol
                );
                IpcResponse::Success
            }
            IpcRequest::SetMute { channel, muted } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.muted.insert(channel, muted);
                }
                {
                    let gm = self.graph.lock().unwrap();
                    let _ = gm.set_channel_mute(channel, muted);
                    gm.dsp_engine.set_channel_mute(channel, muted);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Mute for {} set to {}", channel, muted);
                IpcResponse::Success
            }
            IpcRequest::SetChatMix { value } => {
                let (game_vol, chat_vol) = {
                    let mut cfg = self.config.write().unwrap();
                    cfg.chatmix = value.clamp(-100, 100);
                    (
                        cfg.effective_volume(ChannelId::Game),
                        cfg.effective_volume(ChannelId::Chat),
                    )
                };
                {
                    let gm = self.graph.lock().unwrap();
                    let _ = gm.set_channel_volume(ChannelId::Game, game_vol);
                    let _ = gm.set_channel_volume(ChannelId::Chat, chat_vol);
                    gm.dsp_engine
                        .set_channel_volume(ChannelId::Game, game_vol);
                    gm.dsp_engine
                        .set_channel_volume(ChannelId::Chat, chat_vol);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!(
                    "ChatMix balance set to {} (game: {}%, chat: {}%)",
                    value,
                    game_vol,
                    chat_vol
                );
                IpcResponse::Success
            }
            IpcRequest::SetPreset { channel, preset } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.active_presets.insert(channel, preset.id.clone());
                    cfg.custom_presets.insert(preset.id.clone(), preset.clone());
                }
                {
                    let mut gm = self.graph.lock().unwrap();
                    if let Err(e) = gm.apply_preset(channel, &preset) {
                        tracing::warn!(
                            "Failed to apply preset '{}' to {}: {e}",
                            preset.name,
                            channel
                        );
                    }
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Preset '{}' applied to {}", preset.name, channel);
                IpcResponse::Success
            }
            IpcRequest::SetMicMonitor { enabled, volume } => {
                let mut gm = self.graph.lock().unwrap();
                match gm.set_mic_monitor(enabled, volume) {
                    Ok(()) => {
                        let mut mon = self.mic_monitor_state.lock().unwrap();
                        *mon = (enabled, volume);
                        tracing::info!("Mic monitor set to enabled={}, volume={}", enabled, volume);
                        IpcResponse::Success
                    }
                    Err(e) => {
                        tracing::error!("Failed to set mic monitor: {e}");
                        IpcResponse::Error(e.to_string())
                    }
                }
            }
            IpcRequest::GetCapabilities => {
                let gm = self.graph.lock().unwrap();
                IpcResponse::Capabilities(gm.get_capabilities())
            }
            IpcRequest::GetMeters => {
                let gm = self.graph.lock().unwrap();
                let raw_meters = gm.sample_meters();
                let meters = raw_meters
                    .into_iter()
                    .map(|(ch, (peak, rms))| {
                        (
                            ch,
                            voicegg_ipc::ChannelMeter {
                                peak_dbfs: peak,
                                rms_dbfs: rms,
                            },
                        )
                    })
                    .collect();
                IpcResponse::Meters(meters)
            }
            IpcRequest::RouteApp {
                binary_name,
                target_channel,
            } => {
                let raw_trimmed = binary_name.trim();
                let clean_name = std::path::Path::new(raw_trimmed)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(raw_trimmed);
                if clean_name.is_empty()
                    || clean_name.len() > 128
                    || clean_name.contains('\0')
                    || clean_name.chars().any(|c| c.is_control())
                {
                    tracing::warn!(
                        "Rejected invalid or suspicious binary name: {:?}",
                        binary_name
                    );
                    return IpcResponse::Error(format!("Invalid binary name '{}'", binary_name));
                }
                let binary_name = clean_name.to_string();

                tracing::info!(
                    "Routing application '{}' to {}",
                    binary_name,
                    target_channel
                );
                let gm = self.graph.lock().unwrap();
                match gm.route_app_by_name(&binary_name, target_channel) {
                    Ok(()) => {
                        let mut cfg = self.config.write().unwrap();
                        if let Some(existing) = cfg
                            .routing_rules
                            .iter_mut()
                            .find(|r| r.binary_name.eq_ignore_ascii_case(&binary_name))
                        {
                            existing.target_channel = target_channel;
                        } else {
                            cfg.routing_rules.push(voicegg_core::AppRouteRule {
                                binary_name: binary_name.clone(),
                                target_channel,
                                volume: 100,
                            });
                        }
                        self.dirty.store(true, Ordering::Relaxed);
                        IpcResponse::Success
                    }
                    Err(e) => IpcResponse::Error(e.to_string()),
                }
            }
            IpcRequest::GetDevices => {
                let gm = self.graph.lock().unwrap();
                match gm.list_devices() {
                    Ok(devs) => IpcResponse::Devices(devs),
                    Err(e) => IpcResponse::Error(e.to_string()),
                }
            }
            IpcRequest::SetDevice {
                device_type,
                device_name,
            } => {
                let mut gm = self.graph.lock().unwrap();
                let res = match device_type {
                    voicegg_pw::DeviceType::Sink => {
                        let r = gm.set_output_sink(&device_name);
                        if r.is_ok() {
                            let mut cfg = self.config.write().unwrap();
                            cfg.preferred_output_device = Some(device_name.clone());
                            self.dirty.store(true, Ordering::Relaxed);
                        }
                        r
                    }
                    voicegg_pw::DeviceType::Source => {
                        let r = gm.set_input_source(&device_name);
                        if r.is_ok() {
                            let mut cfg = self.config.write().unwrap();
                            cfg.preferred_input_device = Some(device_name.clone());
                            self.dirty.store(true, Ordering::Relaxed);
                        }
                        r
                    }
                };
                match res {
                    Ok(()) => IpcResponse::Success,
                    Err(e) => IpcResponse::Error(e.to_string()),
                }
            }
            IpcRequest::PanicReset => {
                tracing::warn!("PanicReset requested: resetting audio graph to system defaults");
                let mut gm = self.graph.lock().unwrap();
                let _ = gm.teardown_virtual_devices();
                IpcResponse::Success
            }
            IpcRequest::Shutdown => {
                tracing::info!("Shutdown requested via IPC: initiating graceful termination");
                let mut gm = self.graph.lock().unwrap();
                let _ = gm.teardown_virtual_devices();
                let _ = self.shutdown_tx.send(());
                IpcResponse::Success
            }
            IpcRequest::SetLanguage { language } => {
                tracing::info!(language = %language, "Setting application language");
                let mut cfg = self.config.write().unwrap();
                cfg.language = language;
                self.dirty.store(true, Ordering::Relaxed);
                IpcResponse::Success
            }
            IpcRequest::SetAppVolume {
                binary_name,
                volume,
            } => {
                tracing::info!(binary = %binary_name, volume, "Setting application volume");
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.set_app_volume(binary_name.clone(), volume);
                    if let Some(rule) = cfg
                        .routing_rules
                        .iter_mut()
                        .find(|r| r.binary_name.eq_ignore_ascii_case(&binary_name))
                    {
                        rule.volume = volume;
                    }
                }
                self.dirty.store(true, Ordering::Relaxed);
                let gm = self.graph.lock().unwrap();
                let _ = gm.set_app_volume_by_name(&binary_name, volume);
                IpcResponse::Success
            }
            IpcRequest::CyclePreset { channel } => {
                use voicegg_core::{Preset, PresetCategory};
                let all_presets: Vec<Preset> = Preset::builtin_presets()
                    .into_iter()
                    .filter(|p| match channel {
                        ChannelId::Game => p.category == PresetCategory::Game,
                        ChannelId::Chat => p.category == PresetCategory::Chat,
                        ChannelId::Media => p.category == PresetCategory::Media,
                        ChannelId::Mic => p.category == PresetCategory::Mic,
                        _ => true,
                    })
                    .collect();

                if all_presets.is_empty() {
                    IpcResponse::Error("No presets available for channel".to_string())
                } else {
                    let next_preset = {
                        let cfg = self.config.read().unwrap();
                        let current_id = cfg.active_presets.get(&channel);
                        let idx = current_id
                            .and_then(|id| all_presets.iter().position(|p| &p.id == id))
                            .map(|i| (i + 1) % all_presets.len())
                            .unwrap_or(0);
                        all_presets[idx].clone()
                    };

                    {
                        let mut cfg = self.config.write().unwrap();
                        cfg.active_presets
                            .insert(channel, next_preset.id.clone());
                    }
                    {
                        let mut gm = self.graph.lock().unwrap();
                        if let Err(e) = gm.apply_preset(channel, &next_preset) {
                            tracing::warn!("Failed to apply cycled preset: {e}");
                        }
                    }
                    self.dirty.store(true, Ordering::Relaxed);
                    IpcResponse::Success
                }
            }
            IpcRequest::SetStreamerMode { enabled } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.streamer_mode = enabled;
                }
                {
                    let mut gm = self.graph.lock().unwrap();
                    let _ = gm.set_streamer_mode(enabled);
                    gm.dsp_engine.set_streamer_mode(enabled);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Streamer mode set to {}", enabled);
                IpcResponse::Success
            }
            IpcRequest::SetPersonalMix { channel, enabled } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.personal_mix.insert(channel, enabled);
                }
                {
                    let gm = self.graph.lock().unwrap();
                    gm.dsp_engine.set_personal_mix(channel, enabled);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Personal mix for {} set to {}", channel, enabled);
                IpcResponse::Success
            }
            IpcRequest::SetStreamMix { channel, enabled } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.stream_mix.insert(channel, enabled);
                }
                {
                    let gm = self.graph.lock().unwrap();
                    gm.dsp_engine.set_stream_mix(channel, enabled);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Stream mix for {} set to {}", channel, enabled);
                IpcResponse::Success
            }
            IpcRequest::ToggleFavoritePreset { channel, preset_id } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    let list = cfg.favorite_presets.entry(channel).or_default();
                    if let Some(pos) = list.iter().position(|id| id == &preset_id) {
                        list.remove(pos);
                    } else {
                        list.push(preset_id);
                    }
                }
                self.dirty.store(true, Ordering::Relaxed);
                IpcResponse::Success
            }
            IpcRequest::SetChannelColor { channel, hex } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    if let Some(ch_theme) = cfg.theme.channels.get_mut(&channel) {
                        ch_theme.accent_hex = hex.clone();
                    } else {
                        cfg.theme.channels.insert(
                            channel,
                            voicegg_core::ChannelTheme {
                                accent_hex: hex.clone(),
                                secondary_hex: hex.clone(),
                            },
                        );
                    }
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Channel color for {} set to {}", channel, hex);
                IpcResponse::Success
            }
            IpcRequest::SaveCustomPreset { channel, preset } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.custom_presets.insert(preset.id.clone(), preset.clone());
                    cfg.active_presets.insert(channel, preset.id.clone());
                }
                {
                    let mut gm = self.graph.lock().unwrap();
                    let _ = gm.apply_preset(channel, &preset);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Saved custom preset '{}' for {}", preset.name, channel);
                IpcResponse::Success
            }
            IpcRequest::DeleteCustomPreset { preset_id } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.custom_presets.remove(&preset_id);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Deleted custom preset '{}'", preset_id);
                IpcResponse::Success
            }
            IpcRequest::SetQuantum { quantum } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.buffer_quantum = quantum;
                }
                let _ = std::process::Command::new("pw-metadata")
                    .args(["-n", "settings", "0", "clock.force-quantum", &quantum.to_string()])
                    .status();
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Buffer quantum set to {}", quantum);
                IpcResponse::Success
            }
            IpcRequest::RegisterClientPid { pid } => {
                tracing::info!("Registering GUI client PID: {}", pid);
                if let Ok(mut pids) = self.client_pids.write() {
                    pids.insert(pid);
                }
                IpcResponse::Success
            }
            IpcRequest::SetAutoGameDetection { enabled } => {
                tracing::info!("Setting auto game detection: {}", enabled);
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.set_auto_game_detection(enabled);
                }
                self.dirty.store(true, Ordering::Relaxed);
                IpcResponse::Success
            }
            IpcRequest::SetCloseToTray { enabled } => {
                tracing::info!("Setting close-to-tray: {}", enabled);
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.set_close_to_tray(enabled);
                }
                self.dirty.store(true, Ordering::Relaxed);
                IpcResponse::Success
            }
            IpcRequest::SetAppMute { binary_name, muted } => {
                tracing::info!(binary = %binary_name, muted, "Setting application mute");
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.set_app_mute(binary_name.clone(), muted);
                }
                self.dirty.store(true, Ordering::Relaxed);
                let gm = self.graph.lock().unwrap();
                if let Ok(streams) = gm.list_active_streams() {
                    for stream in streams {
                        if stream.binary_name.eq_ignore_ascii_case(&binary_name)
                            || stream.app_name.eq_ignore_ascii_case(&binary_name)
                            || stream.id.to_string() == binary_name
                        {
                            let mute_val = if muted { "1" } else { "0" };
                            let _ = std::process::Command::new("pactl")
                                .env("LC_ALL", "C")
                                .args(["set-sink-input-mute", &stream.id.to_string(), mute_val])
                                .output();
                        }
                    }
                }
                IpcResponse::Success
            }
            IpcRequest::ResetPreset { channel, preset_id } => {
                tracing::info!("Resetting preset '{}' for channel {}", preset_id, channel);
                let factory = voicegg_core::Preset::builtin_presets()
                    .into_iter()
                    .find(|p| p.id.eq_ignore_ascii_case(&preset_id));

                if let Some(preset) = factory {
                    {
                        let mut cfg = self.config.write().unwrap();
                        cfg.custom_presets.remove(&preset_id);
                        cfg.active_presets.insert(channel, preset.id.clone());
                    }
                    {
                        let mut gm = self.graph.lock().unwrap();
                        let _ = gm.apply_preset(channel, &preset);
                    }
                    self.dirty.store(true, Ordering::Relaxed);
                    IpcResponse::Success
                } else {
                    IpcResponse::Error(format!("Unknown built-in preset '{preset_id}'"))
                }
            }
        }
    }
}

/// Automatically classifies an unrouted binary into a recommended channel.
fn smart_classify_app(binary: &str) -> ChannelId {
    let lower = binary.to_ascii_lowercase();

    // Voice & Chat communication
    if lower.contains("discord")
        || lower.contains("vesktop")
        || lower.contains("webcord")
        || lower.contains("mumble")
        || lower.contains("ts3")
        || lower.contains("teamspeak")
        || lower.contains("skype")
        || lower.contains("zoom")
        || lower.contains("slack")
    {
        return ChannelId::Chat;
    }

    // Browsers and Media Players
    if lower.contains("spotify")
        || lower.contains("firefox")
        || lower.contains("chrome")
        || lower.contains("chromium")
        || lower.contains("brave")
        || lower.contains("vlc")
        || lower.contains("mpv")
        || lower.contains("audacious")
        || lower.contains("cider")
    {
        return ChannelId::Media;
    }

    // Default to Game for gaming launchers and wine
    if lower.contains("steam")
        || lower.contains("lutris")
        || lower.contains("heroic")
        || lower.contains("wine")
    {
        return ChannelId::Game;
    }

    // Fallback to Media for audio playback
    ChannelId::Media
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting voicegg-daemon...");

    let mut graph_manager = GraphManager::new();
    graph_manager.init()?;
    graph_manager.setup_virtual_devices()?;

    // Spawn janitor watchdog child with piped stdin.
    // If voicegg-daemon is killed (even with kill -9) or crashes, the OS kernel
    // closes the pipe, causing the janitor to immediately restore system audio.
    let (janitor_sink, janitor_source) = (
        graph_manager.active_output_sink().map(|s| s.to_string()),
        graph_manager.active_input_source().map(|s| s.to_string()),
    );
    let mut janitor_child = {
        let mut cmd = std::process::Command::new("voicegg");
        cmd.arg("janitor-watch");
        if let Some(ref s) = janitor_sink {
            cmd.args(["--phys-sink", s]);
        }
        if let Some(ref s) = janitor_source {
            cmd.args(["--phys-source", s]);
        }
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()
    };

    // Load saved configuration or create default
    let loaded_config = VoiceggConfig::load();
    tracing::info!(
        "Loaded configuration with {} routing rules",
        loaded_config.routing_rules.len()
    );

    // Synchronize initial configuration with audio graph and DSP engine
    for ch in [
        ChannelId::Master,
        ChannelId::Game,
        ChannelId::Chat,
        ChannelId::Media,
        ChannelId::Aux,
        ChannelId::Mic,
    ] {
        let vol = loaded_config.effective_volume(ch);
        let muted = loaded_config.muted.get(&ch).copied().unwrap_or(false);
        let _ = graph_manager.set_channel_volume(ch, vol);
        let _ = graph_manager.set_channel_mute(ch, muted);
        graph_manager.dsp_engine.set_channel_volume(ch, vol);
        graph_manager.dsp_engine.set_channel_mute(ch, muted);
        if let Some(&pm) = loaded_config.personal_mix.get(&ch) {
            graph_manager.dsp_engine.set_personal_mix(ch, pm);
        }
        if let Some(&sm) = loaded_config.stream_mix.get(&ch) {
            graph_manager.dsp_engine.set_stream_mix(ch, sm);
        }
    }
    graph_manager
        .dsp_engine
        .set_streamer_mode(loaded_config.streamer_mode);

    let builtin_presets = voicegg_core::Preset::builtin_presets();
    for (ch, preset_id) in &loaded_config.active_presets {
        let preset = loaded_config
            .custom_presets
            .get(preset_id)
            .cloned()
            .or_else(|| {
                builtin_presets
                    .iter()
                    .find(|p| p.id.eq_ignore_ascii_case(preset_id))
                    .cloned()
            });
        if let Some(p) = preset {
            let _ = graph_manager.apply_preset(*ch, &p);
        }
    }

    if let Some(ref sink) = loaded_config.preferred_output_device {
        let _ = graph_manager.set_output_sink(sink);
    }
    if let Some(ref source) = loaded_config.preferred_input_device {
        let _ = graph_manager.set_input_source(source);
    }

    let graph = Arc::new(Mutex::new(graph_manager));
    let config = Arc::new(RwLock::new(loaded_config));
    let active_game = Arc::new(RwLock::new(None));
    let dirty = Arc::new(AtomicBool::new(false));
    let mic_monitor_state = Arc::new(Mutex::new((false, 75_u8)));
    let client_pids = Arc::new(RwLock::new(HashSet::new()));

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::unbounded_channel();

    let handler = Arc::new(DaemonHandler {
        graph: Arc::clone(&graph),
        config: Arc::clone(&config),
        active_game: Arc::clone(&active_game),
        dirty: Arc::clone(&dirty),
        mic_monitor_state,
        shutdown_tx,
        client_pids,
    });

    let socket_path = voicegg_ipc::default_socket_path();
    tracing::info!("Binding IPC socket at {:?}", socket_path);
    let server = IpcServer::bind(&socket_path)?;

    tokio::spawn(async move {
        if let Err(e) = server.run(handler).await {
            tracing::error!("IPC server encountered error: {}", e);
        }
    });

    // Background router and game detector loop
    let bg_graph = Arc::clone(&graph);
    let bg_config = Arc::clone(&config);
    let bg_active_game = Arc::clone(&active_game);
    let bg_dirty = Arc::clone(&dirty);
    let detector = GameDetector::new();

    tokio::spawn(async move {
        let mut routed_stream_ids = HashSet::<u32>::new();
        let mut previous_game_preset: Option<String> = None;

        loop {
            sleep(Duration::from_millis(1500)).await;

            // 1. Process active stream discovery & auto-routing
            {
                let gm = bg_graph.lock().unwrap();
                if let Ok(streams) = gm.list_active_streams() {
                    let active_stream_ids: HashSet<u32> =
                        streams.iter().map(|s| s.id).collect();
                    routed_stream_ids.retain(|id| active_stream_ids.contains(id));

                    let cfg = bg_config.read().unwrap();
                    for stream in streams {
                        let name_lower = stream.binary_name.to_ascii_lowercase();
                        let app_lower = stream.app_name.to_ascii_lowercase();
                        if name_lower.contains("loopback")
                            || app_lower.contains("loopback")
                            || name_lower == "pacat"
                            || name_lower == "parec"
                            || name_lower.starts_with("voicegg")
                            || app_lower.starts_with("voicegg")
                        {
                            continue;
                        }

                        // Route every newly discovered stream ID, or streams on Master/None
                        let is_new = routed_stream_ids.insert(stream.id);
                        if is_new
                            || stream.current_channel.is_none()
                            || stream.current_channel == Some(ChannelId::Master)
                        {
                            // Find explicit rule matching binary_name or app_name
                            let target =
                                if let Some(rule) = cfg.routing_rules.iter().find(|r| {
                                    r.binary_name.eq_ignore_ascii_case(&stream.binary_name)
                                        || r.binary_name.eq_ignore_ascii_case(&stream.app_name)
                                }) {
                                    rule.target_channel
                                } else {
                                    smart_classify_app(&format!(
                                        "{} {}",
                                        stream.binary_name, stream.app_name
                                    ))
                                };

                            if stream.current_channel != Some(target) {
                                let _ = gm.route_stream(stream.id, target);
                            }

                            let vol = cfg.get_app_volume(&stream.binary_name);
                            if vol != 100 {
                                let _ = gm.set_stream_volume(stream.id, vol);
                            }

                            if cfg.get_app_mute(&stream.binary_name)
                                || cfg.get_app_mute(&stream.app_name)
                            {
                                let _ = std::process::Command::new("pactl")
                                    .env("LC_ALL", "C")
                                    .args([
                                        "set-sink-input-mute",
                                        &stream.id.to_string(),
                                        "1",
                                    ])
                                    .output();
                            }
                        }
                    }
                }
            }

            // 2. Scan for running games
            let auto_detect = {
                let cfg = bg_config.read().unwrap();
                cfg.auto_game_detection
            };

            if auto_detect {
                let detected = detector.scan_processes();
                let mut current = bg_active_game.write().unwrap();

                match detected {
                    Some(game) => {
                        if current.as_deref() != Some(&game.name) {
                            tracing::info!(
                                "Game detected: '{}' (applying preset '{}')",
                                game.name,
                                game.preset_id
                            );
                            *current = Some(game.name);

                            // Save user's current game preset before switching
                            if previous_game_preset.is_none() {
                                let prev = {
                                    let cfg = bg_config.read().unwrap();
                                    cfg.active_presets
                                        .get(&ChannelId::Game)
                                        .cloned()
                                        .unwrap_or_else(|| "game_flat".to_string())
                                };
                                previous_game_preset = Some(prev);
                            }

                            let custom_preset = {
                                let mut cfg = bg_config.write().unwrap();
                                cfg.active_presets
                                    .insert(ChannelId::Game, game.preset_id.clone());
                                cfg.custom_presets.get(&game.preset_id).cloned()
                            };
                            bg_dirty.store(true, Ordering::Relaxed);

                            // Route game process stream to Game channel
                            let mut gm = bg_graph.lock().unwrap();
                            let _ = gm.route_app_by_name(&game.matched_binary, ChannelId::Game);
                            let builtin = voicegg_core::Preset::builtin_presets();
                            if let Some(p) = custom_preset.or_else(|| {
                                builtin
                                    .into_iter()
                                    .find(|p| p.id.eq_ignore_ascii_case(&game.preset_id))
                            }) {
                                let _ = gm.apply_preset(ChannelId::Game, &p);
                            }
                        }
                    }
                    None => {
                        if current.is_some() {
                            tracing::info!("Game exited. Reverting Game channel preset.");
                            *current = None;
                            let restore_preset_id = previous_game_preset
                                .take()
                                .unwrap_or_else(|| "game_flat".to_string());
                            let (preset_to_apply, custom) = {
                                let mut cfg = bg_config.write().unwrap();
                                cfg.active_presets
                                    .insert(ChannelId::Game, restore_preset_id.clone());
                                let c = cfg.custom_presets.get(&restore_preset_id).cloned();
                                (restore_preset_id, c)
                            };
                            bg_dirty.store(true, Ordering::Relaxed);
                            let mut gm = bg_graph.lock().unwrap();
                            let builtin = voicegg_core::Preset::builtin_presets();
                            if let Some(p) = custom.or_else(|| {
                                builtin
                                    .into_iter()
                                    .find(|p| p.id.eq_ignore_ascii_case(&preset_to_apply))
                            }) {
                                let _ = gm.apply_preset(ChannelId::Game, &p);
                            }
                        }
                    }
                }
            }

            // 3. Debounced config persistence
            if bg_dirty.swap(false, Ordering::Relaxed) {
                let cfg = bg_config.read().unwrap();
                if let Err(e) = cfg.save() {
                    tracing::error!("Failed to persist VoiceGG config: {}", e);
                } else {
                    tracing::debug!("VoiceGG config saved to disk.");
                }
            }
        }
    });

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigquit = signal(SignalKind::quit())?;

    tracing::info!("voicegg-daemon running. Waiting for termination signals...");

    tokio::select! {
        res = tokio::signal::ctrl_c() => {
            if let Err(e) = res {
                tracing::warn!("Failed to listen for Ctrl-C: {e}");
            }
            tracing::info!("Shutdown signal received via SIGINT (Ctrl-C), cleaning up virtual devices...");
        }
        _ = sigterm.recv() => {
            tracing::info!("Shutdown signal received via SIGTERM, cleaning up virtual devices...");
        }
        _ = sigquit.recv() => {
            tracing::info!("Shutdown signal received via SIGQUIT, cleaning up virtual devices...");
        }
        _ = shutdown_rx.recv() => {
            tracing::info!("Shutdown signal received via IPC, cleaning up virtual devices...");
        }
    }

    {
        let mut gm = graph.lock().unwrap();
        let _ = gm.teardown_virtual_devices();
    }

    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
        tracing::info!("Cleaned up IPC socket file at {:?}", socket_path);
    }

    if let Some(mut child) = janitor_child.take() {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = writeln!(stdin, "CLEAN");
            let _ = stdin.flush();
        }
        let _ = child.wait();
    }

    tracing::info!("voicegg-daemon exited cleanly.");

    std::process::exit(0);
}
