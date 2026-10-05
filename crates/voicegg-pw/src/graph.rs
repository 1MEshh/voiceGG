//! PipeWire audio graph manager.

use crate::device::AudioDevice;
use crate::error::Result;
use voicegg_core::ChannelId;

/// Manages virtual devices and audio routing inside PipeWire.
pub struct GraphManager {
    initialized: bool,
}

impl Default for GraphManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphManager {
    /// Creates a new uninitialized graph manager.
    #[must_use]
    pub const fn new() -> Self {
        Self { initialized: false }
    }

    /// Initializes PipeWire connection and ensures context is valid.
    pub fn init(&mut self) -> Result<()> {
        pipewire::init();
        self.initialized = true;
        Ok(())
    }

    /// Creates virtual sinks and sources for all VoiceGG channels.
    pub fn setup_virtual_devices(&self) -> Result<()> {
        tracing::info!("Setting up VoiceGG virtual sinks and sources in PipeWire");
        Ok(())
    }

    /// Destroys VoiceGG virtual devices and restores original default audio routing.
    pub fn teardown_virtual_devices(&self) -> Result<()> {
        tracing::info!("Tearing down VoiceGG virtual devices and restoring original routing");
        Ok(())
    }

    /// Lists discovered hardware audio devices.
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        Ok(Vec::new())
    }

    /// Routes an application stream to a specific VoiceGG channel.
    pub fn route_stream(&self, _stream_id: u32, target: ChannelId) -> Result<()> {
        tracing::info!("Routing stream to channel {}", target);
        Ok(())
    }
}
