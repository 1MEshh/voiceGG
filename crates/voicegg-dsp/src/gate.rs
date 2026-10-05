//! Noise gate processor with attack, hold, and release ballistics.

use voicegg_core::NoiseGateConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateState {
    Closed,
    Opening,
    Open,
    Holding,
    Releasing,
}

/// Noise gate processor to eliminate background microphone noise and hum when silent.
#[derive(Debug, Clone)]
pub struct NoiseGate {
    sample_rate: f32,
    enabled: bool,
    threshold_linear: f32,
    attack_coeff: f32,
    release_coeff: f32,
    hold_samples: usize,
    hold_counter: usize,
    state: GateState,
    current_gain: f32,
}

impl NoiseGate {
    /// Creates a new noise gate processor.
    #[must_use]
    pub fn new(sample_rate: f32, config: &NoiseGateConfig) -> Self {
        let mut gate = Self {
            sample_rate,
            enabled: config.enabled,
            threshold_linear: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            hold_samples: 0,
            hold_counter: 0,
            state: GateState::Closed,
            current_gain: 0.0,
        };
        gate.update_config(config);
        gate
    }

    /// Updates noise gate parameters.
    pub fn update_config(&mut self, config: &NoiseGateConfig) {
        self.enabled = config.enabled;
        self.threshold_linear = 10.0_f32.powf(config.threshold_db / 20.0);
        self.attack_coeff = (-1.0 / (config.attack_ms.max(0.1) * 0.001 * self.sample_rate)).exp();
        self.release_coeff = (-1.0 / (config.release_ms.max(1.0) * 0.001 * self.sample_rate)).exp();
        self.hold_samples = (config.hold_ms * 0.001 * self.sample_rate) as usize;
    }

    /// Resets gate state.
    pub fn reset(&mut self) {
        self.state = GateState::Closed;
        self.current_gain = 0.0;
        self.hold_counter = 0;
    }

    /// Processes a single audio sample.
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }

        let abs_in = input.abs();

        match self.state {
            GateState::Closed => {
                if abs_in > self.threshold_linear {
                    self.state = GateState::Opening;
                }
            }
            GateState::Opening => {
                self.current_gain =
                    self.current_gain * self.attack_coeff + (1.0 - self.attack_coeff);
                if self.current_gain >= 0.99 {
                    self.current_gain = 1.0;
                    self.state = GateState::Open;
                }
            }
            GateState::Open => {
                if abs_in < self.threshold_linear {
                    self.state = GateState::Holding;
                    self.hold_counter = 0;
                }
            }
            GateState::Holding => {
                if abs_in > self.threshold_linear {
                    self.state = GateState::Open;
                } else {
                    self.hold_counter += 1;
                    if self.hold_counter >= self.hold_samples {
                        self.state = GateState::Releasing;
                    }
                }
            }
            GateState::Releasing => {
                if abs_in > self.threshold_linear {
                    self.state = GateState::Opening;
                } else {
                    self.current_gain *= self.release_coeff;
                    if self.current_gain <= 0.001 {
                        self.current_gain = 0.0;
                        self.state = GateState::Closed;
                    }
                }
            }
        }

        input * self.current_gain
    }

    /// Processes an in-place audio slice.
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
    fn test_gate_mutes_low_level_noise() {
        let cfg = NoiseGateConfig {
            enabled: true,
            threshold_db: -30.0, // ~0.0316
            attack_ms: 1.0,
            release_ms: 5.0,
            ..Default::default()
        };

        let mut gate = NoiseGate::new(48000.0, &cfg);

        // Feed quiet noise below threshold (-40 dBFS, ~0.01)
        let mut buffer = vec![0.005_f32; 100];
        gate.process_slice(&mut buffer);

        // Output should be completely muted
        for sample in buffer {
            assert_eq!(sample, 0.0);
        }
    }
}
