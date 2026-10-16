//! 10-Band Parametric Equalizer processor with parameter smoothing.

use crate::biquad::{BiquadFilter, BiquadParams};
use voicegg_core::{EqConfig, FilterType};

const MAX_BANDS: usize = 10;

/// Cascaded 10-band parametric equalizer with input gain and macro tone shaping.
#[derive(Debug, Clone)]
pub struct ParametricEq {
    sample_rate: f32,
    filters: [BiquadFilter; MAX_BANDS],
    bass_filter: BiquadFilter,
    voice_filter: BiquadFilter,
    treble_filter: BiquadFilter,
    input_gain_lin: f32,
    enabled: bool,
}

impl ParametricEq {
    /// Creates a new parametric equalizer initialized with flat 10-band response.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let filters = [BiquadFilter::default(); MAX_BANDS];
        let bass_filter = BiquadFilter::new(BiquadParams::compute(
            FilterType::LowShelf,
            sample_rate,
            120.0,
            0.0,
            0.707,
        ));
        let voice_filter = BiquadFilter::new(BiquadParams::compute(
            FilterType::Peak,
            sample_rate,
            1200.0,
            0.0,
            1.0,
        ));
        let treble_filter = BiquadFilter::new(BiquadParams::compute(
            FilterType::HighShelf,
            sample_rate,
            6000.0,
            0.0,
            0.707,
        ));
        let mut eq = Self {
            sample_rate,
            filters,
            bass_filter,
            voice_filter,
            treble_filter,
            input_gain_lin: 1.0,
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

    /// Updates the quick macro tone shaping filters (Bass 120Hz LowShelf, Voice 1200Hz Peak, Treble 6000Hz HighShelf).
    pub fn set_macro_tones(&mut self, bass_db: f32, voice_db: f32, treble_db: f32) {
        self.bass_filter.set_params(BiquadParams::compute(
            FilterType::LowShelf,
            self.sample_rate,
            120.0,
            bass_db,
            0.707,
        ));
        self.voice_filter.set_params(BiquadParams::compute(
            FilterType::Peak,
            self.sample_rate,
            1200.0,
            voice_db,
            1.0,
        ));
        self.treble_filter.set_params(BiquadParams::compute(
            FilterType::HighShelf,
            self.sample_rate,
            6000.0,
            treble_db,
            0.707,
        ));
    }

    /// Sets the input preamp gain in decibels.
    pub fn set_input_gain(&mut self, gain_db: f32) {
        self.input_gain_lin = 10.0_f32.powf(gain_db / 20.0);
    }

    /// Returns the current linear input preamp gain factor.
    #[must_use]
    pub fn input_gain_lin(&self) -> f32 {
        self.input_gain_lin
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

            let effective_gain = band.gain_db.clamp(-24.0, 24.0);

            let params = BiquadParams::compute(
                band.filter_type,
                self.sample_rate,
                band.freq_hz,
                effective_gain,
                band.q,
            );

            self.filters[i].set_params(params);
        }

        self.set_macro_tones(config.bass_db, config.voice_db, config.treble_db);
        self.set_input_gain(config.input_gain_db);
    }

    /// Resets filter delay registers to zero.
    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
        self.bass_filter.reset();
        self.voice_filter.reset();
        self.treble_filter.reset();
    }

    /// Processes a single audio sample in-place through input gain, 10 cascaded bands, and macro tones.
    #[inline(always)]
    pub fn process_sample(&mut self, mut sample: f32) -> f32 {
        if !self.enabled {
            return sample;
        }
        sample *= self.input_gain_lin;
        for filter in &mut self.filters {
            sample = filter.process_sample(sample);
        }
        sample = self.bass_filter.process_sample(sample);
        sample = self.voice_filter.process_sample(sample);
        sample = self.treble_filter.process_sample(sample);
        sample
    }

    /// Processes an audio slice in-place through input gain, 10 bands, and macro tones.
    #[inline]
    pub fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for sample in samples.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    /// Processes an audio slice in-place (alias for `process`).
    #[inline]
    pub fn process_slice(&mut self, buffer: &mut [f32]) {
        self.process(buffer);
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

        eq.process(&mut buffer);

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

        eq.process(&mut buffer);
        assert_eq!(buffer, original);
    }

    #[test]
    fn test_input_gain() {
        let mut eq = ParametricEq::new(48000.0);
        eq.set_input_gain(6.0); // +6 dB is ~2.0x linear
        assert!((eq.input_gain_lin() - 10.0_f32.powf(6.0 / 20.0)).abs() < 1e-4);

        let mut buffer = vec![0.25_f32; 64];
        eq.process(&mut buffer);
        let expected = 0.25 * 10.0_f32.powf(6.0 / 20.0);
        for &sample in &buffer {
            assert!((sample - expected).abs() < 1e-3);
        }
    }

    #[test]
    fn test_macro_tones() {
        let mut eq = ParametricEq::new(48000.0);
        eq.set_macro_tones(3.0, -2.0, 4.0);

        let mut buffer = vec![0.2_f32; 256];
        eq.process(&mut buffer);
        for &sample in &buffer {
            assert!(!sample.is_nan());
        }
    }
}
