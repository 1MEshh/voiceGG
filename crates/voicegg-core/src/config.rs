//! Persistent application configuration and user settings.

use crate::channel::ChannelId;
use crate::preset::Preset;
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

fn default_language() -> String {
    "en".to_string()
}

/// Global VoiceGG state and configuration saved to disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceggConfig {
    /// Schema version for future migrations.
    pub version: u32,
    /// Selected UI language code (e.g. "en", "ar").
    #[serde(default = "default_language")]
    pub language: String,
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
    /// User customized presets persisted across sessions.
    #[serde(default)]
    pub custom_presets: HashMap<String, Preset>,
    /// Individual application volume overrides (binary_name -> volume percentage 0..150).
    #[serde(default)]
    pub app_volumes: HashMap<String, u8>,
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
            language: "en".to_string(),
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
            custom_presets: HashMap::new(),
            app_volumes: HashMap::new(),
        }
    }
}

impl VoiceggConfig {
    /// Returns the configured volume for a given application binary (defaults to 100).
    #[must_use]
    pub fn get_app_volume(&self, binary_name: &str) -> u8 {
        self.app_volumes.get(binary_name).copied().unwrap_or(100)
    }

    /// Sets the volume override for a given application binary.
    pub fn set_app_volume(&mut self, binary_name: String, volume: u8) {
        self.app_volumes.insert(binary_name, volume);
    }
    /// Returns the standard config file path ($XDG_CONFIG_HOME/voicegg/config.toml).
    #[must_use]
    pub fn config_path() -> std::path::PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join("voicegg").join("config.toml")
    }

    /// Loads the configuration from disk, creating default config if it doesn't exist.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<Self>(&content) {
                    return config;
                }
                // Backup corrupt file and recover gracefully
                let corrupt_path = path.with_extension("toml.corrupt");
                let _ = std::fs::rename(&path, corrupt_path);
            }
        }
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    /// Atomically saves the configuration to disk.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let tmp_path = path.with_extension("toml.tmp");
        std::fs::write(&tmp_path, serialized)?;
        std::fs::rename(tmp_path, path)?;
        Ok(())
    }
}
