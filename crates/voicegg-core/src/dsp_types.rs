//! DSP configuration structures and filter types.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

/// Supported biquad filter types for the parametric equalizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterType {
    /// Peaking EQ filter.
    Peak,
    /// Low shelf filter.
    LowShelf,
    /// High shelf filter.
    HighShelf,
    /// Low pass filter (high cut).
    LowPass,
    /// High pass filter (low cut).
    HighPass,
    /// Notch / band reject filter.
    Notch,
}

/// A single parametric EQ band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    /// Center / cutoff frequency in Hertz (20.0 .. 20000.0).
    pub freq_hz: f32,
    /// Gain in decibels (-24.0 .. +24.0).
    pub gain_db: f32,
    /// Quality factor Q (0.1 .. 10.0).
    pub q: f32,
    /// Type of filter for this band.
    pub filter_type: FilterType,
    /// Whether this specific band is active.
    pub enabled: bool,
}

impl Default for EqBand {
    fn default() -> Self {
        Self {
            freq_hz: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            filter_type: FilterType::Peak,
            enabled: true,
        }
    }
}

impl EqBand {
    /// Creates a new EQ band with boundary validation.
    pub fn new(freq_hz: f32, gain_db: f32, q: f32, filter_type: FilterType) -> Result<Self> {
        let band = Self {
            freq_hz,
            gain_db,
            q,
            filter_type,
            enabled: true,
        };
        band.validate()?;
        Ok(band)
    }

    /// Validates parameter ranges to prevent invalid audio DSP configurations.
    pub fn validate(&self) -> Result<()> {
        if !(10.0..=24000.0).contains(&self.freq_hz) {
            return Err(CoreError::OutOfRange(format!(
                "Frequency {} Hz out of bounds [10, 24000]",
                self.freq_hz
            )));
        }
        if !(-30.0..=30.0).contains(&self.gain_db) {
            return Err(CoreError::OutOfRange(format!(
                "Gain {} dB out of bounds [-30, +30]",
                self.gain_db
            )));
        }
        if !(0.05..=20.0).contains(&self.q) {
            return Err(CoreError::OutOfRange(format!(
                "Q factor {} out of bounds [0.05, 20.0]",
                self.q
            )));
        }
        Ok(())
    }
}

/// 10-Band Parametric Equalizer Configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqConfig {
    /// Global equalizer toggle.
    pub enabled: bool,
    /// Individual bands (typically up to 10 bands).
    pub bands: Vec<EqBand>,
    /// Quick Bass slider (-12.0 .. +12.0 dB).
    pub bass_db: f32,
    /// Quick Voice slider (-12.0 .. +12.0 dB).
    pub voice_db: f32,
    /// Quick Treble slider (-12.0 .. +12.0 dB).
    pub treble_db: f32,
}

impl Default for EqConfig {
    fn default() -> Self {
        Self::flat_10_band()
    }
}

impl EqConfig {
    /// Creates a default flat 10-band equalizer configuration.
    #[must_use]
    pub fn flat_10_band() -> Self {
        let default_freqs = [
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

        let bands = default_freqs
            .iter()
            .map(|&(freq, filter_type)| EqBand {
                freq_hz: freq,
                gain_db: 0.0,
                q: 1.0,
                filter_type,
                enabled: true,
            })
            .collect();

        Self {
            enabled: true,
            bands,
            bass_db: 0.0,
            voice_db: 0.0,
            treble_db: 0.0,
        }
    }

    /// Validates all bands in the configuration.
    pub fn validate(&self) -> Result<()> {
        for band in &self.bands {
            band.validate()?;
        }
        Ok(())
    }
}

/// AI Noise Cancellation configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseCancellerConfig {
    /// Global on/off toggle.
    pub enabled: bool,
    /// Reduction amount from 0.0 (bypass/dry) to 100.0 (maximum suppression).
    pub amount: f32,
    /// Voice activity detection threshold (0.0 .. 1.0).
    pub vad_threshold: f32,
}

impl Default for NoiseCancellerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            amount: 75.0,
            vad_threshold: 0.5,
        }
    }
}

/// Dynamic range compressor configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressorConfig {
    /// Global toggle.
    pub enabled: bool,
    /// Threshold in dBFS (-60.0 .. 0.0).
    pub threshold_db: f32,
    /// Compression ratio (1.0 .. 20.0).
    pub ratio: f32,
    /// Attack time in milliseconds (0.1 .. 100.0).
    pub attack_ms: f32,
    /// Release time in milliseconds (10.0 .. 1000.0).
    pub release_ms: f32,
    /// Makeup gain in decibels (0.0 .. 24.0).
    pub makeup_gain_db: f32,
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain_db: 0.0,
        }
    }
}

/// Noise Gate configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseGateConfig {
    /// Global toggle.
    pub enabled: bool,
    /// Threshold in dBFS (-80.0 .. 0.0).
    pub threshold_db: f32,
    /// Attack time in milliseconds.
    pub attack_ms: f32,
    /// Hold time in milliseconds before release begins.
    pub hold_ms: f32,
    /// Release time in milliseconds.
    pub release_ms: f32,
}

impl Default for NoiseGateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -45.0,
            attack_ms: 2.0,
            hold_ms: 50.0,
            release_ms: 80.0,
        }
    }
}

/// Output true-peak limiter / clip guard configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimiterConfig {
    /// Global toggle.
    pub enabled: bool,
    /// Ceiling in dBFS (e.g. -1.0 dBFS).
    pub ceiling_db: f32,
    /// Release time in milliseconds.
    pub release_ms: f32,
}

impl Default for LimiterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ceiling_db: -1.0,
            release_ms: 50.0,
        }
    }
}
