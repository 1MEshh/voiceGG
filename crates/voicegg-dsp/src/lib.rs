#![forbid(unsafe_code)]

//! Digital Signal Processing algorithms for VoiceGG.
//!
//! Includes 10-band parametric biquad equalizers, peak limiters, noise gates,
//! compressors, meters, and AI-assisted noise cancellation.

pub mod biquad;
pub mod compressor;
pub mod eq;
pub mod gate;
pub mod limiter;
pub mod meter;

pub use biquad::{BiquadFilter, BiquadParams};
pub use compressor::Compressor;
pub use eq::ParametricEq;
pub use gate::NoiseGate;
pub use limiter::Limiter;
pub use meter::VUMeter;
