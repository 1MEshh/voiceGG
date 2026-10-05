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

    /// Returns a list of all built-in presets across Game, Chat, Media, and Mic.
    #[must_use]
    pub fn builtin_presets() -> Vec<Preset> {
        vec![
            // Game Presets
            Self::create_game_preset(
                "game_flat",
                "Flat (Gaming)",
                "Neutral reference response without frequency coloration",
                &["flat", "reference", "neutral"],
                [0.0; 10],
                0.0, 0.0, 0.0,
            ),
            Self::create_game_preset(
                "game_cs2",
                "Counter-Strike 2",
                "Footstep isolation, bomb defusal cues, and gunshot punch for CS2",
                &["cs2", "tactical", "footsteps", "fps"],
                [-3.0, -1.5, 0.0, 0.5, 1.5, 3.0, 4.5, 3.5, 1.0, 0.0],
                -1.5, 3.0, 2.0,
            ),
            Self::create_game_preset(
                "game_apex",
                "Apex Legends",
                "Shield cracking, sliding cues, and directional footsteps for Apex",
                &["apex", "battle_royale", "footsteps", "fps"],
                [-2.0, -1.0, 0.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.5, 0.0],
                -1.0, 3.5, 2.0,
            ),
            Self::create_game_preset(
                "game_overwatch2",
                "Overwatch 2",
                "Character footsteps and ultimate audio cue enhancement",
                &["overwatch", "hero_shooter", "cues", "fps"],
                [-1.0, 0.0, 0.5, 1.0, 2.0, 3.5, 4.0, 2.5, 1.0, 0.0],
                0.0, 3.0, 1.5,
            ),
            Self::create_game_preset(
                "game_valorant",
                "Valorant",
                "Precision footstep clarity and ability detection (headphone tuning)",
                &["valorant", "tactical", "footsteps", "fps"],
                [-4.0, -2.0, 0.0, 1.0, 2.0, 3.5, 5.0, 4.0, 1.5, 0.0],
                -2.0, 4.0, 2.5,
            ),
            Self::create_game_preset(
                "game_footsteps",
                "Competitive Footsteps",
                "Universal footstep isolation by attenuating masking bass rumble",
                &["footsteps", "competitive", "fps"],
                [-6.0, -3.0, -1.0, 1.0, 2.5, 4.5, 5.0, 3.0, 1.0, 0.0],
                -4.0, 4.0, 1.5,
            ),
            Self::create_game_preset(
                "game_immersive",
                "Deep Immersion",
                "Cinematic sub-bass rumble with detailed top-end spatial cues",
                &["immersive", "cinematic", "bass", "rpg"],
                [5.0, 4.0, 2.0, 0.0, -1.0, 0.0, 1.5, 2.5, 3.5, 4.0],
                4.0, 0.0, 3.0,
            ),

            // Chat Presets
            Self::create_chat_preset(
                "chat_clear_voice",
                "Clear Voice",
                "Enhances vocal intelligibility by cutting low boom and boosting presence",
                &["voice", "clarity", "discord"],
                [-6.0, -3.0, 0.0, 1.0, 2.5, 3.5, 4.0, 2.5, 0.0, -2.0],
            ),
            Self::create_chat_preset(
                "chat_reduce_boom",
                "Reduce Boom",
                "Cuts muddy 150-300 Hz frequencies from cheap or bass-heavy mics",
                &["voice", "anti_boom", "clean"],
                [-4.0, -3.0, -4.0, -3.0, 0.0, 1.5, 2.0, 1.5, 0.0, 0.0],
            ),
            Self::create_chat_preset(
                "chat_warm",
                "Warm Voice",
                "Full-bodied, smooth vocal tone for prolonged listening comfort",
                &["voice", "warm", "smooth"],
                [0.0, 1.0, 2.0, 1.5, 0.5, 0.0, 1.0, 1.5, 0.0, -1.0],
            ),

            // Media Presets
            Self::create_media_preset(
                "media_music_bass",
                "Music Bass Boost",
                "Punchy low-end emphasis for electronic, hip-hop, and rock tracks",
                &["music", "bass", "edm", "rock"],
                [5.5, 4.5, 3.0, 1.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.5],
            ),
            Self::create_media_preset(
                "media_podcast",
                "Podcast & Speech",
                "Optimized for dialogue clarity, interviews, and audiobooks",
                &["podcast", "speech", "dialogue"],
                [-4.0, -2.0, 0.0, 1.0, 2.5, 3.0, 3.5, 2.0, -1.0, -3.0],
            ),
            Self::create_media_preset(
                "media_movie",
                "Movie Theater",
                "Expansive dynamic range with impactful cinematic bass and crisp dialogue",
                &["movie", "cinema", "wide"],
                [4.0, 3.0, 1.5, 0.0, 0.0, 1.5, 2.0, 2.5, 3.0, 3.5],
            ),

            // Mic Presets
            Self::create_mic_preset(
                "mic_broadcast",
                "Broadcast Studio",
                "Professional broadcast sound with 75% AI noise removal, gate, and limiter",
                &["mic", "broadcast", "studio", "streaming"],
                75.0,
                -42.0,
                3.0,
                -20.0,
            ),
            Self::create_mic_preset(
                "mic_deep_voice",
                "Deep Radio Voice",
                "Warm low-mid radio presence with gentle compression and noise suppression",
                &["mic", "radio", "warm", "podcast"],
                70.0,
                -40.0,
                3.5,
                -18.0,
            ),
            Self::create_mic_preset(
                "mic_streamer",
                "Aggressive Streamer",
                "High noise suppression (85%) and tight gate for noisy keyboard environments",
                &["mic", "streamer", "anti_keyboard", "gaming"],
                85.0,
                -36.0,
                4.0,
                -16.0,
            ),
        ]
    }

    /// Looks up a built-in preset by its unique ID.
    #[must_use]
    pub fn get_preset_by_id(id: &str) -> Option<Preset> {
        Self::builtin_presets().into_iter().find(|p| p.id.eq_ignore_ascii_case(id))
    }

    fn create_game_preset(
        id: &str,
        name: &str,
        desc: &str,
        tags: &[&str],
        gains: [f32; 10],
        bass: f32,
        voice: f32,
        treble: f32,
    ) -> Self {
        Self {
            schema: Self::CURRENT_SCHEMA,
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category: PresetCategory::Game,
            tags: tags.iter().map(|s| (*s).to_string()).collect(),
            eq: Self::make_10_band_eq(gains, bass, voice, treble),
            noise_canceller: None,
            compressor: None,
            noise_gate: None,
            limiter: Some(LimiterConfig {
                enabled: true,
                ceiling_db: -6.0,
                release_ms: 50.0,
            }),
        }
    }

    fn create_chat_preset(
        id: &str,
        name: &str,
        desc: &str,
        tags: &[&str],
        gains: [f32; 10],
    ) -> Self {
        Self {
            schema: Self::CURRENT_SCHEMA,
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category: PresetCategory::Chat,
            tags: tags.iter().map(|s| (*s).to_string()).collect(),
            eq: Self::make_10_band_eq(gains, 0.0, 2.0, 1.0),
            noise_canceller: Some(NoiseCancellerConfig {
                enabled: true,
                amount: 60.0,
                vad_threshold: 0.5,
            }),
            compressor: None,
            noise_gate: None,
            limiter: Some(LimiterConfig {
                enabled: true,
                ceiling_db: -6.0,
                release_ms: 50.0,
            }),
        }
    }

    fn create_media_preset(
        id: &str,
        name: &str,
        desc: &str,
        tags: &[&str],
        gains: [f32; 10],
    ) -> Self {
        Self {
            schema: Self::CURRENT_SCHEMA,
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category: PresetCategory::Media,
            tags: tags.iter().map(|s| (*s).to_string()).collect(),
            eq: Self::make_10_band_eq(gains, 0.0, 0.0, 0.0),
            noise_canceller: None,
            compressor: None,
            noise_gate: None,
            limiter: Some(LimiterConfig {
                enabled: true,
                ceiling_db: -6.0,
                release_ms: 50.0,
            }),
        }
    }

    fn create_mic_preset(
        id: &str,
        name: &str,
        desc: &str,
        tags: &[&str],
        noise_amount: f32,
        gate_threshold: f32,
        comp_ratio: f32,
        comp_threshold: f32,
    ) -> Self {
        Self {
            schema: Self::CURRENT_SCHEMA,
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category: PresetCategory::Mic,
            tags: tags.iter().map(|s| (*s).to_string()).collect(),
            eq: Self::make_10_band_eq(
                [-6.0, -3.0, 0.0, 0.5, 1.5, 2.5, 3.0, 2.0, 1.0, 0.0],
                -2.0,
                2.5,
                1.5,
            ),
            noise_canceller: Some(NoiseCancellerConfig {
                enabled: true,
                amount: noise_amount,
                vad_threshold: 0.5,
            }),
            compressor: Some(CompressorConfig {
                enabled: true,
                threshold_db: comp_threshold,
                ratio: comp_ratio,
                attack_ms: 5.0,
                release_ms: 60.0,
                makeup_gain_db: 3.0,
            }),
            noise_gate: Some(NoiseGateConfig {
                enabled: true,
                threshold_db: gate_threshold,
                attack_ms: 2.0,
                hold_ms: 50.0,
                release_ms: 80.0,
            }),
            limiter: Some(LimiterConfig {
                enabled: true,
                ceiling_db: -1.0,
                release_ms: 30.0,
            }),
        }
    }

    fn make_10_band_eq(gains: [f32; 10], bass: f32, voice: f32, treble: f32) -> EqConfig {
        use crate::dsp_types::{EqBand, FilterType};
        let freqs = [
            (32.0, FilterType::LowShelf),
            (64.0, FilterType::Peak),
            (125.0, FilterType::Peak),
            (250.0, FilterType::Peak),
            (500.0, FilterType::Peak),
            (1000.0, FilterType::Peak),
            (2000.0, FilterType::Peak),
            (4000.0, FilterType::Peak),
            (8000.0, FilterType::Peak),
            (16000.0, FilterType::HighShelf),
        ];

        let bands = freqs
            .iter()
            .zip(gains.iter())
            .map(|(&(freq_hz, filter_type), &gain_db)| EqBand {
                freq_hz,
                gain_db,
                q: 1.0,
                filter_type,
                enabled: true,
            })
            .collect();

        EqConfig {
            enabled: true,
            bands,
            bass_db: bass,
            voice_db: voice,
            treble_db: treble,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_builtin_presets_are_valid() {
        let presets = Preset::builtin_presets();
        assert!(!presets.is_empty(), "Builtin presets should not be empty");
        for p in &presets {
            assert!(
                p.validate().is_ok(),
                "Preset '{}' failed validation: {:?}",
                p.name,
                p.validate()
            );
        }
    }

    #[test]
    fn test_lookup_game_preset() {
        let cs2 = Preset::get_preset_by_id("game_cs2");
        assert!(cs2.is_some());
        assert_eq!(cs2.unwrap().category, PresetCategory::Game);
    }

    #[test]
    fn test_export_builtin_presets_to_repo() {
        let repo_presets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("presets");
        if repo_presets.exists() {
            for p in Preset::builtin_presets() {
                let file_path = repo_presets.join(format!("{}.json", p.id));
                let json = serde_json::to_string_pretty(&p).unwrap();
                let _ = std::fs::write(file_path, json);
            }
        }
    }
}
