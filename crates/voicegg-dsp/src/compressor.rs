//! Dynamic range compressor implementation.

use voicegg_core::CompressorConfig;

/// Dynamic range compressor with feedforward peak detector and makeup gain.
#[derive(Debug, Clone)]
pub struct Compressor {
    sample_rate: f32,
    enabled: bool,
    threshold_db: f32,
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    makeup_linear: f32,
    knee_db: f32,
    envelope: f32,
}

impl Compressor {
    /// Creates a new compressor with the given sample rate and configuration.
    #[must_use]
    pub fn new(sample_rate: f32, config: &CompressorConfig) -> Self {
        let mut comp = Self {
            sample_rate,
            enabled: config.enabled,
            threshold_db: config.threshold_db,
            ratio: config.ratio,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            makeup_linear: 1.0,
            knee_db: config.knee_db,
            envelope: 0.0,
        };
        comp.update_config(config);
        comp
    }

    /// Updates dynamic compressor parameters.
    pub fn update_config(&mut self, config: &CompressorConfig) {
        self.enabled = config.enabled;
        self.threshold_db = config.threshold_db;
        self.ratio = config.ratio.max(1.0);
        self.attack_coeff = (-1.0 / (config.attack_ms.max(0.1) * 0.001 * self.sample_rate)).exp();
        self.release_coeff = (-1.0 / (config.release_ms.max(1.0) * 0.001 * self.sample_rate)).exp();
        self.makeup_linear = 10.0_f32.powf(config.makeup_gain_db / 20.0);
        self.knee_db = config.knee_db.max(0.0);
    }

    /// Resets the envelope detector.
    pub fn reset(&mut self) {
        self.envelope = 0.0;
    }

    /// Processes a single audio sample.
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }

        let abs_in = input.abs();
        let coeff = if abs_in > self.envelope {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.envelope = self.envelope * coeff + abs_in * (1.0 - coeff);

        let env_db = if self.envelope <= 1e-5 {
            -100.0
        } else {
            20.0 * self.envelope.log10()
        };

        let gain_db = if self.knee_db <= 0.01 {
            if env_db > self.threshold_db {
                (self.threshold_db - env_db) * (1.0 - 1.0 / self.ratio)
            } else {
                0.0
            }
        } else {
            let half_knee = self.knee_db / 2.0;
            if env_db <= self.threshold_db - half_knee {
                0.0
            } else if env_db >= self.threshold_db + half_knee {
                (self.threshold_db - env_db) * (1.0 - 1.0 / self.ratio)
            } else {
                let dx = env_db - self.threshold_db + half_knee;
                ((1.0 / self.ratio - 1.0) * dx * dx) / (2.0 * self.knee_db)
            }
        };

        let gain_linear = 10.0_f32.powf(gain_db / 20.0) * self.makeup_linear;
        input * gain_linear
    }

    /// Processes a slice of audio in-place.
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
    fn test_compressor_reduces_gain() {
        let cfg = CompressorConfig {
            enabled: true,
            threshold_db: -10.0,
            ratio: 4.0,
            attack_ms: 0.5,
            ..Default::default()
        };

        let mut comp = Compressor::new(48000.0, &cfg);

        // Feed full scale 1.0 (0 dBFS) signal
        let mut buffer = vec![1.0_f32; 1000];
        comp.process_slice(&mut buffer);

        let final_sample = buffer.last().copied().unwrap();
        // Since input is 0 dBFS and threshold is -10 dBFS with ratio 4:
        // Gain reduction = (10) * (1 - 1/4) = 7.5 dB reduction -> output should be around -7.5 dBFS (~0.42)
        assert!(final_sample < 0.6);
        assert!(final_sample > 0.3);
    }

    #[test]
    fn test_compressor_soft_knee_smoothness() {
        let hard_cfg = CompressorConfig {
            enabled: true,
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 0.1,
            knee_db: 0.0,
            ..Default::default()
        };
        let soft_cfg = CompressorConfig {
            enabled: true,
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 0.1,
            knee_db: 6.0, // knee spans -23 dBFS to -17 dBFS
            ..Default::default()
        };

        let mut hard_comp = Compressor::new(48000.0, &hard_cfg);
        let mut soft_comp = Compressor::new(48000.0, &soft_cfg);

        // Test at -21 dBFS (0.0891) - inside soft knee transition, below hard threshold
        let test_val = 0.0891_f32;
        let mut hard_buf = vec![test_val; 500];
        let mut soft_buf = vec![test_val; 500];

        hard_comp.process_slice(&mut hard_buf);
        soft_comp.process_slice(&mut soft_buf);

        let hard_out = hard_buf.last().copied().unwrap();
        let soft_out = soft_buf.last().copied().unwrap();

        // Hard knee must be untouched (no compression below -20 dBFS)
        assert!((hard_out - test_val).abs() < 1e-4);
        // Soft knee must introduce subtle gentle attenuation smoothly
        assert!(soft_out < hard_out);
        assert!(soft_out > 0.08);
    }
}
