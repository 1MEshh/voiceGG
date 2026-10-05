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

    /// Feeds an audio buffer into the meter.
    pub fn process(&mut self, samples: &[f32]) {
        for &s in samples {
            let abs = s.abs();
            if abs > self.peak {
                self.peak = abs;
            }
            self.sum_squares += s * s;
        }
        self.sample_count += samples.len();
    }

    /// Computes the current peak in dBFS (-100.0 dBFS floor).
    #[must_use]
    pub fn peak_dbfs(&self) -> f32 {
        if self.peak <= 1e-5 {
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
        let rms = mean_square.sqrt();
        if rms <= 1e-5 {
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
        // 1.0 peak
        let buffer = vec![1.0, -1.0, 1.0, -1.0];
        meter.process(&buffer);
        assert!((meter.peak_dbfs() - 0.0).abs() < 1e-3);
    }
}
