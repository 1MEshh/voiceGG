#![forbid(unsafe_code)]

//! Digital Signal Processing algorithms for VoiceGG.
//!
//! Includes parametric biquad equalizers, peak limiters, noise gates,
//! compressors, meters, and AI-assisted noise cancellation.

pub mod biquad;
pub mod meter;

pub use biquad::{BiquadFilter, BiquadParams};
pub use meter::VUMeter;
