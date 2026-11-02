//! Biquad IIR filter implementation using Robert Bristow-Johnson's Audio EQ Cookbook.

use voicegg_core::FilterType;

/// Coefficients for a Direct Form II Transposed biquad filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadParams {
    /// Feedforward coefficient b0
    pub b0: f32,
    /// Feedforward coefficient b1
    pub b1: f32,
    /// Feedforward coefficient b2
    pub b2: f32,
    /// Feedback coefficient a1
    pub a1: f32,
    /// Feedback coefficient a2
    pub a2: f32,
}

impl Default for BiquadParams {
    fn default() -> Self {
        Self::identity()
    }
}

impl BiquadParams {
    /// Identity passthrough filter (b0 = 1, all others 0).
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Computes filter coefficients given filter type, sample rate, cutoff frequency, gain in dB, and Q factor.
    #[must_use]
    pub fn compute(
        filter_type: FilterType,
        sample_rate: f32,
        freq_hz: f32,
        gain_db: f32,
        q: f32,
    ) -> Self {
        use std::f32::consts::PI;

        let nyquist = sample_rate * 0.49;
        let clamped_freq = freq_hz.clamp(20.0, nyquist);
        let omega = 2.0 * PI * clamped_freq / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q.max(0.01));
        let a = 10.0_f32.powf(gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::Peak => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_omega;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_omega + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
                let a2 = (a + 1.0) + (a - 1.0) * cos_omega - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_omega + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
                let a2 = (a + 1.0) - (a - 1.0) * cos_omega - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_omega) * 0.5;
                let b1 = 1.0 - cos_omega;
                let b2 = (1.0 - cos_omega) * 0.5;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_omega) * 0.5;
                let b1 = -(1.0 + cos_omega);
                let b2 = (1.0 + cos_omega) * 0.5;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_omega;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        if !a0.is_finite() || a0.abs() < f32::EPSILON {
            return Self::identity();
        }

        let inv_a0 = 1.0 / a0;
        let b0 = b0 * inv_a0;
        let b1 = b1 * inv_a0;
        let b2 = b2 * inv_a0;
        let a1 = a1 * inv_a0;
        let a2 = a2 * inv_a0;

        if !b0.is_finite() || !b1.is_finite() || !b2.is_finite() || !a1.is_finite() || !a2.is_finite() {
            return Self::identity();
        }

        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
        }
    }
}

/// A biquad filter instance with internal state memory and parameter smoothing.
#[derive(Debug, Clone, Copy)]
pub struct BiquadFilter {
    params: BiquadParams,
    target_params: BiquadParams,
    ramp_remaining: usize,
    // Delay registers for Direct Form II Transposed
    s1: f32,
    s2: f32,
}

impl Default for BiquadFilter {
    fn default() -> Self {
        Self::new(BiquadParams::identity())
    }
}

impl BiquadFilter {
    /// Creates a new biquad filter with the given coefficients.
    #[must_use]
    pub const fn new(params: BiquadParams) -> Self {
        Self {
            params,
            target_params: params,
            ramp_remaining: 0,
            s1: 0.0,
            s2: 0.0,
        }
    }

    /// Returns the target filter coefficients.
    #[must_use]
    pub fn target_params(&self) -> BiquadParams {
        self.target_params
    }

    /// Returns the current active filter coefficients.
    #[must_use]
    pub fn params(&self) -> BiquadParams {
        self.params
    }

    /// Updates the filter coefficients with a smooth crossfade ramp to prevent audio clicks.
    pub fn set_params(&mut self, params: BiquadParams) {
        if self.params == params && self.ramp_remaining == 0 {
            return;
        }
        self.target_params = params;
        self.ramp_remaining = 128; // Smooth ramp over ~2.6 ms @ 48 kHz
    }

    /// Resets the filter's internal history state to zero and finishes any active ramp.
    pub fn reset(&mut self) {
        self.params = self.target_params;
        self.ramp_remaining = 0;
        self.s1 = 0.0;
        self.s2 = 0.0;
    }

    /// Processes a single audio sample (Direct Form II Transposed).
    #[inline(always)]
    pub fn process_sample(&mut self, mut input: f32) -> f32 {
        if !input.is_finite() {
            input = 0.0;
        }

        if self.ramp_remaining > 0 {
            let step = 1.0 / (self.ramp_remaining as f32);
            self.params.b0 += (self.target_params.b0 - self.params.b0) * step;
            self.params.b1 += (self.target_params.b1 - self.params.b1) * step;
            self.params.b2 += (self.target_params.b2 - self.params.b2) * step;
            self.params.a1 += (self.target_params.a1 - self.params.a1) * step;
            self.params.a2 += (self.target_params.a2 - self.params.a2) * step;
            self.ramp_remaining -= 1;
            if self.ramp_remaining == 0 {
                self.params = self.target_params;
            }
        }
        let mut output = self.params.b0 * input + self.s1;
        self.s1 = self.params.b1 * input - self.params.a1 * output + self.s2;
        self.s2 = self.params.b2 * input - self.params.a2 * output;

        // Flush denormals in delay registers to prevent CPU performance penalties
        if self.s1.abs() < 1e-25 {
            self.s1 = 0.0;
        }
        if self.s2.abs() < 1e-25 {
            self.s2 = 0.0;
        }

        if !output.is_finite() {
            output = 0.0;
            self.s1 = 0.0;
            self.s2 = 0.0;
        }

        output
    }

    /// Processes an in-place slice of audio samples.
    #[inline]
    pub fn process_slice(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_denormal_and_nan_safety() {
        let mut filter = BiquadFilter::new(BiquadParams::compute(
            FilterType::Peak,
            48000.0,
            1000.0,
            6.0,
            1.0,
        ));

        // Process NaN and Inf
        assert_eq!(filter.process_sample(f32::NAN), 0.0);
        assert_eq!(filter.process_sample(f32::INFINITY), 0.0);
        assert_eq!(filter.process_sample(f32::NEG_INFINITY), 0.0);

        // Process denormal / tiny numbers
        let tiny = 1e-38_f32;
        let out = filter.process_sample(tiny);
        assert!(out.is_finite());
        assert!(!out.is_nan());
    }

    #[test]
    fn test_biquad_compute_zero_division_guard() {
        // Extreme values should not panic or produce NaN coefficients
        let params = BiquadParams::compute(FilterType::Peak, 48000.0, 0.0, 0.0, 0.0);
        assert!(params.b0.is_finite());
        assert!(params.a1.is_finite());
    }
}
