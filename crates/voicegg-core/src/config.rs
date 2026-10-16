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

fn default_personal_mix() -> HashMap<ChannelId, bool> {
    let mut map = HashMap::new();
    for channel in [
        ChannelId::Master,
        ChannelId::Game,
        ChannelId::Chat,
        ChannelId::Media,
        ChannelId::Aux,
        ChannelId::Mic,
        ChannelId::StreamMix,
    ] {
        map.insert(channel, true);
    }
    map
}

fn default_stream_mix() -> HashMap<ChannelId, bool> {
    let mut map = HashMap::new();
    for channel in [
        ChannelId::Master,
        ChannelId::Game,
        ChannelId::Chat,
        ChannelId::Media,
        ChannelId::Aux,
        ChannelId::Mic,
        ChannelId::StreamMix,
    ] {
        map.insert(channel, true);
    }
    map
}

fn default_favorite_presets() -> HashMap<ChannelId, Vec<String>> {
    let mut map = HashMap::new();
    map.insert(ChannelId::Master, vec!["flat".to_string()]);
    map.insert(
        ChannelId::Game,
        vec![
            "game_flat".to_string(),
            "game_cs2".to_string(),
            "game_apex".to_string(),
        ],
    );
    map.insert(
        ChannelId::Chat,
        vec!["chat_clarity".to_string(), "chat_broadcast".to_string()],
    );
    map.insert(
        ChannelId::Media,
        vec!["media_music".to_string(), "media_movie".to_string()],
    );
    map.insert(ChannelId::Aux, vec!["flat".to_string()]);
    map.insert(
        ChannelId::Mic,
        vec!["mic_podcast".to_string(), "mic_clarity".to_string()],
    );
    map.insert(ChannelId::StreamMix, vec!["flat".to_string()]);
    map
}

/// Calculates the effective volume for a channel taking ChatMix into account (-100 Game to +100 Chat).
#[must_use]
#[allow(clippy::collapsible_match)]
pub fn calculate_chatmix_volume(channel: ChannelId, base_volume: u8, chatmix: i8) -> u8 {
    let base = f32::from(base_volume);
    let factor = match channel {
        ChannelId::Game => {
            if chatmix > 0 {
                1.0 - (f32::from(chatmix) / 100.0)
            } else {
                1.0
            }
        }
        ChannelId::Chat => {
            if chatmix < 0 {
                1.0 + (f32::from(chatmix) / 100.0)
            } else {
                1.0
            }
        }
        _ => 1.0,
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        ((base * factor).round() as u8).min(150)
    }
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
    /// Whether streamer dual-mix mode is active.
    #[serde(default)]
    pub streamer_mode: bool,
    /// Personal headphone monitor mix channel toggles.
    #[serde(default = "default_personal_mix")]
    pub personal_mix: HashMap<ChannelId, bool>,
    /// Stream audience broadcast mix channel toggles.
    #[serde(default = "default_stream_mix")]
    pub stream_mix: HashMap<ChannelId, bool>,
    /// Favorite presets per channel for quick switching.
    #[serde(default = "default_favorite_presets")]
    pub favorite_presets: HashMap<ChannelId, Vec<String>>,
}

impl Default for VoiceggConfig {
    fn default() -> Self {
        let mut volumes = HashMap::new();
        let mut muted = HashMap::new();
        let mut active_presets = HashMap::new();
        let mut personal_mix = HashMap::new();
        let mut stream_mix = HashMap::new();

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
            personal_mix.insert(channel, true);
            stream_mix.insert(channel, true);
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
            streamer_mode: false,
            personal_mix,
            stream_mix,
            favorite_presets: default_favorite_presets(),
        }
    }
}

impl VoiceggConfig {
    /// Returns the effective output volume for a channel taking ChatMix into account.
    #[must_use]
    pub fn effective_volume(&self, channel: ChannelId) -> u8 {
        let base_vol = self.volumes.get(&channel).copied().unwrap_or(100);
        calculate_chatmix_volume(channel, base_vol, self.chatmix)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_chatmix_volume_neutral() {
        assert_eq!(calculate_chatmix_volume(ChannelId::Game, 100, 0), 100);
        assert_eq!(calculate_chatmix_volume(ChannelId::Chat, 100, 0), 100);
        assert_eq!(calculate_chatmix_volume(ChannelId::Media, 100, 0), 100);
    }

    #[test]
    fn test_calculate_chatmix_volume_favor_chat() {
        // Chatmix > 0 favors chat, reduces game
        assert_eq!(calculate_chatmix_volume(ChannelId::Game, 100, 50), 50);
        assert_eq!(calculate_chatmix_volume(ChannelId::Game, 100, 100), 0);
        assert_eq!(calculate_chatmix_volume(ChannelId::Chat, 100, 50), 100);
        assert_eq!(calculate_chatmix_volume(ChannelId::Media, 100, 50), 100);
    }

    #[test]
    fn test_calculate_chatmix_volume_favor_game() {
        // Chatmix < 0 favors game, reduces chat
        assert_eq!(calculate_chatmix_volume(ChannelId::Game, 100, -50), 100);
        assert_eq!(calculate_chatmix_volume(ChannelId::Chat, 100, -50), 50);
        assert_eq!(calculate_chatmix_volume(ChannelId::Chat, 100, -100), 0);
        assert_eq!(calculate_chatmix_volume(ChannelId::Media, 100, -50), 100);
    }

    #[test]
    fn test_effective_volume_method() {
        let mut config = VoiceggConfig::default();
        config.chatmix = 50;
        config.volumes.insert(ChannelId::Game, 80);
        config.volumes.insert(ChannelId::Chat, 90);

        // Game reduced by 50% from 80 -> 40
        assert_eq!(config.effective_volume(ChannelId::Game), 40);
        // Chat remains 90
        assert_eq!(config.effective_volume(ChannelId::Chat), 90);
    }

    #[test]
    fn test_streamer_mode_and_mix_defaults() {
        let config = VoiceggConfig::default();
        assert!(!config.streamer_mode);

        for channel in [
            ChannelId::Master,
            ChannelId::Game,
            ChannelId::Chat,
            ChannelId::Media,
            ChannelId::Aux,
            ChannelId::Mic,
            ChannelId::StreamMix,
        ] {
            assert_eq!(config.personal_mix.get(&channel), Some(&true));
            assert_eq!(config.stream_mix.get(&channel), Some(&true));
            assert!(config.favorite_presets.contains_key(&channel));
        }
    }
}
