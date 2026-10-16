//! Tauri 2 backend bridge connecting the GUI to voicegg-daemon via Unix socket IPC.

use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;
use tauri::State;
use tokio::sync::Mutex;
use tokio::time::sleep;
use voicegg_core::{ChannelId, PluginCapabilities, Preset};
use voicegg_ipc::{ChannelMeter, IpcClient, IpcRequest, IpcResponse, SystemStatus};
use voicegg_pw::AudioDevice;

struct IpcState {
    client: Mutex<Option<IpcClient>>,
}

impl IpcState {
    async fn send_request(&self, req: IpcRequest) -> Result<IpcResponse, String> {
        let mut lock = self.client.lock().await;

        if lock.is_none() {
            match IpcClient::connect_default().await {
                Ok(c) => *lock = Some(c),
                Err(_) => {
                    // Attempt to start the daemon if not running
                    let _ = Command::new("voicegg-daemon").spawn();
                    sleep(Duration::from_millis(400)).await;

                    match IpcClient::connect_default().await {
                        Ok(c) => *lock = Some(c),
                        Err(e) => {
                            return Err(format!(
                                "Could not connect to voicegg-daemon: {e}. Is PipeWire running?"
                            ))
                        }
                    }
                }
            }
        }

        if let Some(ref mut client) = *lock {
            match client.request(req.clone()).await {
                Ok(res) => Ok(res),
                Err(_) => {
                    // Reconnect attempt on broken pipe
                    *lock = None;
                    if let Ok(mut c) = IpcClient::connect_default().await {
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
    let preset = Preset::get_preset_by_id(&preset_id)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(IpcState {
            client: Mutex::new(None),
        })
        .setup(|app| {
            use tauri::{
                menu::{Menu, MenuItem, PredefinedMenuItem},
                tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
                Manager,
            };

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

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("VoiceGG Audio Mixer")
                .menu(&menu)
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
                            let state = app_handle.state::<IpcState>();
                            let _ = state.send_request(voicegg_ipc::IpcRequest::Shutdown).await;
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                            app_handle.exit(0);
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

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
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
            delete_custom_preset
        ])
        .run(tauri::generate_context!())
        .expect("error while running VoiceGG GUI application");
}
