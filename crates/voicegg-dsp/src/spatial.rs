//! 3D Spatial Audio virtualization processor (Haas ITD crossfeed + Mid/Side soundstage widening).

/// Stereo 3D spatial soundstage processor.
#[derive(Debug, Clone)]
pub struct SpatialProcessor {
    sample_rate: f32,
    enabled: bool,
    distance: f32,
    immersion: f32,
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    write_idx: usize,
    lp_state_l: f32,
    lp_state_r: f32,
}

impl SpatialProcessor {
    /// Creates a new spatial processor for the given sample rate, virtual speaker distance (0.0..1.0),
    /// and immersion factor (0.0..1.0).
    #[must_use]
    pub fn new(sample_rate: f32, distance: f32, immersion: f32) -> Self {
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        // Max ITD delay ~0.6ms (~29 samples at 48kHz)
        let max_delay = ((sr * 0.001).ceil() as usize).max(32);
        Self {
            sample_rate: sr,
            enabled: true,
            distance: distance.clamp(0.0, 1.0),
            immersion: immersion.clamp(0.0, 1.0),
            delay_l: vec![0.0; max_delay],
            delay_r: vec![0.0; max_delay],
            write_idx: 0,
            lp_state_l: 0.0,
            lp_state_r: 0.0,
        }
    }

    /// Updates spatial distance (0.0..1.0) and immersion (0.0..1.0) parameters.
    pub fn set_params(&mut self, distance: f32, immersion: f32) {
        self.distance = distance.clamp(0.0, 1.0);
        self.immersion = immersion.clamp(0.0, 1.0);
    }

    /// Enables or disables spatial processing.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns whether spatial processing is active.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Clears internal delay lines and filter state.
    pub fn reset(&mut self) {
        self.delay_l.fill(0.0);
        self.delay_r.fill(0.0);
        self.write_idx = 0;
        self.lp_state_l = 0.0;
        self.lp_state_r = 0.0;
    }

    /// Processes stereo audio buffers in-place to expand directional soundstage cues.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        if !self.enabled || self.immersion <= 0.001 {
            return;
        }
        let len = left.len().min(right.len());
        let buf_len = self.delay_l.len();
        if buf_len == 0 || len == 0 {
            return;
        }

        // Interaural time delay (ITD) in samples (~0.25ms to 0.55ms)
        let delay_samples = (((0.00025 + 0.0003 * self.distance) * self.sample_rate) as usize)
            .clamp(1, buf_len - 1);

        // Side widening factor (up to 1.65x at 100% immersion)
        let width = 1.0 + 0.65 * self.immersion;
        // Subtle out-of-phase acoustic crossfeed for pin-point localization
        let crossfeed = 0.18 * self.immersion;
        // One-pole low-pass coefficient (~1.8kHz head-shadowing filter)
        let lp_coeff = (-2.0 * std::f32::consts::PI * 1800.0 / self.sample_rate).exp();

        for i in 0..len {
            let l_in = left[i];
            let r_in = right[i];

            self.delay_l[self.write_idx] = l_in;
            self.delay_r[self.write_idx] = r_in;

            let read_idx = (self.write_idx + buf_len - delay_samples) % buf_len;
            let delayed_l = self.delay_l[read_idx];
            let delayed_r = self.delay_r[read_idx];

            self.lp_state_l = self.lp_state_l * lp_coeff + delayed_l * (1.0 - lp_coeff);
            self.lp_state_r = self.lp_state_r * lp_coeff + delayed_r * (1.0 - lp_coeff);

            // Mid/Side decomposition with soundstage widening
            let mid = 0.5 * (l_in + r_in);
            let side = 0.5 * (l_in - r_in) * width;

            // Combine widened M/S with head-shadowed ITD cues, normalized to prevent clipping
            let norm = 1.0 / (1.0 + 0.15 * self.immersion);
            left[i] = ((mid + side) - crossfeed * self.lp_state_r) * norm;
            right[i] = ((mid - side) - crossfeed * self.lp_state_l) * norm;

            self.write_idx = (self.write_idx + 1) % buf_len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_processor_widens_stereo() {
        let mut sp = SpatialProcessor::new(48000.0, 0.5, 0.8);
        let mut left = vec![0.5_f32; 128];
        let mut right = vec![-0.2_f32; 128];
        let orig_diff = (left[64] - right[64]).abs();

        sp.process_stereo(&mut left, &mut right);

        let new_diff = (left[64] - right[64]).abs();
        assert!(new_diff > orig_diff, "Expected wider stereo separation");
    }
}
