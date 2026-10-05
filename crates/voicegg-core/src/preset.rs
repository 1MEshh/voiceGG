//! Audio preset representations and serialization.

use crate::dsp_types::{
    CompressorConfig, EqConfig, LimiterConfig, NoiseCancellerConfig, NoiseGateConfig,
};
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

/// Categories for VoiceGG presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresetCategory {
    /// Presets tailored for specific games or gaming audio.
    Game,
    /// Presets tailored for voice chat clarity and intelligibility.
    Chat,
    /// Presets tailored for music, movies, or media consumption.
    Media,
    /// Presets tailored for microphone processing and voice broadcast.
    Mic,
    /// User-created custom presets.
    Custom,
}

/// A complete audio processing preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    /// Schema version for forward/backward compatibility.
    pub schema: u32,
    /// Unique identifier for the preset.
    pub id: String,
    /// Human-readable title of the preset.
    pub name: String,
    /// Optional description of the sound profile or intended game.
    #[serde(default)]
    pub description: String,
    /// Primary category of the preset.
    pub category: PresetCategory,
    /// Tags for categorization and quick search (e.g. "fps", "footsteps", "apex").
    #[serde(default)]
    pub tags: Vec<String>,
    /// Equalizer settings.
    pub eq: EqConfig,
    /// Optional noise cancellation settings (primarily used for Mic and Chat).
    #[serde(default)]
    pub noise_canceller: Option<NoiseCancellerConfig>,
    /// Optional compressor settings (primarily used for Mic).
    #[serde(default)]
    pub compressor: Option<CompressorConfig>,
    /// Optional noise gate settings (primarily used for Mic).
    #[serde(default)]
    pub noise_gate: Option<NoiseGateConfig>,
    /// Optional output limiter settings.
    #[serde(default)]
    pub limiter: Option<LimiterConfig>,
}

impl Preset {
    /// Current schema version.
    pub const CURRENT_SCHEMA: u32 = 1;

    /// Validates preset values to ensure they do not exceed safe DSP limits.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(CoreError::InvalidPreset(
                "Preset name cannot be empty".to_string(),
            ));
        }
        if self.id.trim().is_empty() {
            return Err(CoreError::InvalidPreset(
                "Preset ID cannot be empty".to_string(),
            ));
        }
        self.eq.validate()?;
        Ok(())
    }
}
