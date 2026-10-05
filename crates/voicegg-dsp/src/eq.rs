//! 10-Band Parametric Equalizer processor with parameter smoothing.

use crate::biquad::{BiquadFilter, BiquadParams};
use voicegg_core::EqConfig;

const MAX_BANDS: usize = 10;

/// Cascaded 10-band parametric equalizer.
#[derive(Debug, Clone)]
pub struct ParametricEq {
    sample_rate: f32,
    filters: [BiquadFilter; MAX_BANDS],
    enabled: bool,
}

impl ParametricEq {
    /// Creates a new parametric equalizer initialized with flat 10-band response.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let filters = [BiquadFilter::default(); MAX_BANDS];
        let mut eq = Self {
            sample_rate,
            filters,
            enabled: true,
        };
        let default_cfg = EqConfig::flat_10_band();
        eq.update_config(&default_cfg);
        eq
    }

    /// Sets the sample rate and recalculates filter coefficients.
    pub fn set_sample_rate(&mut self, sample_rate: f32, config: &EqConfig) {
        self.sample_rate = sample_rate;
        self.update_config(config);
    }

    /// Updates the equalizer bands and quick tone sliders.
    pub fn update_config(&mut self, config: &EqConfig) {
        self.enabled = config.enabled;
        if !self.enabled {
            return;
        }

        for (i, band) in config.bands.iter().take(MAX_BANDS).enumerate() {
            if !band.enabled {
                self.filters[i].set_params(BiquadParams::identity());
                continue;
            }

            // Apply quick tone adjustments:
            // Bass: influences bands <= 250 Hz
            // Voice: influences bands 500 .. 2000 Hz
            // Treble: influences bands >= 4000 Hz
            let tone_gain = if band.freq_hz <= 250.0 {
                config.bass_db
            } else if (500.0..=2000.0).contains(&band.freq_hz) {
                config.voice_db
            } else if band.freq_hz >= 4000.0 {
                config.treble_db
            } else {
                0.0
            };

            let effective_gain = (band.gain_db + tone_gain).clamp(-24.0, 24.0);

            let params = BiquadParams::compute(
                band.filter_type,
                self.sample_rate,
                band.freq_hz,
                effective_gain,
                band.q,
            );

            self.filters[i].set_params(params);
        }
    }

    /// Resets filter delay registers to zero.
    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
    }

    /// Processes a single audio sample in-place through all cascaded bands.
    #[inline(always)]
    pub fn process_sample(&mut self, mut sample: f32) -> f32 {
        if !self.enabled {
            return sample;
        }
        for filter in &mut self.filters {
            sample = filter.process_sample(sample);
        }
        sample
    }

    /// Processes an audio slice in-place.
    #[inline]
    pub fn process_slice(&mut self, buffer: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_eq_passthrough() {
        let mut eq = ParametricEq::new(48000.0);
        let mut buffer = vec![0.5_f32; 128];
        let original = buffer.clone();

        eq.process_slice(&mut buffer);

        for (a, b) in buffer.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-4);
        }
    }

    #[test]
    fn test_eq_disabled_bypass() {
        let mut eq = ParametricEq::new(48000.0);
        let mut cfg = EqConfig::flat_10_band();
        cfg.enabled = false;
        eq.update_config(&cfg);

        let mut buffer = vec![0.75_f32; 64];
        let original = buffer.clone();

        eq.process_slice(&mut buffer);
        assert_eq!(buffer, original);
    }
}
