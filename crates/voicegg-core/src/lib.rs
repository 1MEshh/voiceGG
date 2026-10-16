#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Core types, configuration, and data structures for VoiceGG.

pub mod channel;
pub mod config;
pub mod dsp_types;
pub mod error;
pub mod preset;
pub mod theme;

pub use channel::ChannelId;
pub use config::{calculate_chatmix_volume, AppRouteRule, VoiceggConfig};
pub use dsp_types::{
    BiquadCoefficients, CompressorConfig, EqBand, EqConfig, FilterType, LimiterConfig,
    NoiseCancellerConfig, NoiseGateConfig, PluginCapabilities, SmartVolumeConfig,
    SpatialConfig,
};
pub use error::{CoreError, Result};
pub use preset::{Preset, PresetCategory};
pub use theme::{ChannelTheme, ThemeConfig};
