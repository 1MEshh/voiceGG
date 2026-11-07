//! Fast true-peak audio limiter with hard safety ceiling.

/// Peak limiter with configurable ceiling, fast attack, and exponential decay.
#[derive(Debug, Clone)]
pub struct Limiter {
    ceiling_linear: f32,
    release_coeff: f32,
    envelope: f32,
    sample_rate: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        // Safe default: -0.5 dBFS ceiling at 48000 Hz sample rate
        Self::new(48000.0, -0.5, 50.0)
    }
}

impl Limiter {
    /// Creates a new limiter with the given sample rate, ceiling in dBFS, and release time in ms.
    #[must_use]
    pub fn new(sample_rate: f32, ceiling_db: f32, release_ms: f32) -> Self {
        // Enforce hard ceiling capped at -0.1 dBFS maximum for safety
        let clamped_ceiling_db = ceiling_db.min(-0.1);
        let ceiling_linear = 10.0_f32.powf(clamped_ceiling_db / 20.0);
        let release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();

        Self {
            ceiling_linear,
            release_coeff,
            envelope: 0.0,
            sample_rate,
        }
    }

    /// Updates the sample rate and recomputes the release coefficient.
    pub fn set_sample_rate(&mut self, sample_rate: f32, release_ms: f32) {
        self.sample_rate = sample_rate;
        self.release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();
    }

    /// Resets the internal envelope history.
    pub fn reset(&mut self) {
        self.envelope = 0.0;
    }

    /// Processes a single sample in-place through the limiter.
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let abs_in = input.abs();
        if abs_in > self.envelope {
            self.envelope = abs_in;
        } else {
            self.envelope =
                self.envelope * self.release_coeff + abs_in * (1.0 - self.release_coeff);
        }

        let gain = if self.envelope > self.ceiling_linear {
            self.ceiling_linear / self.envelope
        } else {
            1.0
        };

        // Clamp to absolute ceiling as a failsafe
        (input * gain).clamp(-self.ceiling_linear, self.ceiling_linear)
    }

    /// Processes an audio buffer in-place.
    #[inline]
    pub fn process_slice(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    /// Processes stereo audio in-place using a linked envelope detector.
    ///
    /// Computes `max(|L[i]|, |R[i]|)` per sample and applies identical gain reduction
    /// to both channels, preventing stereo image collapse and eliminating sequential-buffer modulation.
    #[inline]
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        for i in 0..n {
            let l_abs = left[i].abs();
            let r_abs = right[i].abs();
            let abs_in = l_abs.max(r_abs);

            if abs_in > self.envelope {
                self.envelope = abs_in;
            } else {
                self.envelope =
                    self.envelope * self.release_coeff + abs_in * (1.0 - self.release_coeff);
            }

            let gain = if self.envelope > self.ceiling_linear {
                self.ceiling_linear / self.envelope
            } else {
                1.0
            };

            left[i] = (left[i] * gain).clamp(-self.ceiling_linear, self.ceiling_linear);
            right[i] = (right[i] * gain).clamp(-self.ceiling_linear, self.ceiling_linear);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_stereo_linking() {
        let mut limiter = Limiter::new(48000.0, -6.0, 50.0);
        let ceiling = 10.0_f32.powf(-6.0 / 20.0);

        // L is loud (10.0 = +20 dB), R is quiet (0.1)
        let mut left = vec![10.0_f32; 100];
        let mut right = vec![0.1_f32; 100];

        limiter.process_stereo(&mut left, &mut right);

        for sample in &left {
            assert!(sample.abs() <= ceiling + 1e-4);
        }
        // Gain reduction applied to left is (ceiling / 10.0)
        // Right channel must be scaled by the EXACT same factor!
        let expected_gain = ceiling / 10.0;
        let last_r = right.last().copied().unwrap();
        assert!(
            (last_r - 0.1 * expected_gain).abs() < 1e-4,
            "Stereo-linked limiter must attenuate both channels equally: expected {}, got {}",
            0.1 * expected_gain,
            last_r
        );
    }

    #[test]
    fn test_limiter_clamps_loud_signal() {
        let mut limiter = Limiter::new(48000.0, -6.0, 50.0);
        let ceiling = 10.0_f32.powf(-6.0 / 20.0);

        // Feed extremely loud 10.0 (+20 dB) signal
        let mut buffer = vec![10.0_f32; 100];
        limiter.process_slice(&mut buffer);

        for sample in buffer {
            assert!(sample.abs() <= ceiling + 1e-4);
        }
    }

    #[test]
    fn test_limiter_transparent_on_quiet_signal() {
        let mut limiter = Limiter::new(48000.0, -6.0, 50.0);
        let mut buffer = vec![0.1_f32; 100];
        let original = buffer.clone();

        limiter.process_slice(&mut buffer);

        for (a, b) in buffer.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-4);
        }
    }
}
