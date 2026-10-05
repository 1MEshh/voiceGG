//! VoiceGG Background Daemon.
//!
//! Owns the PipeWire audio graph, runs the game detection engine,
//! auto-routes application streams, and serves client IPC requests.

mod game_detector;

use anyhow::Result;
use game_detector::GameDetector;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::signal;
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
}

impl RequestHandler for DaemonHandler {
    fn handle_request(&self, req: IpcRequest) -> IpcResponse {
        match req {
            IpcRequest::GetStatus => {
                let cfg = {
                    let lock = self.config.read().unwrap();
                    lock.clone()
                };
                let (devices, streams) = {
                    let gm = self.graph.lock().unwrap();
                    (
                        gm.list_devices().unwrap_or_default(),
                        gm.list_active_streams().unwrap_or_default(),
                    )
                };
                let game = self.active_game.read().unwrap().clone();
                IpcResponse::Status(Box::new(SystemStatus {
                    config: cfg,
                    devices,
                    streams,
                    active_game: game,
                }))
            }
            IpcRequest::SetVolume { channel, volume } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.volumes.insert(channel, volume);
                }
                let gm = self.graph.lock().unwrap();
                let _ = gm.set_channel_volume(channel, volume);
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Volume for {} set to {}%", channel, volume);
                IpcResponse::Success
            }
            IpcRequest::SetMute { channel, muted } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.muted.insert(channel, muted);
                }
                let gm = self.graph.lock().unwrap();
                let _ = gm.set_channel_mute(channel, muted);
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Mute for {} set to {}", channel, muted);
                IpcResponse::Success
            }
            IpcRequest::SetChatMix { value } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.chatmix = value.clamp(-100, 100);
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("ChatMix balance set to {}", value);
                IpcResponse::Success
            }
            IpcRequest::SetPreset { channel, preset } => {
                {
                    let mut cfg = self.config.write().unwrap();
                    cfg.active_presets.insert(channel, preset.id.clone());
                }
                self.dirty.store(true, Ordering::Relaxed);
                tracing::info!("Preset '{}' applied to {}", preset.name, channel);
                IpcResponse::Success
            }
            IpcRequest::RouteApp {
                binary_name,
                target_channel,
            } => {
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
            IpcRequest::PanicReset => {
                tracing::warn!("PanicReset requested: resetting audio graph to system defaults");
                let mut gm = self.graph.lock().unwrap();
                let _ = gm.teardown_virtual_devices();
                IpcResponse::Success
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

    // Load saved configuration or create default
    let loaded_config = VoiceggConfig::load();
    tracing::info!(
        "Loaded configuration with {} routing rules",
        loaded_config.routing_rules.len()
    );

    let graph = Arc::new(Mutex::new(graph_manager));
    let config = Arc::new(RwLock::new(loaded_config));
    let active_game = Arc::new(RwLock::new(None));
    let dirty = Arc::new(AtomicBool::new(false));

    let handler = Arc::new(DaemonHandler {
        graph: Arc::clone(&graph),
        config: Arc::clone(&config),
        active_game: Arc::clone(&active_game),
        dirty: Arc::clone(&dirty),
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
        loop {
            sleep(Duration::from_millis(1500)).await;

            // 1. Process active stream routing
            {
                let gm = bg_graph.lock().unwrap();
                if let Ok(streams) = gm.list_active_streams() {
                    let cfg = bg_config.read().unwrap();
                    for stream in streams {
                        if stream.current_channel.is_none() {
                            // Find explicit rule
                            let target =
                                if let Some(rule) = cfg.routing_rules.iter().find(|r| {
                                    r.binary_name.eq_ignore_ascii_case(&stream.binary_name)
                                }) {
                                    rule.target_channel
                                } else {
                                    smart_classify_app(&stream.binary_name)
                                };

                            let _ = gm.route_stream(stream.id, target);
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
                            let mut cfg = bg_config.write().unwrap();
                            cfg.active_presets
                                .insert(ChannelId::Game, game.preset_id.clone());
                            bg_dirty.store(true, Ordering::Relaxed);

                            // Route game process stream to Game channel
                            let gm = bg_graph.lock().unwrap();
                            let _ = gm.route_app_by_name(&game.matched_binary, ChannelId::Game);
                        }
                    }
                    None => {
                        if current.is_some() {
                            tracing::info!("Game exited. Reverting Game channel preset.");
                            *current = None;
                            let mut cfg = bg_config.write().unwrap();
                            cfg.active_presets
                                .insert(ChannelId::Game, "game_flat".to_string());
                            bg_dirty.store(true, Ordering::Relaxed);
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

    tracing::info!("voicegg-daemon running. Waiting for termination signals...");

    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received, cleaning up virtual devices...");

    let mut gm = graph.lock().unwrap();
    gm.teardown_virtual_devices()?;
    tracing::info!("voicegg-daemon exited cleanly.");

    Ok(())
}
