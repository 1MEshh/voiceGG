//! PipeWire audio graph integration for VoiceGG.
//!
//! Manages virtual playback sinks, virtual capture sources, device discovery,
//! and per-application routing metadata.

pub mod device;
pub mod error;
pub mod graph;

pub use device::{AudioDevice, DeviceType};
pub use error::{PwError, Result};
pub use graph::GraphManager;
