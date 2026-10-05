//! PipeWire and PulseAudio audio graph manager.

use crate::device::{ActiveStream, AudioDevice, DeviceType};
use crate::error::{PwError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use voicegg_core::ChannelId;

#[derive(Debug, Default, Serialize, Deserialize)]
struct GraphState {
    original_default_sink: Option<String>,
    original_default_source: Option<String>,
    loaded_modules: Vec<u32>,
}

/// Directed graph cycle detector to prevent audio feedback loops.
#[derive(Debug, Default, Clone)]
pub struct GraphCycleDetector {
    edges: std::collections::HashMap<String, Vec<String>>,
}

impl GraphCycleDetector {
    /// Creates a new cycle detector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if adding a directed edge `from -> to` would create a cycle.
    #[must_use]
    pub fn would_cycle(&self, from: &str, to: &str) -> bool {
        if from == to {
            return true;
        }
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(to.to_string());

        while let Some(current) = queue.pop_front() {
            if current == from {
                return true;
            }
            if visited.insert(current.clone()) {
                if let Some(neighbors) = self.edges.get(&current) {
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }
        false
    }

    /// Records a directed edge.
    pub fn add_edge(&mut self, from: String, to: String) {
        self.edges.entry(from).or_default().push(to);
    }

    /// Clears all recorded edges.
    pub fn clear(&mut self) {
        self.edges.clear();
    }
}

/// Manages virtual devices, hardware discovery, and audio routing inside PipeWire.
pub struct GraphManager {
    initialized: bool,
    state: GraphState,
    cycle_detector: GraphCycleDetector,
}

impl Default for GraphManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphManager {
    /// Creates a new uninitialized graph manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: GraphState::default(),
            cycle_detector: GraphCycleDetector::new(),
        }
    }

    fn state_file_path() -> PathBuf {
        let base = dirs::runtime_dir()
            .unwrap_or_else(|| dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp")));
        base.join("voicegg").join("modules.json")
    }

    fn save_state(&self) {
        let path = Self::state_file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.state) {
            let _ = fs::write(path, json);
        }
    }

    /// Removes any stale loaded VoiceGG modules from PipeWire and restores defaults.
    pub fn cleanup_stale_modules() {
        let path = Self::state_file_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(old_state) = serde_json::from_str::<GraphState>(&content) {
                    tracing::info!(
                        "Found stale state with {} modules; cleaning up...",
                        old_state.loaded_modules.len()
                    );
                    for &id in old_state.loaded_modules.iter().rev() {
                        let _ = Command::new("pactl")
                            .args(["unload-module", &id.to_string()])
                            .output();
                    }
                    if let Some(sink) = old_state.original_default_sink {
                        let _ = Command::new("pactl")
                            .args(["set-default-sink", &sink])
                            .output();
                    }
                    if let Some(source) = old_state.original_default_source {
                        let _ = Command::new("pactl")
                            .args(["set-default-source", &source])
                            .output();
                    }
                }
            }
            let _ = fs::remove_file(&path);
        }
    }

    /// Initializes PipeWire connection and cleans up any old dangling modules.
    pub fn init(&mut self) -> Result<()> {
        pipewire::init();
        Self::cleanup_stale_modules();
        self.initialized = true;
        Ok(())
    }

    fn load_null_sink(&mut self, name: &str, desc: &str, is_source: bool) -> Result<u32> {
        let prop_arg = if is_source {
            format!("sink_properties=media.class=Audio/Source/Virtual device.description={desc}")
        } else {
            format!("sink_properties=device.description={desc}")
        };
        let name_arg = format!("sink_name={name}");

        let output = Command::new("pactl")
            .args(["load-module", "module-null-sink", &name_arg, &prop_arg])
            .output()
            .map_err(|e| PwError::Generic(format!("Failed to execute pactl load-module: {e}")))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(PwError::Generic(format!(
                "Failed to create virtual device '{name}': {err}"
            )));
        }

        let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let module_id: u32 = out_str.parse().map_err(|_| {
            PwError::Generic(format!("pactl did not return a valid module ID: {out_str}"))
        })?;

        self.state.loaded_modules.push(module_id);
        self.save_state();
        Ok(module_id)
    }

    fn load_loopback(&mut self, source: &str, sink: &str) -> Result<u32> {
        // SAFETY GUARD: Check for cycles before creating loopback
        if self.cycle_detector.would_cycle(source, sink) {
            return Err(PwError::Generic(format!(
                "Feedback loop prevented: linking '{source}' to '{sink}' would create an audio feedback loop"
            )));
        }

        let src_arg = format!("source={source}");
        let sink_arg = format!("sink={sink}");

        let output = Command::new("pactl")
            .args([
                "load-module",
                "module-loopback",
                &src_arg,
                &sink_arg,
                "latency_msec=5",
            ])
            .output()
            .map_err(|e| PwError::Generic(format!("Failed to execute pactl load-module: {e}")))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(PwError::Generic(format!(
                "Failed to link '{source}' to '{sink}': {err}"
            )));
        }

        let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let module_id: u32 = out_str.parse().map_err(|_| {
            PwError::Generic(format!("pactl did not return a valid module ID: {out_str}"))
        })?;

        self.cycle_detector
            .add_edge(source.to_string(), sink.to_string());
        self.state.loaded_modules.push(module_id);
        self.save_state();
        Ok(module_id)
    }

    /// Sets up all VoiceGG virtual sinks, sources, and internal loopback links.
    pub fn setup_virtual_devices(&mut self) -> Result<()> {
        tracing::info!("Creating VoiceGG virtual channels in PipeWire...");

        // Save original default devices
        if let Ok(out) = Command::new("pactl").arg("get-default-sink").output() {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !name.is_empty() && !name.starts_with("voicegg_") {
                    self.state.original_default_sink = Some(name);
                }
            }
        }

        if let Ok(out) = Command::new("pactl").arg("get-default-source").output() {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !name.is_empty() && !name.starts_with("voicegg_") {
                    self.state.original_default_source = Some(name);
                }
            }
        }

        // 1. Create Master Sink
        self.load_null_sink("voicegg_sink_master", "VoiceGG_Master", false)?;

        // 2. Create Channel Sinks
        self.load_null_sink("voicegg_sink_game", "VoiceGG_Game", false)?;
        self.load_null_sink("voicegg_sink_chat", "VoiceGG_Chat", false)?;
        self.load_null_sink("voicegg_sink_media", "VoiceGG_Media", false)?;
        self.load_null_sink("voicegg_sink_aux", "VoiceGG_Aux", false)?;

        // 3. Create Sources (Microphone and Stream Mix)
        self.load_null_sink("voicegg_source_mic", "VoiceGG_Microphone", true)?;
        self.load_null_sink("voicegg_source_stream_mix", "VoiceGG_Stream_Mix", true)?;

        // 4. Link Channel Sinks to Master
        self.load_loopback("voicegg_sink_game.monitor", "voicegg_sink_master")?;
        self.load_loopback("voicegg_sink_chat.monitor", "voicegg_sink_master")?;
        self.load_loopback("voicegg_sink_media.monitor", "voicegg_sink_master")?;
        self.load_loopback("voicegg_sink_aux.monitor", "voicegg_sink_master")?;

        // 5. Link Master Sink to Physical Output Device
        if let Some(target_sink) = self.state.original_default_sink.clone() {
            tracing::info!(
                "Connecting VoiceGG Master to physical sink '{}'",
                target_sink
            );
            let _ = self.load_loopback("voicegg_sink_master.monitor", &target_sink);
        }

        // 6. Link Physical Mic to VoiceGG Mic Source.
        // SAFETY RULE (PLAN.md §0.3): never use a monitor as a mic — that creates a feedback loop.
        if let Some(mic_source) = self.state.original_default_source.clone() {
            if mic_source.ends_with(".monitor")
                || mic_source.starts_with("alsa_output")
                || mic_source.starts_with("voicegg_")
            {
                tracing::warn!("No real microphone found (default source is a monitor/output); Mic channel left empty");
            } else {
                tracing::info!(
                    "Connecting physical microphone '{}' to VoiceGG Mic",
                    mic_source
                );
                let _ = self.load_loopback(&mic_source, "voicegg_source_mic");
            }
        }

        // System defaults are preserved and NOT hijacked (PLAN.md §0.3 rule 4).
        tracing::info!("VoiceGG virtual devices created safely. System defaults preserved.");
        Ok(())
    }

    /// Destroys all VoiceGG virtual devices and restores original default audio devices.
    pub fn teardown_virtual_devices(&mut self) -> Result<()> {
        tracing::info!("Tearing down VoiceGG virtual devices...");

        // 1. Restore original default sink & source first
        if let Some(ref sink) = self.state.original_default_sink {
            tracing::info!("Restoring default output sink to '{}'", sink);
            let _ = Command::new("pactl")
                .args(["set-default-sink", sink])
                .output();
        }

        if let Some(ref source) = self.state.original_default_source {
            tracing::info!("Restoring default input source to '{}'", source);
            let _ = Command::new("pactl")
                .args(["set-default-source", source])
                .output();
        }

        // 2. Unload all modules in reverse order
        for &id in self.state.loaded_modules.iter().rev() {
            let _ = Command::new("pactl")
                .args(["unload-module", &id.to_string()])
                .output();
        }

        self.state.loaded_modules.clear();
        self.cycle_detector.clear();
        let _ = fs::remove_file(Self::state_file_path());
        tracing::info!("VoiceGG teardown complete. System audio restored to original state.");

        Ok(())
    }

    /// Discovers physical hardware audio devices (sinks and sources), filtering out VoiceGG virtual devices.
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        let mut devices = Vec::new();

        // Output Sinks
        if let Ok(output) = Command::new("pactl").args(["list", "sinks"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            devices.extend(Self::parse_pactl_devices(&text, DeviceType::Sink));
        }

        // Input Sources
        if let Ok(output) = Command::new("pactl").args(["list", "sources"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            devices.extend(Self::parse_pactl_devices(&text, DeviceType::Source));
        }

        Ok(devices)
    }

    fn parse_pactl_devices(text: &str, device_type: DeviceType) -> Vec<AudioDevice> {
        let mut devices = Vec::new();
        let blocks = text.split("\nSink #").flat_map(|s| s.split("\nSource #"));

        for block in blocks {
            let mut id = 0;
            let mut name = String::new();
            let mut description = String::new();

            for line in block.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Name: ") {
                    name = trimmed.trim_start_matches("Name: ").trim().to_string();
                } else if trimmed.starts_with("Description: ") {
                    description = trimmed
                        .trim_start_matches("Description: ")
                        .trim()
                        .to_string();
                } else if trimmed.starts_with("Sink #") || trimmed.starts_with("Source #") {
                    let num_str = trimmed.split('#').nth(1).unwrap_or("0").trim();
                    id = num_str.parse().unwrap_or(0);
                }
            }

            // Exclude VoiceGG virtual devices and monitor streams
            if !name.is_empty() && !name.starts_with("voicegg_") && !name.ends_with(".monitor") {
                if description.is_empty() {
                    description = name.clone();
                }
                devices.push(AudioDevice {
                    id,
                    name,
                    description,
                    device_type,
                    is_default: false,
                });
            }
        }

        devices
    }

    fn get_sink_id_to_name_map() -> std::collections::HashMap<u32, String> {
        let mut map = std::collections::HashMap::new();
        if let Ok(output) = Command::new("pactl")
            .args(["list", "sinks", "short"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    if let Ok(id) = parts[0].trim().parse::<u32>() {
                        map.insert(id, parts[1].trim().to_string());
                    }
                }
            }
        }
        map
    }

    /// Lists active application audio playback streams.
    pub fn list_active_streams(&self) -> Result<Vec<ActiveStream>> {
        let sink_map = Self::get_sink_id_to_name_map();
        let output = Command::new("pactl")
            .args(["list", "sink-inputs"])
            .output()
            .map_err(|e| PwError::Generic(format!("Failed to run pactl list sink-inputs: {e}")))?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut streams = Vec::new();

        for block in text.split("Sink Input #") {
            if block.trim().is_empty() {
                continue;
            }
            let mut id = 0;
            let mut app_name = String::new();
            let mut binary_name = String::new();
            let mut pid = None;
            let mut sink_name = String::new();

            for line in block.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Sink: ") {
                    let s_str = trimmed.trim_start_matches("Sink: ").trim();
                    if let Ok(s_id) = s_str.parse::<u32>() {
                        if let Some(name) = sink_map.get(&s_id) {
                            sink_name = name.clone();
                        } else {
                            sink_name = s_str.to_string();
                        }
                    } else {
                        sink_name = s_str.to_string();
                    }
                } else if trimmed.starts_with("application.name = \"") {
                    app_name = trimmed
                        .trim_start_matches("application.name = \"")
                        .trim_end_matches('"')
                        .to_string();
                } else if trimmed.starts_with("application.process.binary = \"") {
                    binary_name = trimmed
                        .trim_start_matches("application.process.binary = \"")
                        .trim_end_matches('"')
                        .to_string();
                } else if trimmed.starts_with("application.process.id = \"") {
                    let p_str = trimmed
                        .trim_start_matches("application.process.id = \"")
                        .trim_end_matches('"');
                    pid = p_str.parse().ok();
                }
            }

            if let Some(first_line) = block.lines().next() {
                id = first_line.trim().parse().unwrap_or(0);
            }

            let current_channel = match sink_name.as_str() {
                "voicegg_sink_game" => Some(ChannelId::Game),
                "voicegg_sink_chat" => Some(ChannelId::Chat),
                "voicegg_sink_media" => Some(ChannelId::Media),
                "voicegg_sink_aux" => Some(ChannelId::Aux),
                "voicegg_sink_master" => Some(ChannelId::Master),
                _ => None,
            };

            if id > 0 && (!app_name.is_empty() || !binary_name.is_empty()) {
                streams.push(ActiveStream {
                    id,
                    app_name,
                    binary_name,
                    pid,
                    current_channel,
                });
            }
        }

        Ok(streams)
    }

    /// Moves a specific playback stream to a VoiceGG target channel.
    pub fn route_stream(&self, stream_id: u32, target: ChannelId) -> Result<()> {
        let sink_name = target.pipewire_node_name();
        let output = Command::new("pactl")
            .args(["move-sink-input", &stream_id.to_string(), sink_name])
            .output()
            .map_err(|e| PwError::RoutingFailed(format!("Failed to move sink input: {e}")))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(PwError::RoutingFailed(format!(
                "Failed to route stream {stream_id} to {sink_name}: {err}"
            )));
        }

        tracing::info!("Stream {} routed to {}", stream_id, sink_name);
        Ok(())
    }

    /// Routes any active application matching the executable binary name to the specified channel.
    pub fn route_app_by_name(&self, binary: &str, target: ChannelId) -> Result<()> {
        let streams = self.list_active_streams()?;
        for stream in streams {
            if stream.binary_name.eq_ignore_ascii_case(binary) {
                self.route_stream(stream.id, target)?;
            }
        }
        Ok(())
    }

    /// Sets the volume of a channel sink or source in PipeWire (0 .. 150%).
    pub fn set_channel_volume(&self, channel: ChannelId, volume: u8) -> Result<()> {
        let node_name = channel.pipewire_node_name();
        let vol_str = format!("{volume}%");
        let cmd = if channel == ChannelId::Mic {
            "set-source-volume"
        } else {
            "set-sink-volume"
        };
        let _ = Command::new("pactl")
            .args([cmd, node_name, &vol_str])
            .output();
        Ok(())
    }

    /// Sets the mute status of a channel sink or source in PipeWire.
    pub fn set_channel_mute(&self, channel: ChannelId, muted: bool) -> Result<()> {
        let node_name = channel.pipewire_node_name();
        let mute_str = if muted { "1" } else { "0" };
        let cmd = if channel == ChannelId::Mic {
            "set-source-mute"
        } else {
            "set-sink-mute"
        };
        let _ = Command::new("pactl")
            .args([cmd, node_name, mute_str])
            .output();
        Ok(())
    }
}

impl Drop for GraphManager {
    fn drop(&mut self) {
        if self.initialized && !self.state.loaded_modules.is_empty() {
            let _ = self.teardown_virtual_devices();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_detector_direct_loop() {
        let detector = GraphCycleDetector::new();
        // Self-loop
        assert!(detector.would_cycle("nodeA", "nodeA"));
    }

    #[test]
    fn test_cycle_detector_multi_hop_loop() {
        let mut detector = GraphCycleDetector::new();
        detector.add_edge("A".to_string(), "B".to_string());
        detector.add_edge("B".to_string(), "C".to_string());

        // A -> B -> C: adding C -> A should be detected as a cycle
        assert!(detector.would_cycle("C", "A"));

        // Adding A -> D should be completely safe
        assert!(!detector.would_cycle("A", "D"));
        detector.add_edge("A".to_string(), "D".to_string());

        // D -> C is safe
        assert!(!detector.would_cycle("D", "C"));

        // D -> A would cycle (A -> D -> A)
        assert!(detector.would_cycle("D", "A"));
    }
}
