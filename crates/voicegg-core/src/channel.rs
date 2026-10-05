//! Audio channel definitions for VoiceGG.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Standard VoiceGG channel identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelId {
    /// Master mix channel.
    Master,
    /// Game audio channel.
    Game,
    /// Voice communication (e.g. Discord, Teamspeak) channel.
    Chat,
    /// Music, browser, streaming video audio channel.
    Media,
    /// Auxiliary audio channel.
    Aux,
    /// Microphone capture input channel.
    Mic,
    /// Streamer broadcast mix channel.
    StreamMix,
}

impl ChannelId {
    /// Returns the human-readable display name.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Master => "Master",
            Self::Game => "Game",
            Self::Chat => "Chat",
            Self::Media => "Media",
            Self::Aux => "Aux",
            Self::Mic => "Mic",
            Self::StreamMix => "Stream Mix",
        }
    }

    /// Returns the PipeWire node name for this channel's virtual device.
    #[must_use]
    pub const fn pipewire_node_name(&self) -> &'static str {
        match self {
            Self::Master => "voicegg_sink_master",
            Self::Game => "voicegg_sink_game",
            Self::Chat => "voicegg_sink_chat",
            Self::Media => "voicegg_sink_media",
            Self::Aux => "voicegg_sink_aux",
            Self::Mic => "voicegg_source_mic",
            Self::StreamMix => "voicegg_source_stream_mix",
        }
    }

    /// Returns the PipeWire node description shown to the desktop environment.
    #[must_use]
    pub const fn pipewire_description(&self) -> &'static str {
        match self {
            Self::Master => "VoiceGG Master",
            Self::Game => "VoiceGG Game",
            Self::Chat => "VoiceGG Chat",
            Self::Media => "VoiceGG Media",
            Self::Aux => "VoiceGG Aux",
            Self::Mic => "VoiceGG Microphone",
            Self::StreamMix => "VoiceGG Stream Mix",
        }
    }

    /// Returns true if this channel represents an output sink (playback).
    #[must_use]
    pub const fn is_sink(&self) -> bool {
        matches!(
            self,
            Self::Master | Self::Game | Self::Chat | Self::Media | Self::Aux
        )
    }

    /// Returns true if this channel represents an input source (recording/capture).
    #[must_use]
    pub const fn is_source(&self) -> bool {
        matches!(self, Self::Mic | Self::StreamMix)
    }
}

impl fmt::Display for ChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl FromStr for ChannelId {
    type Err = crate::error::CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "master" => Ok(Self::Master),
            "game" => Ok(Self::Game),
            "chat" => Ok(Self::Chat),
            "media" => Ok(Self::Media),
            "aux" => Ok(Self::Aux),
            "mic" | "microphone" => Ok(Self::Mic),
            "stream" | "streammix" | "stream_mix" => Ok(Self::StreamMix),
            _ => Err(crate::error::CoreError::OutOfRange(format!(
                "Unknown channel identifier '{s}'"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_id_parsing() {
        assert_eq!("game".parse::<ChannelId>().unwrap(), ChannelId::Game);
        assert_eq!("CHAT".parse::<ChannelId>().unwrap(), ChannelId::Chat);
        assert_eq!("mic".parse::<ChannelId>().unwrap(), ChannelId::Mic);
        assert!("unknown".parse::<ChannelId>().is_err());
    }

    #[test]
    fn test_channel_types() {
        assert!(ChannelId::Game.is_sink());
        assert!(!ChannelId::Game.is_source());
        assert!(ChannelId::Mic.is_source());
        assert!(!ChannelId::Mic.is_sink());
    }
}
