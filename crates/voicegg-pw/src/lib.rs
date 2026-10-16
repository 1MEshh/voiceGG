//! PipeWire audio graph integration for VoiceGG.
//!
//! Manages virtual playback sinks, virtual capture sources, device discovery,
//! and per-application routing metadata.

pub mod chain;
pub mod device;
pub mod dsp_engine;
pub mod error;
pub mod graph;

pub use chain::{
    detect_capabilities, generate_channel_chain_conf, generate_mic_chain_conf, hot_update_eq_band,
    FilterChainRunner,
};
pub use device::{ActiveStream, AudioDevice, DeviceType};
pub use dsp_engine::{ChannelMeter, DspChannelWorker, DspEngine};
pub use error::{PwError, Result};
pub use graph::GraphManager;
