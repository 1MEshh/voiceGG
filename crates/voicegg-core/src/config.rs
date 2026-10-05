//! Persistent application configuration and user settings.

use crate::channel::ChannelId;
use crate::theme::ThemeConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An application stream routing rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRouteRule {
    /// Binary executable name (e.g. "discord", "firefox", "cs2").
    pub binary_name: String,
    /// Destination channel to automatically route audio to.
    pub target_channel: ChannelId,
    /// Custom volume multiplier (0.0 .. 2.0).
    #[serde(default = "default_app_volume")]
    pub volume: u8,
}

fn default_app_volume() -> u8 {
    100
}

/// Global VoiceGG state and configuration saved to disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceggConfig {
    /// Schema version for future migrations.
    pub version: u32,
    /// Application routing rules.
    pub routing_rules: Vec<AppRouteRule>,
    /// Active preset IDs per channel.
    pub active_presets: HashMap<ChannelId, String>,
    /// Volume per channel in percentage (0 .. 150).
    pub volumes: HashMap<ChannelId, u8>,
    /// Mute state per channel.
    pub muted: HashMap<ChannelId, bool>,
    /// ChatMix value between -100 (Full Game) and +100 (Full Chat). 0 is balanced.
    pub chatmix: i8,
    /// Auto game detection enabled.
    pub auto_game_detection: bool,
    /// Low-latency buffer quantum (e.g. 128 or 256).
    pub buffer_quantum: u32,
    /// Preferred physical audio output sink name.
    pub preferred_output_device: Option<String>,
    /// Preferred physical audio input source name.
    pub preferred_input_device: Option<String>,
    /// Visual UI theme settings.
    pub theme: ThemeConfig,
}

impl Default for VoiceggConfig {
    fn default() -> Self {
        let mut volumes = HashMap::new();
        let mut muted = HashMap::new();
        let mut active_presets = HashMap::new();

        for channel in [
            ChannelId::Master,
            ChannelId::Game,
            ChannelId::Chat,
            ChannelId::Media,
            ChannelId::Aux,
            ChannelId::Mic,
            ChannelId::StreamMix,
        ] {
            volumes.insert(channel, 100);
            muted.insert(channel, false);
            active_presets.insert(channel, "flat".to_string());
        }

        // Sensible default routing rules
        let routing_rules = vec![
            AppRouteRule {
                binary_name: "discord".to_string(),
                target_channel: ChannelId::Chat,
                volume: 100,
            },
            AppRouteRule {
                binary_name: "vesktop".to_string(),
                target_channel: ChannelId::Chat,
                volume: 100,
            },
            AppRouteRule {
                binary_name: "spotify".to_string(),
                target_channel: ChannelId::Media,
                volume: 100,
            },
            AppRouteRule {
                binary_name: "firefox".to_string(),
                target_channel: ChannelId::Media,
                volume: 100,
            },
            AppRouteRule {
                binary_name: "chromium".to_string(),
                target_channel: ChannelId::Media,
                volume: 100,
            },
            AppRouteRule {
                binary_name: "chrome".to_string(),
                target_channel: ChannelId::Media,
                volume: 100,
            },
            AppRouteRule {
                binary_name: "cs2".to_string(),
                target_channel: ChannelId::Game,
                volume: 100,
            },
        ];

        Self {
            version: 1,
            routing_rules,
            active_presets,
            volumes,
            muted,
            chatmix: 0,
            auto_game_detection: true,
            buffer_quantum: 256,
            preferred_output_device: None,
            preferred_input_device: None,
            theme: ThemeConfig::default(),
        }
    }
}
