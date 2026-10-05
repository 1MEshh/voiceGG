//! Audio device enumeration types.

use serde::{Deserialize, Serialize};

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
    /// PipeWire object ID.
    pub id: u32,
    /// Unique node name (e.g. "alsa_output.pci-0000_00_1f.3.analog-stereo").
    pub name: String,
    /// Human-readable friendly description (e.g. "CX31993 384Khz HiFi Audio").
    pub description: String,
    /// Device type.
    pub device_type: DeviceType,
    /// Whether this is currently marked as default in WirePlumber.
    pub is_default: bool,
}
