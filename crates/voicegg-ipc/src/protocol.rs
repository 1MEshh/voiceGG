//! Message schemas and structures exchanged over IPC.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use voicegg_core::{ChannelId, PluginCapabilities, Preset, VoiceggConfig};
use voicegg_pw::{ActiveStream, AudioDevice};
pub use voicegg_pw::ChannelMeter;

/// Overall system state snapshot returned by the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemStatus {
    /// Active configuration and channel settings.
    pub config: VoiceggConfig,
    /// Available output and input audio devices.
    pub devices: Vec<AudioDevice>,
    /// Currently active application playback streams.
    pub streams: Vec<ActiveStream>,
    /// Active running game name if auto-detected.
    pub active_game: Option<String>,
    /// Available DSP plugin capabilities.
    pub capabilities: PluginCapabilities,
    /// Whether microphone monitoring (sidetone) is active.
    pub mic_monitor_enabled: bool,
    /// Microphone monitor volume in percentage (0 .. 100).
    pub mic_monitor_volume: u8,
}

/// Commands/Requests sent from Client (CLI / GUI) to Daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
#[allow(clippy::large_enum_variant)]
pub enum IpcRequest {
    /// Request the full system status.
    GetStatus,
    /// Set volume for a channel (0 .. 150).
    SetVolume {
        /// Channel to adjust.
        channel: ChannelId,
        /// Volume in percentage.
        volume: u8,
    },
    /// Set mute status for a channel.
    SetMute {
        /// Channel to adjust.
        channel: ChannelId,
        /// Mute state.
        muted: bool,
    },
    /// Set ChatMix balance (-100 to +100).
    SetChatMix {
        /// Balance value.
        value: i8,
    },
    /// Apply an audio preset to a channel.
    SetPreset {
        /// Channel to apply to.
        channel: ChannelId,
        /// Audio preset.
        preset: Preset,
    },
    /// Route an application stream to a specific channel.
    RouteApp {
        /// Application binary name.
        binary_name: String,
        /// Destination channel.
        target_channel: ChannelId,
    },
    /// List hardware audio devices.
    GetDevices,
    /// Set preferred hardware audio device (sink or source).
    SetDevice {
        /// Device type (Sink or Source).
        device_type: voicegg_pw::DeviceType,
        /// Physical device node name (e.g. alsa_output.usb-... or default).
        device_name: String,
    },
    /// Set microphone monitor (sidetone) loopback state and volume.
    SetMicMonitor {
        /// Whether monitoring is enabled.
        enabled: bool,
        /// Monitor loopback volume (0 .. 100).
        volume: u8,
    },
    /// Query detected DSP plugin capabilities.
    GetCapabilities,
    /// Query instantaneous audio meter levels across channels.
    GetMeters,
    /// Emergency panic: destroy all virtual devices and restore default system routing.
    PanicReset,
    /// Gracefully shut down daemon, unlinking socket and tearing down virtual devices.
    Shutdown,
    /// Update UI language setting.
    SetLanguage {
        /// Language code (e.g. "en", "ar").
        language: String,
    },
    /// Set volume for a specific application stream.
    SetAppVolume {
        /// Application binary name.
        binary_name: String,
        /// Volume in percentage (0 .. 150).
        volume: u8,
    },
    /// Cycle to the next available preset for a channel.
    CyclePreset {
        /// Channel to cycle.
        channel: ChannelId,
    },
    /// Set streamer dual-bus mode enabled or disabled.
    SetStreamerMode {
        /// Whether streamer mode is active.
        enabled: bool,
    },
    /// Set personal headphone mix toggle for a channel.
    SetPersonalMix {
        /// Channel to toggle.
        channel: ChannelId,
        /// Enabled in personal mix.
        enabled: bool,
    },
    /// Set stream audience mix toggle for a channel.
    SetStreamMix {
        /// Channel to toggle.
        channel: ChannelId,
        /// Enabled in stream mix.
        enabled: bool,
    },
    /// Toggle favorite status of a preset for a channel.
    ToggleFavoritePreset {
        /// Target channel.
        channel: ChannelId,
        /// Preset ID.
        preset_id: String,
    },
    /// Set accent color for a specific channel.
    SetChannelColor {
        /// Target channel.
        channel: ChannelId,
        /// Color hex code.
        hex: String,
    },
    /// Save a custom user preset.
    SaveCustomPreset {
        /// Channel for the preset.
        channel: ChannelId,
        /// Full preset definition.
        preset: Preset,
    },
    /// Delete a custom user preset by ID.
    DeleteCustomPreset {
        /// ID of preset to delete.
        preset_id: String,
    },
    /// Set low-latency audio buffer quantum (e.g. 64, 128, 256, 512, 1024).
    SetQuantum {
        /// Buffer quantum size.
        quantum: u32,
    },
    /// Register GUI client PID for lifecycle tethering.
    RegisterClientPid {
        /// Client process ID.
        pid: u32,
    },
    /// Set auto game detection enabled or disabled.
    SetAutoGameDetection {
        /// Whether auto game detection is enabled.
        enabled: bool,
    },
    /// Set close-to-tray window behavior.
    SetCloseToTray {
        /// Whether closing the window minimizes to tray instead of exiting.
        enabled: bool,
    },
    /// Set mute status for a specific application stream.
    SetAppMute {
        /// Application binary name.
        binary_name: String,
        /// Mute state.
        muted: bool,
    },
    /// Reset a built-in preset to its factory defaults.
    ResetPreset {
        /// Target channel.
        channel: ChannelId,
        /// Preset ID to reset.
        preset_id: String,
    },
    /// Keepalive ping request to verify daemon responsiveness.
    Ping,
}

/// Responses returned from Daemon to Client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcResponse {
    /// Operation succeeded without payload.
    Success,
    /// Keepalive pong response.
    Pong,
    /// Full system status payload.
    Status(Box<SystemStatus>),
    /// Audio devices list payload.
    Devices(Vec<AudioDevice>),
    /// Host DSP plugin capabilities.
    Capabilities(PluginCapabilities),
    /// Channel audio meter snapshot.
    Meters(HashMap<ChannelId, ChannelMeter>),
    /// Operation failed with an error message.
    Error(String),
}

/// Configuration options for IPC client requests and timeout thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestConfig {
    /// Timeout in milliseconds (default: 2000 ms).
    pub timeout_ms: u64,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
        }
    }
}

impl RequestConfig {
    /// Sane default timeout for IPC roundtrips (2000 ms).
    pub const DEFAULT_TIMEOUT_MS: u64 = 2000;

    /// Creates a default request configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
        }
    }

    /// Creates a request configuration with an explicit timeout in milliseconds.
    #[must_use]
    pub const fn with_timeout_ms(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }

    /// Converts timeout configuration to a standard `std::time::Duration`.
    #[must_use]
    pub fn duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms)
    }
}

/// Asynchronous events streamed from Daemon to subscribed Clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcEvent {
    /// Live meter updates (sent at 30-60 Hz).
    MeterUpdate {
        /// Channel reporting the meter.
        channel: ChannelId,
        /// Peak level in dBFS.
        peak_dbfs: f32,
        /// RMS level in dBFS.
        rms_dbfs: f32,
    },
    /// Active game process detected or ended.
    GameDetected {
        /// Name of the game detected, or None if game ended.
        game_name: Option<String>,
        /// Preset applied.
        preset_id: Option<String>,
    },
    /// Notification that the routing or app streams changed.
    AppsChanged,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_request_shutdown_serialization() {
        let req = IpcRequest::Shutdown;
        let serialized = serde_json::to_string(&req).expect("Failed to serialize Shutdown request");
        assert!(serialized.contains("Shutdown"));
        let deserialized: IpcRequest =
            serde_json::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(req, deserialized);
    }

    #[test]
    fn test_set_volume_serialization() {
        let req = IpcRequest::SetVolume {
            channel: ChannelId::Master,
            volume: 85,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_route_app_serialization() {
        let req = IpcRequest::RouteApp {
            binary_name: "cs2".to_string(),
            target_channel: ChannelId::Game,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_ipc_response_serialization() {
        let res = IpcResponse::Success;
        let json = serde_json::to_string(&res).unwrap();
        let decoded: IpcResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(res, decoded);
    }

    #[test]
    fn test_mic_monitor_request_serialization() {
        let req = IpcRequest::SetMicMonitor {
            enabled: true,
            volume: 75,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_capabilities_and_meters_serialization() {
        let caps = PluginCapabilities {
            rnnoise: true,
            gate: false,
            compressor: true,
            biquad_eq: true,
        };
        let resp = IpcResponse::Capabilities(caps);
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: IpcResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, decoded);

        let mut meters_map = HashMap::new();
        meters_map.insert(
            ChannelId::Mic,
            ChannelMeter {
                peak_dbfs: -12.5,
                rms_dbfs: -20.1,
            },
        );
        let resp_meters = IpcResponse::Meters(meters_map);
        let json_m = serde_json::to_string(&resp_meters).unwrap();
        let decoded_m: IpcResponse = serde_json::from_str(&json_m).unwrap();
        assert_eq!(resp_meters, decoded_m);
    }

    #[test]
    fn test_set_language_serialization() {
        let req = IpcRequest::SetLanguage {
            language: "ar".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_set_app_volume_serialization() {
        let req = IpcRequest::SetAppVolume {
            binary_name: "discord".to_string(),
            volume: 85,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_cycle_preset_serialization() {
        let req = IpcRequest::CyclePreset {
            channel: ChannelId::Game,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_sonar_3_protocol_requests_serialization() {
        let req_streamer = IpcRequest::SetStreamerMode { enabled: true };
        assert_eq!(req_streamer, serde_json::from_str(&serde_json::to_string(&req_streamer).unwrap()).unwrap());

        let req_pmix = IpcRequest::SetPersonalMix { channel: ChannelId::Media, enabled: false };
        assert_eq!(req_pmix, serde_json::from_str(&serde_json::to_string(&req_pmix).unwrap()).unwrap());

        let req_smix = IpcRequest::SetStreamMix { channel: ChannelId::Chat, enabled: true };
        assert_eq!(req_smix, serde_json::from_str(&serde_json::to_string(&req_smix).unwrap()).unwrap());

        let req_fav = IpcRequest::ToggleFavoritePreset { channel: ChannelId::Game, preset_id: "game_cs2".to_string() };
        assert_eq!(req_fav, serde_json::from_str(&serde_json::to_string(&req_fav).unwrap()).unwrap());

        let req_color = IpcRequest::SetChannelColor { channel: ChannelId::Mic, hex: "#FF0077".to_string() };
        assert_eq!(req_color, serde_json::from_str(&serde_json::to_string(&req_color).unwrap()).unwrap());

        let req_quantum = IpcRequest::SetQuantum { quantum: 128 };
        assert_eq!(req_quantum, serde_json::from_str(&serde_json::to_string(&req_quantum).unwrap()).unwrap());

        let req_del = IpcRequest::DeleteCustomPreset { preset_id: "custom_123".to_string() };
        assert_eq!(req_del, serde_json::from_str(&serde_json::to_string(&req_del).unwrap()).unwrap());

        let req_pid = IpcRequest::RegisterClientPid { pid: 12345 };
        assert_eq!(req_pid, serde_json::from_str(&serde_json::to_string(&req_pid).unwrap()).unwrap());

        let req_auto_game = IpcRequest::SetAutoGameDetection { enabled: true };
        assert_eq!(req_auto_game, serde_json::from_str(&serde_json::to_string(&req_auto_game).unwrap()).unwrap());

        let req_tray = IpcRequest::SetCloseToTray { enabled: false };
        assert_eq!(req_tray, serde_json::from_str(&serde_json::to_string(&req_tray).unwrap()).unwrap());

        let req_mute = IpcRequest::SetAppMute { binary_name: "discord".to_string(), muted: true };
        assert_eq!(req_mute, serde_json::from_str(&serde_json::to_string(&req_mute).unwrap()).unwrap());

        let req_reset = IpcRequest::ResetPreset { channel: ChannelId::Game, preset_id: "game_flat".to_string() };
        assert_eq!(req_reset, serde_json::from_str(&serde_json::to_string(&req_reset).unwrap()).unwrap());
    }

    #[test]
    fn test_request_config_defaults_and_serialization() {
        let default_cfg = RequestConfig::default();
        assert_eq!(default_cfg.timeout_ms, 2000);
        assert_eq!(default_cfg.duration(), std::time::Duration::from_millis(2000));

        let custom_cfg = RequestConfig::with_timeout_ms(5000);
        assert_eq!(custom_cfg.timeout_ms, 5000);

        let json = serde_json::to_string(&custom_cfg).unwrap();
        let decoded: RequestConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(custom_cfg, decoded);
    }
}
