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
    /// Input preamp gain in decibels.
    #[serde(default)]
    pub input_gain_db: f32,
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
            input_gain_db: 0.0,
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

fn default_knee_db() -> f32 {
    3.0
}

fn default_hysteresis_db() -> f32 {
    6.0
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
    /// Soft knee transition width in decibels (0.0 .. 12.0).
    #[serde(default = "default_knee_db")]
    pub knee_db: f32,
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
            knee_db: 3.0,
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
    /// Hysteresis difference between open and close threshold in decibels (0.0 .. 24.0).
    #[serde(default = "default_hysteresis_db")]
    pub hysteresis_db: f32,
}

impl Default for NoiseGateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -45.0,
            attack_ms: 2.0,
            hold_ms: 50.0,
            release_ms: 80.0,
            hysteresis_db: 6.0,
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

/// Available DSP plugin capabilities detected on the host system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilities {
    /// Whether RNNoise LADSPA plugin is installed.
    pub rnnoise: bool,
    /// Whether a noise gate plugin is installed.
    pub gate: bool,
    /// Whether a compressor plugin is installed.
    pub compressor: bool,
    /// Whether PipeWire builtin biquad EQ is available.
    pub biquad_eq: bool,
}

impl Default for PluginCapabilities {
    fn default() -> Self {
        Self {
            rnnoise: false,
            gate: false,
            compressor: false,
            biquad_eq: true,
        }
    }
}

/// Raw biquad digital IIR filter coefficients (b0, b1, b2, a0, a1, a2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiquadCoefficients {
    /// Feedforward coefficient b0.
    pub b0: f32,
    /// Feedforward coefficient b1.
    pub b1: f32,
    /// Feedforward coefficient b2.
    pub b2: f32,
    /// Feedback coefficient a0.
    pub a0: f32,
    /// Feedback coefficient a1.
    pub a1: f32,
    /// Feedback coefficient a2.
    pub a2: f32,
}

impl Default for BiquadCoefficients {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl BiquadCoefficients {
    /// Calculates low shelf filter coefficients using RBJ Audio EQ Cookbook formulas.
    #[must_use]
    pub fn low_shelf(freq: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        use std::f32::consts::PI;
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let w0 = 2.0 * PI * freq / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = sin_w0 / (2.0 * q.max(0.001));
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;

        Self {
            b0,
            b1,
            b2,
            a0,
            a1,
            a2,
        }
    }

    /// Calculates peaking EQ filter coefficients using RBJ Audio EQ Cookbook formulas.
    #[must_use]
    pub fn peaking(freq: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        use std::f32::consts::PI;
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let w0 = 2.0 * PI * freq / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = sin_w0 / (2.0 * q.max(0.001));

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self {
            b0,
            b1,
            b2,
            a0,
            a1,
            a2,
        }
    }

    /// Calculates high shelf filter coefficients using RBJ Audio EQ Cookbook formulas.
    #[must_use]
    pub fn high_shelf(freq: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        use std::f32::consts::PI;
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let w0 = 2.0 * PI * freq / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = sin_w0 / (2.0 * q.max(0.001));
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;

        Self {
            b0,
            b1,
            b2,
            a0,
            a1,
            a2,
        }
    }

    /// Calculates high pass filter coefficients using RBJ Audio EQ Cookbook formulas.
    #[must_use]
    pub fn high_pass(freq: f32, q: f32, sample_rate: f32) -> Self {
        use std::f32::consts::PI;
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let w0 = 2.0 * PI * freq / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q.max(0.001));

        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0,
            b1,
            b2,
            a0,
            a1,
            a2,
        }
    }

    /// Calculates low pass filter coefficients using RBJ Audio EQ Cookbook formulas.
    #[must_use]
    pub fn low_pass(freq: f32, q: f32, sample_rate: f32) -> Self {
        use std::f32::consts::PI;
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let w0 = 2.0 * PI * freq / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q.max(0.001));

        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0,
            b1,
            b2,
            a0,
            a1,
            a2,
        }
    }

    /// Computes the magnitude response in decibels (dB) at a given frequency.
    #[must_use]
    pub fn magnitude_response_db(&self, freq: f32, sample_rate: f32) -> f32 {
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let phi = 2.0 * std::f32::consts::PI * freq / sr;
        let cos1 = phi.cos();
        let cos2 = (2.0 * phi).cos();
        let sin1 = phi.sin();
        let sin2 = (2.0 * phi).sin();

        let num_re = self.b0 + self.b1 * cos1 + self.b2 * cos2;
        let num_im = -self.b1 * sin1 - self.b2 * sin2;
        let den_re = self.a0 + self.a1 * cos1 + self.a2 * cos2;
        let den_im = -self.a1 * sin1 - self.a2 * sin2;

        let num_mag_sq = num_re * num_re + num_im * num_im;
        let den_mag_sq = den_re * den_re + den_im * den_im;

        if den_mag_sq <= 1e-20 {
            return 0.0;
        }
        let ratio = num_mag_sq / den_mag_sq;
        if ratio <= 1e-20 {
            return -100.0;
        }
        10.0 * ratio.log10()
    }

    /// Returns a normalized copy of the coefficients where `a0 == 1.0`.
    #[must_use]
    pub fn normalize(&self) -> Self {
        if self.a0.abs() > 1e-12 {
            let inv_a0 = 1.0 / self.a0;
            Self {
                b0: self.b0 * inv_a0,
                b1: self.b1 * inv_a0,
                b2: self.b2 * inv_a0,
                a0: 1.0,
                a1: self.a1 * inv_a0,
                a2: self.a2 * inv_a0,
            }
        } else {
            *self
        }
    }
}

/// Spatial 3D virtualization and immersion configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialConfig {
    /// Whether 3D spatial surround virtualization is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Spatial immersion percentage (0 .. 100).
    #[serde(default = "default_immersion")]
    pub immersion_pct: u8,
}

fn default_immersion() -> u8 {
    50
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            immersion_pct: 50,
        }
    }
}

/// Smart dynamic volume leveling and normalization configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartVolumeConfig {
    /// Whether dynamic volume leveling is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Normalization mode ("quiet", "balanced", "loud").
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "balanced".to_string()
}

impl Default for SmartVolumeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "balanced".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peaking_filter_magnitude_response() {
        let sr = 48000.0;
        let freq = 1000.0;
        let gain_db = 6.0;
        let q = 1.0;
        let filter = BiquadCoefficients::peaking(freq, q, gain_db, sr);

        // At center frequency, response should equal the target gain (+6 dB)
        let resp_center = filter.magnitude_response_db(freq, sr);
        assert!(
            (resp_center - gain_db).abs() < 0.05,
            "Expected ~{} dB at center, got {}",
            gain_db,
            resp_center
        );

        // Far away from center, response should approach 0 dB
        let resp_dc = filter.magnitude_response_db(10.0, sr);
        assert!(
            resp_dc.abs() < 0.2,
            "Expected ~0 dB far below center, got {}",
            resp_dc
        );

        let normalized = filter.normalize();
        assert!((normalized.a0 - 1.0).abs() < 1e-6);
        let resp_norm = normalized.magnitude_response_db(freq, sr);
        assert!((resp_norm - resp_center).abs() < 1e-4);
    }

    #[test]
    fn test_low_shelf_magnitude_response() {
        let sr = 48000.0;
        let freq = 100.0;
        let gain_db = 5.0;
        let filter = BiquadCoefficients::low_shelf(freq, 0.707, gain_db, sr);

        // At DC / very low freq, gain should be ~5 dB
        let resp_low = filter.magnitude_response_db(10.0, sr);
        assert!(
            (resp_low - gain_db).abs() < 0.2,
            "Expected ~{} dB, got {}",
            gain_db,
            resp_low
        );

        // At high frequencies, shelf should approach 0 dB
        let resp_high = filter.magnitude_response_db(10000.0, sr);
        assert!(
            resp_high.abs() < 0.2,
            "Expected ~0 dB at high freq, got {}",
            resp_high
        );
    }

    #[test]
    fn test_high_shelf_magnitude_response() {
        let sr = 48000.0;
        let freq = 8000.0;
        let gain_db = -4.0;
        let filter = BiquadCoefficients::high_shelf(freq, 0.707, gain_db, sr);

        // At DC, gain should approach 0 dB
        let resp_low = filter.magnitude_response_db(100.0, sr);
        assert!(
            resp_low.abs() < 0.2,
            "Expected ~0 dB at low freq, got {}",
            resp_low
        );

        // At 20 kHz, shelf should approach -4 dB
        let resp_high = filter.magnitude_response_db(20000.0, sr);
        assert!(
            (resp_high - gain_db).abs() < 0.5,
            "Expected ~{} dB at high freq, got {}",
            gain_db,
            resp_high
        );
    }

    #[test]
    fn test_high_pass_low_pass() {
        let sr = 48000.0;
        let hp = BiquadCoefficients::high_pass(1000.0, 0.707, sr);
        let lp = BiquadCoefficients::low_pass(1000.0, 0.707, sr);

        // HPF attenuates 100 Hz
        assert!(hp.magnitude_response_db(100.0, sr) < -30.0);
        // HPF passes 10 kHz
        assert!(hp.magnitude_response_db(10000.0, sr).abs() < 0.5);

        // LPF passes 100 Hz
        assert!(lp.magnitude_response_db(100.0, sr).abs() < 0.5);
        // LPF attenuates 10 kHz
        assert!(lp.magnitude_response_db(10000.0, sr) < -30.0);
    }

    #[test]
    fn test_spatial_and_smart_volume_defaults() {
        let spatial = SpatialConfig::default();
        assert!(!spatial.enabled);
        assert_eq!(spatial.immersion_pct, 50);

        let smart_vol = SmartVolumeConfig::default();
        assert!(!smart_vol.enabled);
        assert_eq!(smart_vol.mode, "balanced");

        let eq = EqConfig::default();
        assert_eq!(eq.input_gain_db, 0.0);
    }
}
