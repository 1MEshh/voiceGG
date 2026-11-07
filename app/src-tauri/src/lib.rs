//! Tauri 2 backend bridge connecting the GUI to voicegg-daemon via Unix socket IPC.

use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;
use tokio::time::sleep;
use voicegg_core::{ChannelId, PluginCapabilities, Preset};
use voicegg_ipc::{ChannelMeter, IpcClient, IpcRequest, IpcResponse, SystemStatus};
use voicegg_pw::AudioDevice;

struct IpcState {
    client: Mutex<Option<IpcClient>>,
    shutting_down: AtomicBool,
    close_to_tray: AtomicBool,
}

impl Default for IpcState {
    fn default() -> Self {
        Self {
            client: Mutex::new(None),
            shutting_down: AtomicBool::new(false),
            close_to_tray: AtomicBool::new(true),
        }
    }
}

impl IpcState {
    async fn send_request(&self, req: IpcRequest) -> Result<IpcResponse, String> {
        // Immediate guard against sending requests (or spawning daemon) during shutdown
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err("Application is shutting down".into());
        }

        let mut lock = self.client.lock().await;

        if self.shutting_down.load(Ordering::Relaxed) {
            return Err("Application is shutting down".into());
        }

        if lock.is_none() {
            match IpcClient::connect_default().await {
                Ok(mut c) => {
                    let pid = std::process::id();
                    let _ = c.request(IpcRequest::RegisterClientPid { pid }).await;
                    *lock = Some(c);
                }
                Err(_) => {
                    if self.shutting_down.load(Ordering::Relaxed) {
                        return Err("Application is shutting down".into());
                    }

                    // Attempt to start the daemon if not running
                    let _ = Command::new("voicegg-daemon").spawn();
                    sleep(Duration::from_millis(400)).await;

                    if self.shutting_down.load(Ordering::Relaxed) {
                        return Err("Application is shutting down".into());
                    }

                    match IpcClient::connect_default().await {
                        Ok(mut c) => {
                            let pid = std::process::id();
                            let _ = c.request(IpcRequest::RegisterClientPid { pid }).await;
                            *lock = Some(c);
                        }
                        Err(e) => {
                            return Err(format!(
                                "Could not connect to voicegg-daemon: {e}. Is PipeWire running?"
                            ));
                        }
                    }
                }
            }
        }

        if let Some(ref mut client) = *lock {
            match client.request(req.clone()).await {
                Ok(res) => Ok(res),
                Err(_) => {
                    if self.shutting_down.load(Ordering::Relaxed) {
                        return Err("Application is shutting down".into());
                    }
                    // Reconnect attempt on broken pipe
                    *lock = None;
                    if let Ok(mut c) = IpcClient::connect_default().await {
                        let pid = std::process::id();
                        let _ = c.request(IpcRequest::RegisterClientPid { pid }).await;
                        let res = c.request(req).await.map_err(|e| e.to_string())?;
                        *lock = Some(c);
                        Ok(res)
                    } else {
                        Err("Daemon disconnected".to_string())
                    }
                }
            }
        } else {
            Err("Failed to initialize IPC connection".to_string())
        }
    }
}

#[tauri::command]
async fn get_status(state: State<'_, IpcState>) -> Result<SystemStatus, String> {
    match state.send_request(IpcRequest::GetStatus).await? {
        IpcResponse::Status(status) => Ok(*status),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response type from daemon".into()),
    }
}

#[tauri::command]
async fn set_volume(channel: String, volume: u8, state: State<'_, IpcState>) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::SetVolume {
            channel: ch,
            volume,
        })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_mute(channel: String, muted: bool, state: State<'_, IpcState>) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::SetMute { channel: ch, muted })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_chatmix(value: i8, state: State<'_, IpcState>) -> Result<(), String> {
    match state.send_request(IpcRequest::SetChatMix { value }).await? {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_preset(
    channel: String,
    preset_id: String,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    let custom_preset = if let Ok(IpcResponse::Status(status)) =
        state.send_request(IpcRequest::GetStatus).await
    {
        status.config.custom_presets.get(&preset_id).cloned()
    } else {
        None
    };
    let preset = custom_preset
        .or_else(|| Preset::get_preset_by_id(&preset_id))
        .ok_or_else(|| format!("Preset '{preset_id}' not found"))?;

    match state
        .send_request(IpcRequest::SetPreset {
            channel: ch,
            preset,
        })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn route_app(
    binary_name: String,
    target_channel: String,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&target_channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::RouteApp {
            binary_name,
            target_channel: ch,
        })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn get_devices(state: State<'_, IpcState>) -> Result<Vec<AudioDevice>, String> {
    match state.send_request(IpcRequest::GetDevices).await? {
        IpcResponse::Devices(devices) => Ok(devices),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
fn get_builtin_presets() -> Vec<Preset> {
    Preset::builtin_presets()
}

#[tauri::command]
async fn set_audio_device(
    device_type: String,
    device_name: String,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let dt = match device_type.to_lowercase().as_str() {
        "sink" => voicegg_pw::DeviceType::Sink,
        "source" => voicegg_pw::DeviceType::Source,
        _ => return Err(format!("Invalid device type: {device_type}")),
    };

    match state
        .send_request(IpcRequest::SetDevice {
            device_type: dt,
            device_name,
        })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn panic_reset(state: State<'_, IpcState>) -> Result<(), String> {
    match state.send_request(IpcRequest::PanicReset).await? {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_mic_monitor(
    enabled: bool,
    volume: u8,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    match state
        .send_request(IpcRequest::SetMicMonitor { enabled, volume })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn get_capabilities(state: State<'_, IpcState>) -> Result<PluginCapabilities, String> {
    match state.send_request(IpcRequest::GetCapabilities).await? {
        IpcResponse::Capabilities(caps) => Ok(caps),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn get_meters(
    state: State<'_, IpcState>,
) -> Result<HashMap<ChannelId, ChannelMeter>, String> {
    match state.send_request(IpcRequest::GetMeters).await? {
        IpcResponse::Meters(meters) => Ok(meters),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn save_preset(
    channel: String,
    preset: Preset,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::SetPreset {
            channel: ch,
            preset,
        })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_language(language: String, state: State<'_, IpcState>) -> Result<(), String> {
    match state
        .send_request(IpcRequest::SetLanguage { language })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_app_volume(
    binary_name: String,
    volume: u8,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    match state
        .send_request(IpcRequest::SetAppVolume {
            binary_name,
            volume,
        })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_streamer_mode(enabled: bool, state: State<'_, IpcState>) -> Result<(), String> {
    match state.send_request(IpcRequest::SetStreamerMode { enabled }).await? {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_personal_mix(
    channel: String,
    enabled: bool,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::SetPersonalMix { channel: ch, enabled })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_stream_mix(
    channel: String,
    enabled: bool,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::SetStreamMix { channel: ch, enabled })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn toggle_favorite_preset(
    channel: String,
    preset_id: String,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::ToggleFavoritePreset { channel: ch, preset_id })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_channel_color(
    channel: String,
    hex: String,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::SetChannelColor { channel: ch, hex })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn save_custom_preset(
    channel: String,
    preset: Preset,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::SaveCustomPreset { channel: ch, preset })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn delete_custom_preset(
    preset_id: String,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    match state
        .send_request(IpcRequest::DeleteCustomPreset { preset_id })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_quantum(quantum: u32, state: State<'_, IpcState>) -> Result<(), String> {
    match state
        .send_request(IpcRequest::SetQuantum { quantum })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_auto_game_detection(
    enabled: bool,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    match state
        .send_request(IpcRequest::SetAutoGameDetection { enabled })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_close_to_tray(
    enabled: bool,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    state.close_to_tray.store(enabled, Ordering::Relaxed);
    match state
        .send_request(IpcRequest::SetCloseToTray { enabled })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn set_app_mute(
    binary_name: String,
    muted: bool,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    match state
        .send_request(IpcRequest::SetAppMute {
            binary_name,
            muted,
        })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

#[tauri::command]
async fn reset_preset(
    channel: String,
    preset_id: String,
    state: State<'_, IpcState>,
) -> Result<(), String> {
    let ch = ChannelId::from_str(&channel).map_err(|e| e.to_string())?;
    match state
        .send_request(IpcRequest::ResetPreset {
            channel: ch,
            preset_id,
        })
        .await?
    {
        IpcResponse::Success => Ok(()),
        IpcResponse::Error(e) => Err(e),
        _ => Err("Unexpected response from daemon".into()),
    }
}

fn kill_voicegg_daemon_processes() {
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let pid_str = file_name.to_string_lossy();
            if !pid_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let comm_path = entry.path().join("comm");
            if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                if comm.trim() == "voicegg-daemon" {
                    let _ = Command::new("kill").args(["-9", &pid_str]).status();
                }
            }
        }
    }
}

fn gui_socket_path() -> std::path::PathBuf {
    if let Some(runtime_dir) = dirs::runtime_dir() {
        runtime_dir.join("voicegg").join("gui.sock")
    } else {
        std::path::PathBuf::from("/tmp/voicegg-gui.sock")
    }
}

fn ensure_single_instance_or_focus() -> bool {
    let socket_path = gui_socket_path();

    if let Some(parent) = socket_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if socket_path.exists() {
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&socket_path) {
            use std::io::Write;
            let _ = stream.write_all(b"FOCUS\n");
            let _ = stream.flush();
            return false;
        }
        let _ = std::fs::remove_file(&socket_path);
    }

    true
}

async fn perform_full_shutdown(app: &tauri::AppHandle) {
    let state = app.state::<IpcState>();

    // 1. FIRST atomic step: gate all requests and prevent any polls from respawning daemon!
    state.shutting_down.store(true, Ordering::Relaxed);

    // 2. Send Shutdown IPC to voicegg-daemon with a 300ms timeout
    let _ = tokio::time::timeout(Duration::from_millis(300), async {
        if let Ok(mut client) = IpcClient::connect_default().await {
            let _ = client.request(IpcRequest::Shutdown).await;
        }
    })
    .await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // 3. Stop systemd user service
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "voicegg-daemon.service"])
        .status();

    // 4. Sweep daemon processes matching exact /proc/<pid>/comm == "voicegg-daemon"
    kill_voicegg_daemon_processes();

    // 5. Guarantee all voicegg_ PipeWire sinks, loopbacks, and parec/pacat DSP processes are destroyed directly in-process
    voicegg_pw::GraphManager::cleanup_stale_modules();

    // 6. Clean up GUI socket
    let _ = std::fs::remove_file(gui_socket_path());

    // 7. Exit GUI process
    app.exit(0);
}

#[tauri::command]
fn hide_window(window: tauri::Window) {
    let _ = window.hide();
}

#[tauri::command]
async fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    perform_full_shutdown(&app).await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !ensure_single_instance_or_focus() {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(IpcState::default())
        .setup(|app| {
            use tauri::{
                menu::{Menu, MenuItem, PredefinedMenuItem},
                tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
            };

            // Single-instance GUI listener
            let socket_path = gui_socket_path();
            if let Some(parent) = socket_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::remove_file(&socket_path);

            if let Ok(listener) = tokio::net::UnixListener::bind(&socket_path) {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        if let Ok((mut stream, _)) = listener.accept().await {
                            use tokio::io::AsyncReadExt;
                            let mut buf = [0u8; 32];
                            if let Ok(n) = stream.read(&mut buf).await {
                                if n > 0 {
                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        let _ = window.show();
                                        let _ = window.unminimize();
                                        let _ = window.set_focus();
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // Sync close_to_tray setting from daemon config on startup
            let init_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = init_app.state::<IpcState>();
                if let Ok(IpcResponse::Status(status)) =
                    state.send_request(IpcRequest::GetStatus).await
                {
                    state
                        .close_to_tray
                        .store(status.config.close_to_tray, Ordering::Relaxed);
                }
            });

            let title_i = MenuItem::with_id(
                app,
                "title",
                "VoiceGG — Sonar for Linux",
                false,
                None::<&str>,
            )?;
            let show_i =
                MenuItem::with_id(app, "toggle_win", "Show / Hide VoiceGG", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let mute_master_i =
                MenuItem::with_id(app, "mute_master", "Toggle Mute Master", true, None::<&str>)?;
            let mute_mic_i = MenuItem::with_id(
                app,
                "mute_mic",
                "Toggle Mute Microphone",
                true,
                None::<&str>,
            )?;
            let monitor_mic_i = MenuItem::with_id(
                app,
                "monitor_mic",
                "Toggle Voice Monitor",
                true,
                None::<&str>,
            )?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let panic_i =
                MenuItem::with_id(app, "panic", "Emergency Reset Audio", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit VoiceGG", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &title_i,
                    &show_i,
                    &sep1,
                    &mute_master_i,
                    &mute_mic_i,
                    &monitor_mic_i,
                    &sep2,
                    &panic_i,
                    &quit_i,
                ],
            )?;

            // Safe handling for window icon (avoids panicking if icon is missing)
            let mut tray_builder = TrayIconBuilder::new()
                .tooltip("VoiceGG Audio Mixer")
                .menu(&menu);

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _tray = tray_builder
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            perform_full_shutdown(&app_handle).await;
                        });
                    }
                    "toggle_win" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    "mute_master" => {
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_handle.state::<IpcState>();
                            if let Ok(voicegg_ipc::IpcResponse::Status(status)) =
                                state.send_request(voicegg_ipc::IpcRequest::GetStatus).await
                            {
                                let current = status
                                    .config
                                    .muted
                                    .get(&voicegg_core::ChannelId::Master)
                                    .copied()
                                    .unwrap_or(false);
                                let _ = state
                                    .send_request(voicegg_ipc::IpcRequest::SetMute {
                                        channel: voicegg_core::ChannelId::Master,
                                        muted: !current,
                                    })
                                    .await;
                                let _ = app_handle.emit("voicegg://status-changed", ());
                            }
                        });
                    }
                    "mute_mic" => {
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_handle.state::<IpcState>();
                            if let Ok(voicegg_ipc::IpcResponse::Status(status)) =
                                state.send_request(voicegg_ipc::IpcRequest::GetStatus).await
                            {
                                let current = status
                                    .config
                                    .muted
                                    .get(&voicegg_core::ChannelId::Mic)
                                    .copied()
                                    .unwrap_or(false);
                                let _ = state
                                    .send_request(voicegg_ipc::IpcRequest::SetMute {
                                        channel: voicegg_core::ChannelId::Mic,
                                        muted: !current,
                                    })
                                    .await;
                                let _ = app_handle.emit("voicegg://status-changed", ());
                            }
                        });
                    }
                    "monitor_mic" => {
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_handle.state::<IpcState>();
                            if let Ok(voicegg_ipc::IpcResponse::Status(status)) =
                                state.send_request(voicegg_ipc::IpcRequest::GetStatus).await
                            {
                                let current = status.mic_monitor_enabled;
                                let vol = status.mic_monitor_volume;
                                let _ = state
                                    .send_request(voicegg_ipc::IpcRequest::SetMicMonitor {
                                        enabled: !current,
                                        volume: vol,
                                    })
                                    .await;
                                let _ = app_handle.emit("voicegg://status-changed", ());
                            }
                        });
                    }
                    "panic" => {
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_handle.state::<IpcState>();
                            let _ = state
                                .send_request(voicegg_ipc::IpcRequest::PanicReset)
                                .await;
                        });
                    }
                    _ => {}
                })
                .build(app)?;

            let sig_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm = signal(SignalKind::terminate()).ok();
                let mut sigint = signal(SignalKind::interrupt()).ok();
                tokio::select! {
                    _ = async {
                        if let Some(ref mut s) = sigterm {
                            let _ = s.recv().await;
                        } else {
                            std::future::pending::<()>().await;
                        }
                    } => {}
                    _ = async {
                        if let Some(ref mut s) = sigint {
                            let _ = s.recv().await;
                        } else {
                            std::future::pending::<()>().await;
                        }
                    } => {}
                }
                perform_full_shutdown(&sig_app).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<IpcState>();
                if state.shutting_down.load(Ordering::Relaxed) {
                    return;
                }
                if state.close_to_tray.load(Ordering::Relaxed) {
                    let _ = window.hide();
                    api.prevent_close();
                } else {
                    api.prevent_close();
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        perform_full_shutdown(&app_handle).await;
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            set_volume,
            set_mute,
            set_chatmix,
            set_preset,
            save_preset,
            route_app,
            get_devices,
            set_audio_device,
            get_builtin_presets,
            panic_reset,
            set_mic_monitor,
            get_capabilities,
            get_meters,
            set_language,
            set_app_volume,
            set_streamer_mode,
            set_personal_mix,
            set_stream_mix,
            toggle_favorite_preset,
            set_channel_color,
            save_custom_preset,
            delete_custom_preset,
            set_quantum,
            hide_window,
            quit_app,
            set_auto_game_detection,
            set_close_to_tray,
            set_app_mute,
            reset_preset
        ])
        .run(tauri::generate_context!())
        .expect("error while running VoiceGG GUI application");
}
