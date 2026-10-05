//! Audio device and active stream representation.

use serde::{Deserialize, Serialize};
use voicegg_core::ChannelId;

/// Type of audio device (Sink/Playback or Source/Capture).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    /// Playback sink (headphones, speakers).
    Sink,
    /// Capture source (microphone, line-in).
    Source,
}

/// Representation of a hardware audio device discovered in PipeWire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDevice {
    /// PulseAudio / PipeWire device ID.
    pub id: u32,
    /// Unique node/device name (e.g. "alsa_output.usb-CX31993...").
    pub name: String,
    /// Human-readable friendly description (e.g. "CX31993 384Khz HiFi Audio").
    pub description: String,
    /// Device type.
    pub device_type: DeviceType,
    /// Whether this is currently the default system device.
    pub is_default: bool,
}

/// Represents an active application audio stream currently playing into PipeWire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveStream {
    /// Sink input ID.
    pub id: u32,
    /// Friendly application title (e.g. "Google Chrome", "Discord", "Spotify").
    pub app_name: String,
    /// Executable binary name (e.g. "chrome", "discord", "spotify", "cs2").
    pub binary_name: String,
    /// Process ID.
    pub pid: Option<u32>,
    /// Target VoiceGG channel if currently routed to one.
    pub current_channel: Option<ChannelId>,
}
