//! AI-assisted Noise Suppressor based on RNNoise.
//!
//! Processes 480-sample frames (10 ms at 48kHz) with variable block support,
//! wet/dry mixing (0 - 100%), and Voice Activity Detection (VAD).

use nnnoiseless::DenoiseState;

/// Number of samples required per RNNoise frame (10 ms @ 48 kHz).
pub const FRAME_SIZE: usize = DenoiseState::FRAME_SIZE;
const SCALE: f32 = 32767.0;
const INV_SCALE: f32 = 1.0 / 32767.0;

/// AI Noise Suppressor with wet/dry crossfade and sample-accurate buffering.
pub struct NoiseSuppressor {
    state: Box<DenoiseState<'static>>,
    enabled: bool,
    wet: f32, // 0.0 (bypass) to 1.0 (full reduction)
    input_buffer: Vec<f32>,
    output_fifo: Vec<f32>,
    dry_fifo: Vec<f32>,
    frame_in: [f32; FRAME_SIZE],
    frame_out: [f32; FRAME_SIZE],
    vad_probability: f32,
}

impl Default for NoiseSuppressor {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseSuppressor {
    /// Creates a new noise suppressor initialized in bypass mode.
    #[must_use]
    pub fn new() -> Self {
        let mut suppressor = Self {
            state: DenoiseState::new(),
            enabled: false,
            wet: 0.0,
            input_buffer: Vec::with_capacity(FRAME_SIZE * 4),
            output_fifo: Vec::with_capacity(FRAME_SIZE * 4),
            dry_fifo: Vec::with_capacity(FRAME_SIZE * 4),
            frame_in: [0.0; FRAME_SIZE],
            frame_out: [0.0; FRAME_SIZE],
            vad_probability: 0.0,
        };
        suppressor.reset();
        suppressor
    }

    /// Resets internal buffers and primes delay line with silence.
    pub fn reset(&mut self) {
        self.state = DenoiseState::new();
        self.input_buffer.clear();
        self.output_fifo.clear();
        self.dry_fifo.clear();
        self.output_fifo.resize(FRAME_SIZE, 0.0);
        self.dry_fifo.resize(FRAME_SIZE, 0.0);
        self.vad_probability = 0.0;
    }

    /// Enables or disables the noise suppressor.
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            if !enabled {
                self.reset();
            }
        }
    }

    /// Sets the noise reduction amount as a percentage (0 = bypass, 100 = full suppression).
    pub fn set_reduction_percent(&mut self, percent: u8) {
        let pct = percent.min(100);
        self.wet = pct as f32 / 100.0;
        self.set_enabled(pct > 0);
    }

    /// Returns the current Voice Activity Detection probability (0.0 to 1.0).
    #[must_use]
    pub fn vad_probability(&self) -> f32 {
        self.vad_probability
    }

    /// Processes a buffer of mono audio samples in-place.
    pub fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled || self.wet <= 0.001 || samples.is_empty() {
            // Immediate zero-overhead passthrough
            return;
        }

        let n = samples.len();

        // Feed dry FIFO and input accumulation buffer
        self.dry_fifo.extend_from_slice(samples);
        self.input_buffer.extend_from_slice(samples);

        // Process all full 480-sample frames
        while self.input_buffer.len() >= FRAME_SIZE {
            for (dest, &src) in self
                .frame_in
                .iter_mut()
                .zip(&self.input_buffer[..FRAME_SIZE])
            {
                *dest = (src * SCALE).clamp(-32768.0, 32767.0);
            }

            let vad = self
                .state
                .process_frame(&mut self.frame_out, &self.frame_in);
            self.vad_probability = vad;

            for &sample in &self.frame_out {
                self.output_fifo.push((sample * INV_SCALE).clamp(-1.0, 1.0));
            }

            self.input_buffer.drain(..FRAME_SIZE);
        }

        // Crossfade wet and dry signals into the destination buffer
        if self.output_fifo.len() >= n && self.dry_fifo.len() >= n {
            for (i, out_sample) in samples.iter_mut().enumerate() {
                let dry = self.dry_fifo[i];
                let wet_sample = self.output_fifo[i];
                *out_sample = ((1.0 - self.wet) * dry + self.wet * wet_sample).clamp(-1.0, 1.0);
            }
            self.output_fifo.drain(..n);
            self.dry_fifo.drain(..n);
        } else {
            // Fallback safety if buffer underrun occurs
            tracing::warn!("NoiseSuppressor buffer underrun; falling back to dry passthrough");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_when_disabled() {
        let mut suppressor = NoiseSuppressor::new();
        suppressor.set_enabled(false);

        let mut samples = vec![0.5; 256];
        suppressor.process(&mut samples);

        for &s in &samples {
            assert_eq!(s, 0.5);
        }
    }

    #[test]
    fn test_silence_input_remains_silent() {
        let mut suppressor = NoiseSuppressor::new();
        suppressor.set_reduction_percent(100);

        // Feed several blocks of silence (48kHz quantum 256)
        for _ in 0..10 {
            let mut samples = vec![0.0; 256];
            suppressor.process(&mut samples);
            for &s in &samples {
                assert!(s.abs() < 1e-4, "Expected silence, got {s}");
            }
        }
    }

    #[test]
    fn test_wet_dry_mix() {
        let mut suppressor = NoiseSuppressor::new();
        suppressor.set_reduction_percent(50); // 50% wet, 50% dry

        let mut samples = vec![0.1; 512];
        suppressor.process(&mut samples);
        // Processed without panicking or producing NaNs
        for &s in &samples {
            assert!(!s.is_nan());
        }
    }
}
