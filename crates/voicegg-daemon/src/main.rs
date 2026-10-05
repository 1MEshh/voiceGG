use anyhow::Result;
use std::sync::{Arc, Mutex, RwLock};
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use voicegg_core::VoiceggConfig;
use voicegg_ipc::{IpcRequest, IpcResponse, IpcServer, RequestHandler, SystemStatus};
use voicegg_pw::GraphManager;

struct DaemonHandler {
    graph: Arc<Mutex<GraphManager>>,
    config: Arc<RwLock<VoiceggConfig>>,
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
                IpcResponse::Status(Box::new(SystemStatus {
                    config: cfg,
                    devices,
                    streams,
                    active_game: None,
                }))
            }
            IpcRequest::SetVolume { channel, volume } => {
                let mut cfg = self.config.write().unwrap();
                cfg.volumes.insert(channel, volume);
                tracing::info!("Volume for {} set to {}%", channel, volume);
                IpcResponse::Success
            }
            IpcRequest::SetMute { channel, muted } => {
                let mut cfg = self.config.write().unwrap();
                cfg.muted.insert(channel, muted);
                tracing::info!("Mute for {} set to {}", channel, muted);
                IpcResponse::Success
            }
            IpcRequest::SetChatMix { value } => {
                let mut cfg = self.config.write().unwrap();
                cfg.chatmix = value.clamp(-100, 100);
                tracing::info!("ChatMix balance set to {}", cfg.chatmix);
                IpcResponse::Success
            }
            IpcRequest::SetPreset { channel, preset } => {
                let mut cfg = self.config.write().unwrap();
                cfg.active_presets.insert(channel, preset.id.clone());
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
                        // Update or add rule
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

    let graph = Arc::new(Mutex::new(graph_manager));
    let config = Arc::new(RwLock::new(VoiceggConfig::default()));

    let handler = Arc::new(DaemonHandler {
        graph: Arc::clone(&graph),
        config,
    });

    let socket_path = voicegg_ipc::default_socket_path();
    tracing::info!("Binding IPC socket at {:?}", socket_path);
    let server = IpcServer::bind(&socket_path)?;

    tokio::spawn(async move {
        if let Err(e) = server.run(handler).await {
            tracing::error!("IPC server encountered error: {}", e);
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
