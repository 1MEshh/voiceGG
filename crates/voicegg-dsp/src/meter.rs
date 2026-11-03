//! Audio volume metering (Peak and RMS in dBFS).

/// A fast, allocation-free level meter for calculating Peak and RMS values.
#[derive(Debug, Clone, Copy)]
pub struct VUMeter {
    peak: f32,
    sum_squares: f32,
    sample_count: usize,
}

impl Default for VUMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl VUMeter {
    /// Creates a new metering accumulator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            peak: 0.0,
            sum_squares: 0.0,
            sample_count: 0,
        }
    }

    /// Feeds an audio buffer into the meter using streamlined chunked arithmetic.
    pub fn process(&mut self, samples: &[f32]) {
        let mut local_peak = self.peak;
        let mut sum0 = 0.0_f32;
        let mut sum1 = 0.0_f32;
        let mut sum2 = 0.0_f32;
        let mut sum3 = 0.0_f32;

        let chunks = samples.chunks_exact(4);
        let remainder = chunks.remainder();

        for chunk in chunks {
            let s0 = if chunk[0].is_finite() { chunk[0] } else { 0.0 };
            let s1 = if chunk[1].is_finite() { chunk[1] } else { 0.0 };
            let s2 = if chunk[2].is_finite() { chunk[2] } else { 0.0 };
            let s3 = if chunk[3].is_finite() { chunk[3] } else { 0.0 };

            let a0 = s0.abs();
            let a1 = s1.abs();
            let a2 = s2.abs();
            let a3 = s3.abs();

            let p01 = if a0 > a1 { a0 } else { a1 };
            let p23 = if a2 > a3 { a2 } else { a3 };
            let p = if p01 > p23 { p01 } else { p23 };
            if p > local_peak {
                local_peak = p;
            }

            sum0 += s0 * s0;
            sum1 += s1 * s1;
            sum2 += s2 * s2;
            sum3 += s3 * s3;
        }

        for &s in remainder {
            let val = if s.is_finite() { s } else { 0.0 };
            let a = val.abs();
            if a > local_peak {
                local_peak = a;
            }
            sum0 += val * val;
        }

        self.peak = local_peak;
        self.sum_squares += (sum0 + sum1) + (sum2 + sum3);
        self.sample_count += samples.len();
    }

    /// Applies a decay coefficient to the current peak value (for peak decay buffers).
    #[inline]
    pub fn decay_peak(&mut self, factor: f32) {
        self.peak *= factor.clamp(0.0, 1.0);
    }

    /// Feeds an audio buffer and applies peak decay arithmetic.
    pub fn process_with_decay(&mut self, samples: &[f32], decay_factor: f32) {
        self.decay_peak(decay_factor);
        self.process(samples);
    }

    /// Computes the current peak in dBFS (-100.0 dBFS floor).
    #[must_use]
    pub fn peak_dbfs(&self) -> f32 {
        if self.peak <= 1e-5 || !self.peak.is_finite() {
            -100.0
        } else {
            20.0 * self.peak.log10()
        }
    }

    /// Computes the current RMS in dBFS (-100.0 dBFS floor).
    #[must_use]
    pub fn rms_dbfs(&self) -> f32 {
        if self.sample_count == 0 {
            return -100.0;
        }
        let mean_square = self.sum_squares / (self.sample_count as f32);
        if mean_square <= 1e-10 || !mean_square.is_finite() {
            return -100.0;
        }
        let rms = mean_square.sqrt();
        if rms <= 1e-5 || !rms.is_finite() {
            -100.0
        } else {
            20.0 * rms.log10()
        }
    }

    /// Resets the meter window.
    pub fn reset(&mut self) {
        self.peak = 0.0;
        self.sum_squares = 0.0;
        self.sample_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_meter() {
        let mut meter = VUMeter::new();
        let silence = vec![0.0_f32; 256];
        meter.process(&silence);
        assert_eq!(meter.peak_dbfs(), -100.0);
        assert_eq!(meter.rms_dbfs(), -100.0);
    }

    #[test]
    fn test_full_scale_sine() {
        let mut meter = VUMeter::new();
        let buffer = vec![1.0, -1.0, 1.0, -1.0];
        meter.process(&buffer);
        assert!((meter.peak_dbfs() - 0.0).abs() < 1e-3);
    }

    #[test]
    fn test_meter_peak_decay() {
        let mut meter = VUMeter::new();
        let buffer = vec![0.5, -0.5, 0.5];
        meter.process(&buffer);
        let peak_before = meter.peak_dbfs();
        meter.decay_peak(0.5);
        let peak_after = meter.peak_dbfs();
        assert!(peak_after < peak_before);
        // Halving linear amplitude corresponds to ~ -6.02 dB
        assert!((peak_before - peak_after - 6.02).abs() < 0.1);
    }

    #[test]
    fn test_nan_safety_in_meter() {
        let mut meter = VUMeter::new();
        let buffer = vec![f32::NAN, 0.5, f32::INFINITY];
        meter.process(&buffer);
        assert!(meter.peak_dbfs().is_finite());
        assert!(meter.rms_dbfs().is_finite());
    }
}
