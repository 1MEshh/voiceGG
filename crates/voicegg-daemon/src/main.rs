//! VoiceGG Daemon - Manages PipeWire audio graph, DSP processing, and IPC.

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use voicegg_core::VoiceggConfig;
use voicegg_ipc::{IpcRequest, IpcResponse, IpcServer, RequestHandler, SystemStatus};
use voicegg_pw::GraphManager;

struct DaemonHandler {
    graph: Arc<tokio::sync::Mutex<GraphManager>>,
    config: Arc<tokio::sync::RwLock<VoiceggConfig>>,
}

impl RequestHandler for DaemonHandler {
    fn handle_request(&self, req: IpcRequest) -> IpcResponse {
        match req {
            IpcRequest::GetStatus => {
                let cfg = {
                    let lock = self.config.blocking_read();
                    lock.clone()
                };
                let devices = {
                    let gm = self.graph.blocking_lock();
                    gm.list_devices().unwrap_or_default()
                };
                IpcResponse::Status(Box::new(SystemStatus {
                    config: cfg,
                    devices,
                    active_game: None,
                }))
            }
            IpcRequest::SetVolume { channel, volume } => {
                let mut cfg = self.config.blocking_write();
                cfg.volumes.insert(channel, volume);
                tracing::info!("Volume for {} set to {}%", channel, volume);
                IpcResponse::Success
            }
            IpcRequest::SetMute { channel, muted } => {
                let mut cfg = self.config.blocking_write();
                cfg.muted.insert(channel, muted);
                tracing::info!("Mute for {} set to {}", channel, muted);
                IpcResponse::Success
            }
            IpcRequest::SetChatMix { value } => {
                let mut cfg = self.config.blocking_write();
                cfg.chatmix = value.clamp(-100, 100);
                tracing::info!("ChatMix balance set to {}", cfg.chatmix);
                IpcResponse::Success
            }
            IpcRequest::SetPreset { channel, preset } => {
                let mut cfg = self.config.blocking_write();
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
                IpcResponse::Success
            }
            IpcRequest::GetDevices => {
                let gm = self.graph.blocking_lock();
                match gm.list_devices() {
                    Ok(devs) => IpcResponse::Devices(devs),
                    Err(e) => IpcResponse::Error(e.to_string()),
                }
            }
            IpcRequest::PanicReset => {
                tracing::warn!("PanicReset requested: resetting audio graph to system defaults");
                let gm = self.graph.blocking_lock();
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

    let graph = Arc::new(tokio::sync::Mutex::new(graph_manager));
    let config = Arc::new(tokio::sync::RwLock::new(VoiceggConfig::default()));

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

    let gm = graph.lock().await;
    gm.teardown_virtual_devices()?;
    tracing::info!("voicegg-daemon exited cleanly.");

    Ok(())
}
