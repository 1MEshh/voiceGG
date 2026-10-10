//! Message schemas and structures exchanged over IPC.

use serde::{Deserialize, Serialize};
use voicegg_core::{ChannelId, Preset, VoiceggConfig};
use voicegg_pw::{ActiveStream, AudioDevice};

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
}

/// Commands/Requests sent from Client (CLI / GUI) to Daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
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
    /// Emergency panic: destroy all virtual devices and restore default system routing.
    PanicReset,
    /// Gracefully shut down daemon, unlinking socket and tearing down virtual devices.
    Shutdown,
}

/// Responses returned from Daemon to Client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcResponse {
    /// Operation succeeded without payload.
    Success,
    /// Full system status payload.
    Status(Box<SystemStatus>),
    /// Audio devices list payload.
    Devices(Vec<AudioDevice>),
    /// Operation failed with an error message.
    Error(String),
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
