//! UI theme and per-channel color customization.

use crate::channel::ChannelId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Visual styling and accent color configuration for a specific audio channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelTheme {
    /// Primary accent color in hex format (e.g. "#10b981").
    pub accent_hex: String,
    /// Secondary or gradient end color in hex format.
    pub secondary_hex: String,
}

/// Overall application theme configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Name of the theme (e.g. "Sonar Dark", "Catppuccin Mocha", "Pywal").
    pub name: String,
    /// Background color in hex format.
    pub background_hex: String,
    /// Surface / card container color in hex format.
    pub surface_hex: String,
    /// Text primary color in hex format.
    pub text_primary_hex: String,
    /// Channel-specific accent colors.
    pub channels: HashMap<ChannelId, ChannelTheme>,
    /// Meter display style ("gradient", "solid", "segmented").
    pub meter_style: String,
    /// Whether to dynamically follow pywal cache (`~/.cache/wal/colors.json`).
    pub follow_pywal: bool,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        let mut channels = HashMap::new();
        channels.insert(
            ChannelId::Master,
            ChannelTheme {
                accent_hex: "#6366f1".to_string(), // Indigo
                secondary_hex: "#818cf8".to_string(),
            },
        );
        channels.insert(
            ChannelId::Game,
            ChannelTheme {
                accent_hex: "#10b981".to_string(), // Emerald green
                secondary_hex: "#34d399".to_string(),
            },
        );
        channels.insert(
            ChannelId::Chat,
            ChannelTheme {
                accent_hex: "#06b6d4".to_string(), // Cyan
                secondary_hex: "#22d3ee".to_string(),
            },
        );
        channels.insert(
            ChannelId::Media,
            ChannelTheme {
                accent_hex: "#ec4899".to_string(), // Pink
                secondary_hex: "#f472b6".to_string(),
            },
        );
        channels.insert(
            ChannelId::Aux,
            ChannelTheme {
                accent_hex: "#8b5cf6".to_string(), // Purple
                secondary_hex: "#a78bfa".to_string(),
            },
        );
        channels.insert(
            ChannelId::Mic,
            ChannelTheme {
                accent_hex: "#f59e0b".to_string(), // Amber / Orange
                secondary_hex: "#fbbf24".to_string(),
            },
        );
        channels.insert(
            ChannelId::StreamMix,
            ChannelTheme {
                accent_hex: "#ef4444".to_string(), // Red
                secondary_hex: "#f87171".to_string(),
            },
        );

        Self {
            name: "Default Dark".to_string(),
            background_hex: "#121418".to_string(),
            surface_hex: "#1a1e24".to_string(),
            text_primary_hex: "#f3f4f6".to_string(),
            channels,
            meter_style: "gradient".to_string(),
            follow_pywal: false,
        }
    }
}
