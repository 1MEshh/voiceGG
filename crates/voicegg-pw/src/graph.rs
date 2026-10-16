//! PipeWire and PulseAudio audio graph manager.

use crate::chain::{
    detect_capabilities, generate_channel_chain_conf, generate_mic_chain_conf, FilterChainRunner,
};
use crate::device::{ActiveStream, AudioDevice, DeviceType};
use crate::dsp_engine::DspEngine;
use crate::error::{PwError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use voicegg_core::{ChannelId, PluginCapabilities, Preset};

#[derive(Debug, Default, Serialize, Deserialize)]
struct GraphState {
    original_default_sink: Option<String>,
    original_default_source: Option<String>,
    loaded_modules: Vec<u32>,
    master_output_module: Option<u32>,
    mic_input_module: Option<u32>,
    #[serde(default)]
    mic_monitor_module: Option<u32>,
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
    capabilities: PluginCapabilities,
    filter_chains: std::collections::HashMap<ChannelId, FilterChainRunner>,
    pub dsp_engine: DspEngine,
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
            capabilities: detect_capabilities(),
            filter_chains: std::collections::HashMap::new(),
            dsp_engine: DspEngine::new(),
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

        // Cleanup sweep for any dangling VoiceGG modules
        if let Ok(output) = Command::new("pactl")
            .args(["list", "modules", "short"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("voicegg_") {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if let Some(id_str) = parts.first() {
                        if let Ok(id) = id_str.trim().parse::<u32>() {
                            tracing::info!("Sweep: unloading orphaned module {id}");
                            let _ = Command::new("pactl")
                                .args(["unload-module", &id.to_string()])
                                .output();
                        }
                    }
                }
            }
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

        // 1. Detect physical default sink
        if let Ok(out) = Command::new("pactl").arg("get-default-sink").output() {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !name.is_empty() && !name.starts_with("voicegg_") {
                    self.state.original_default_sink = Some(name);
                }
            }
        }

        // If original_default_sink is None or starts with voicegg_, scan list_devices()
        // and select the first physical hardware sink (alsa_output.* or bluez_output.*).
        let needs_sink_fallback = match &self.state.original_default_sink {
            None => true,
            Some(s) => s.is_empty() || s.starts_with("voicegg_"),
        };
        if needs_sink_fallback {
            if let Ok(devices) = self.list_devices() {
                if let Some(dev) = devices.iter().find(|d| {
                    d.device_type == DeviceType::Sink
                        && (d.name.starts_with("alsa_output.") || d.name.starts_with("bluez_output."))
                }).or_else(|| {
                    devices.iter().find(|d| d.device_type == DeviceType::Sink)
                }) {
                    tracing::info!("Selected physical hardware sink: {}", dev.name);
                    self.state.original_default_sink = Some(dev.name.clone());
                }
            }
        }

        // Detect physical default source
        if let Ok(out) = Command::new("pactl").arg("get-default-source").output() {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !name.is_empty() && !name.starts_with("voicegg_") && !name.ends_with(".monitor") {
                    self.state.original_default_source = Some(name);
                }
            }
        }

        let needs_source_fallback = match &self.state.original_default_source {
            None => true,
            Some(s) => s.is_empty() || s.starts_with("voicegg_") || s.ends_with(".monitor"),
        };
        if needs_source_fallback {
            if let Ok(devices) = self.list_devices() {
                if let Some(dev) = devices.iter().find(|d| {
                    d.device_type == DeviceType::Source
                        && (d.name.starts_with("alsa_input.") || d.name.starts_with("bluez_input."))
                }).or_else(|| {
                    devices.iter().find(|d| d.device_type == DeviceType::Source)
                }) {
                    tracing::info!("Selected physical hardware source: {}", dev.name);
                    self.state.original_default_source = Some(dev.name.clone());
                }
            }
        }

        // 2. Create the virtual sinks reliably using load_null_sink
        self.load_null_sink("voicegg_sink_master", "VoiceGG Master Output", false)?;
        self.load_null_sink("voicegg_sink_game", "VoiceGG Game Audio", false)?;
        self.load_null_sink("voicegg_sink_chat", "VoiceGG Voice Chat", false)?;
        self.load_null_sink("voicegg_sink_media", "VoiceGG Media / Music", false)?;
        self.load_null_sink("voicegg_sink_aux", "VoiceGG Auxiliary Audio", false)?;
        self.load_null_sink("voicegg_source_stream_mix", "VoiceGG Stream Mix", false)?;
        self.load_null_sink("voicegg_mic_sink", "VoiceGG Mic Sink", false)?;
        self.load_null_sink("voicegg_source_mic", "VoiceGG Processed Microphone", true)?;

        // 3. Link voicegg_sink_master.monitor to original_default_sink with loopback
        if let Some(target_sink) = self.state.original_default_sink.clone() {
            tracing::info!(
                "Connecting VoiceGG Master to physical sink '{}'",
                target_sink
            );
            if let Ok(id) = self.load_loopback("voicegg_sink_master.monitor", &target_sink) {
                self.state.master_output_module = Some(id);
            }
        }

        // 4. Connect playback channels (Game, Chat, Media, Aux) to voicegg_sink_master with loopbacks
        for ch in [
            ChannelId::Game,
            ChannelId::Chat,
            ChannelId::Media,
            ChannelId::Aux,
        ] {
            let src = format!("{}.monitor", ch.pipewire_node_name());
            if let Err(e) = self.load_loopback(&src, "voicegg_sink_master") {
                tracing::warn!("Failed to link playback channel {} to master: {e}", ch);
            }
        }

        // 5. Connect voicegg_mic_sink.monitor to voicegg_source_mic with loopback
        if let Err(e) = self.load_loopback("voicegg_mic_sink.monitor", "voicegg_source_mic") {
            tracing::warn!("Failed to link mic sink to mic source: {e}");
        }

        // 6. Initialize dsp_engine and spawn mic capture worker from original_default_source to voicegg_mic_sink / voicegg_source_mic
        self.dsp_engine = DspEngine::new();
        self.dsp_engine.spawn_mic_worker(
            self.state.original_default_source.as_deref(),
            Some("voicegg_mic_sink"),
        );

        tracing::info!("VoiceGG virtual devices created reliably. System defaults preserved.");
        Ok(())
    }

    /// Destroys all VoiceGG virtual devices and restores original default audio devices.
    pub fn teardown_virtual_devices(&mut self) -> Result<()> {
        tracing::info!("Tearing down VoiceGG virtual devices...");

        // 1. Move all active sink-inputs from voicegg_* sinks back to original_default_sink via pactl move-sink-input
        let default_sink = self.state.original_default_sink.clone().or_else(|| {
            Command::new("pactl")
                .arg("get-default-sink")
                .output()
                .ok()
                .and_then(|out| {
                    if out.status.success() {
                        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !name.is_empty() && !name.starts_with("voicegg_") {
                            Some(name)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
        });

        if let Some(ref sink) = default_sink {
            let sink_map = Self::get_sink_id_to_name_map();
            if let Ok(output) = Command::new("pactl").args(["list", "sink-inputs"]).output() {
                let text = String::from_utf8_lossy(&output.stdout);
                for block in text.split("Sink Input #") {
                    if block.trim().is_empty() {
                        continue;
                    }
                    let mut input_id = 0;
                    let mut input_sink = String::new();
                    for line in block.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("Sink: ") {
                            let s_str = trimmed.trim_start_matches("Sink: ").trim();
                            if let Ok(s_id) = s_str.parse::<u32>() {
                                if let Some(name) = sink_map.get(&s_id) {
                                    input_sink = name.clone();
                                } else {
                                    input_sink = s_str.to_string();
                                }
                            } else {
                                input_sink = s_str.to_string();
                            }
                        }
                    }
                    if let Some(first_line) = block.lines().next() {
                        input_id = first_line.trim().parse().unwrap_or(0);
                    }
                    if input_id > 0 && input_sink.starts_with("voicegg_") {
                        tracing::info!(
                            "Moving sink-input {} from '{}' back to '{}'",
                            input_id,
                            input_sink,
                            sink
                        );
                        let _ = Command::new("pactl")
                            .args(["move-sink-input", &input_id.to_string(), sink])
                            .output();
                    }
                }
            }
        }

        // 2. Stop dsp_engine.stop_all()
        self.dsp_engine.stop_all();

        // 3. Unload mic monitor module if active
        let _ = self.set_mic_monitor(false, 0);

        // Terminate all filter-chain child processes
        self.filter_chains.clear();

        // Restore original default sink & source
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

        // 4. Unload all modules in reverse order
        for &id in self.state.loaded_modules.iter().rev() {
            let _ = Command::new("pactl")
                .args(["unload-module", &id.to_string()])
                .output();
        }

        self.state.loaded_modules.clear();
        self.cycle_detector.clear();

        // 5. Final cleanup sweep: query pactl list modules short, find any module with voicegg_ in its arguments, and unload it
        if let Ok(output) = Command::new("pactl")
            .args(["list", "modules", "short"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("voicegg_") {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if let Some(id_str) = parts.first() {
                        if let Ok(id) = id_str.trim().parse::<u32>() {
                            tracing::info!("Cleanup sweep: unloading module {id}");
                            let _ = Command::new("pactl")
                                .args(["unload-module", &id.to_string()])
                                .output();
                        }
                    }
                }
            }
        }

        // 6. Remove modules.json state file
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
        Ok(Self::parse_sink_inputs_text(&text, &sink_map))
    }

    pub(crate) fn parse_sink_inputs_text(
        text: &str,
        sink_map: &std::collections::HashMap<u32, String>,
    ) -> Vec<ActiveStream> {
        let mut streams = Vec::new();

        for block in text.split("Sink Input #") {
            if block.trim().is_empty() {
                continue;
            }
            let mut id = 0;
            let mut app_name = String::new();
            let mut binary_name = String::new();
            let mut media_name = String::new();
            let mut node_name = String::new();
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
                } else if trimmed.starts_with("media.name = \"") {
                    media_name = trimmed
                        .trim_start_matches("media.name = \"")
                        .trim_end_matches('"')
                        .to_string();
                } else if trimmed.starts_with("node.name = \"") {
                    node_name = trimmed
                        .trim_start_matches("node.name = \"")
                        .trim_end_matches('"')
                        .to_string();
                }
            }

            if let Some(first_line) = block.lines().next() {
                id = first_line.trim().parse().unwrap_or(0);
            }

            // Fallbacks for empty names
            if app_name.is_empty() {
                if !node_name.is_empty() {
                    app_name = node_name.clone();
                } else if !media_name.is_empty() {
                    app_name = media_name.clone();
                } else if !binary_name.is_empty() {
                    app_name = binary_name.clone();
                }
            }
            if binary_name.is_empty() {
                if !node_name.is_empty() {
                    binary_name = node_name.clone();
                } else if !app_name.is_empty() {
                    binary_name = app_name.clone();
                }
            }

            // Recognize Discord (binary = "Discord" or name = "WEBRTC VoiceEngine")
            let is_discord = binary_name.eq_ignore_ascii_case("discord")
                || binary_name.eq_ignore_ascii_case("discordcanary")
                || binary_name.eq_ignore_ascii_case("vesktop")
                || binary_name.eq_ignore_ascii_case("webcord")
                || app_name.contains("WEBRTC")
                || app_name.eq_ignore_ascii_case("Discord");

            if is_discord {
                if binary_name.is_empty() || binary_name.contains("WEBRTC") {
                    binary_name = "Discord".to_string();
                }
                if app_name.is_empty() {
                    app_name = "Discord".to_string();
                }
            }

            // Recognize Chrome / Chromium (binary = "chrome" / "chromium")
            let is_chrome = binary_name.eq_ignore_ascii_case("chrome")
                || binary_name.eq_ignore_ascii_case("chromium")
                || binary_name.eq_ignore_ascii_case("google-chrome")
                || binary_name.eq_ignore_ascii_case("google-chrome-stable")
                || binary_name.eq_ignore_ascii_case("brave")
                || app_name.to_ascii_lowercase().contains("google chrome")
                || app_name.to_ascii_lowercase().contains("chromium");

            if is_chrome {
                if binary_name.is_empty() {
                    binary_name = "chrome".to_string();
                }
                if app_name.is_empty() {
                    app_name = "Google Chrome".to_string();
                }
            }

            // Recognize Firefox
            if binary_name.eq_ignore_ascii_case("firefox")
                || app_name.to_ascii_lowercase().contains("firefox")
            {
                if binary_name.is_empty() {
                    binary_name = "firefox".to_string();
                }
                if app_name.is_empty() {
                    app_name = "Firefox".to_string();
                }
            }

            // Recognize Spotify
            if binary_name.eq_ignore_ascii_case("spotify")
                || app_name.to_ascii_lowercase().contains("spotify")
            {
                if binary_name.is_empty() {
                    binary_name = "spotify".to_string();
                }
                if app_name.is_empty() {
                    app_name = "Spotify".to_string();
                }
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

        streams
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
            let is_match = stream.binary_name.eq_ignore_ascii_case(binary)
                || stream.app_name.eq_ignore_ascii_case(binary)
                || (binary.eq_ignore_ascii_case("discord") && stream.app_name.contains("WEBRTC"));

            if is_match {
                self.route_stream(stream.id, target)?;
            }
        }
        Ok(())
    }

    /// Sets the volume of a specific playback stream (sink-input) by ID (0 .. 150%).
    pub fn set_stream_volume(&self, stream_id: u32, volume: u8) -> Result<()> {
        let vol_str = format!("{volume}%");
        let output = Command::new("pactl")
            .args(["set-sink-input-volume", &stream_id.to_string(), &vol_str])
            .output()
            .map_err(|e| PwError::Generic(format!("Failed to set sink input volume: {e}")))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(PwError::Generic(format!(
                "Failed to set volume on stream {stream_id}: {err}"
            )));
        }
        Ok(())
    }

    /// Sets the volume for all active streams matching the binary name.
    pub fn set_app_volume_by_name(&self, binary: &str, volume: u8) -> Result<()> {
        if let Ok(streams) = self.list_active_streams() {
            for stream in streams {
                if stream.binary_name.eq_ignore_ascii_case(binary) {
                    let _ = self.set_stream_volume(stream.id, volume);
                }
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

    /// Changes the hardware output sink that VoiceGG Master forwards to.
    pub fn set_output_sink(&mut self, sink_name: &str) -> Result<()> {
        let target = if sink_name == "default" || sink_name.is_empty() {
            self.state.original_default_sink.clone().unwrap_or_default()
        } else {
            sink_name.to_string()
        };

        if target.is_empty() {
            return Err(PwError::Generic("No valid target output sink".into()));
        }

        if let Some(mod_id) = self.state.master_output_module.take() {
            let _ = Command::new("pactl")
                .args(["unload-module", &mod_id.to_string()])
                .output();
            self.state.loaded_modules.retain(|&id| id != mod_id);
        }

        tracing::info!("Switching VoiceGG Master output to '{}'", target);
        let mod_id = self.load_loopback("voicegg_sink_master.monitor", &target)?;
        self.state.master_output_module = Some(mod_id);
        self.save_state();
        Ok(())
    }

    /// Changes the hardware input microphone source feeding into VoiceGG Mic.
    pub fn set_input_source(&mut self, source_name: &str) -> Result<()> {
        let target = if source_name == "default" || source_name.is_empty() {
            self.state
                .original_default_source
                .clone()
                .unwrap_or_default()
        } else {
            source_name.to_string()
        };

        if target.ends_with(".monitor")
            || target.starts_with("alsa_output")
            || target.starts_with("voicegg_")
        {
            return Err(PwError::Generic(format!(
                "Cannot use '{target}' as microphone source: feedback safety protection."
            )));
        }

        if let Some(mod_id) = self.state.mic_input_module.take() {
            let _ = Command::new("pactl")
                .args(["unload-module", &mod_id.to_string()])
                .output();
            self.state.loaded_modules.retain(|&id| id != mod_id);
        }

        tracing::info!("Switching VoiceGG Mic input to '{}'", target);
        let mod_id = self.load_loopback(&target, "voicegg_mic_filter_in")?;
        self.state.mic_input_module = Some(mod_id);
        self.save_state();
        Ok(())
    }

    /// Enables or disables real-time microphone sidetone monitoring with feedback protection.
    pub fn set_mic_monitor(&mut self, enabled: bool, volume: u8) -> Result<()> {
        if enabled {
            let output_sink = match self.state.original_default_sink.as_ref() {
                Some(s) if !s.is_empty() => s.clone(),
                _ => {
                    return Err(PwError::Generic(
                        "No physical output sink available for microphone monitoring".into(),
                    ))
                }
            };

            // Feedback guard: physical output sink cannot match physical input mic
            if let Some(ref input_mic) = self.state.original_default_source {
                if &output_sink == input_mic {
                    return Err(PwError::Generic(
                        "Feedback guard: cannot route microphone monitor to input source".into(),
                    ));
                }
            }

            if output_sink.starts_with("voicegg_") {
                return Err(PwError::Generic(
                    "Feedback guard: cannot monitor microphone into a VoiceGG virtual channel"
                        .into(),
                ));
            }

            // Cycle detector checks would_cycle("voicegg_source_mic", &output_sink)
            if self
                .cycle_detector
                .would_cycle("voicegg_source_mic", &output_sink)
            {
                return Err(PwError::Generic(format!(
                    "Feedback loop prevented: linking 'voicegg_source_mic' to '{output_sink}' would cycle"
                )));
            }

            // Unload previous mic monitor module if active
            if let Some(old_id) = self.state.mic_monitor_module.take() {
                let _ = Command::new("pactl")
                    .args(["unload-module", &old_id.to_string()])
                    .output();
                self.state.loaded_modules.retain(|&id| id != old_id);
            }

            let src_arg = "source=voicegg_source_mic";
            let sink_arg = format!("sink={output_sink}");
            let vol_str = format!("sink_input_properties=volume={}%", volume.min(100));

            let output = Command::new("pactl")
                .args([
                    "load-module",
                    "module-loopback",
                    src_arg,
                    &sink_arg,
                    "latency_msec=5",
                    &vol_str,
                ])
                .output()
                .map_err(|e| {
                    PwError::Generic(format!("Failed to load mic monitor loopback: {e}"))
                })?;

            let module_id: u32 = if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                out_str.parse().map_err(|_| {
                    PwError::Generic(format!("pactl did not return a valid module ID: {out_str}"))
                })?
            } else {
                // Fallback without sink_input_properties if unsupported
                let retry = Command::new("pactl")
                    .args([
                        "load-module",
                        "module-loopback",
                        src_arg,
                        &sink_arg,
                        "latency_msec=5",
                    ])
                    .output()
                    .map_err(|e| {
                        PwError::Generic(format!("Failed to load mic monitor loopback: {e}"))
                    })?;

                if !retry.status.success() {
                    let err = String::from_utf8_lossy(&retry.stderr);
                    return Err(PwError::Generic(format!(
                        "Failed to enable microphone monitor: {err}"
                    )));
                }

                let out_str = String::from_utf8_lossy(&retry.stdout).trim().to_string();
                out_str.parse().map_err(|_| {
                    PwError::Generic(format!("pactl did not return a valid module ID: {out_str}"))
                })?
            };

            self.cycle_detector
                .add_edge("voicegg_source_mic".to_string(), output_sink);
            self.state.loaded_modules.push(module_id);
            self.state.mic_monitor_module = Some(module_id);
            self.save_state();
            Ok(())
        } else {
            if let Some(mod_id) = self.state.mic_monitor_module.take() {
                let _ = Command::new("pactl")
                    .args(["unload-module", &mod_id.to_string()])
                    .output();
                self.state.loaded_modules.retain(|&id| id != mod_id);
                self.save_state();
            }
            Ok(())
        }
    }

    /// Returns detected DSP capabilities.
    #[must_use]
    pub fn get_capabilities(&self) -> PluginCapabilities {
        self.capabilities
    }

    /// Samples peak and RMS meter levels (in dBFS) across channels.
    #[must_use]
    pub fn sample_meters(&self) -> std::collections::HashMap<ChannelId, (f32, f32)> {
        let mut meters = std::collections::HashMap::new();

        let engine_meters = self.dsp_engine.sample_meters();
        for (ch, meter) in engine_meters {
            meters.insert(ch, (meter.peak_dbfs, meter.rms_dbfs));
        }

        let active_channels: std::collections::HashSet<ChannelId> = self
            .list_active_streams()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| s.current_channel)
            .collect();

        for channel in [
            ChannelId::Master,
            ChannelId::Game,
            ChannelId::Chat,
            ChannelId::Media,
            ChannelId::Aux,
            ChannelId::Mic,
            ChannelId::StreamMix,
        ] {
            meters.entry(channel).or_insert_with(|| {
                if active_channels.contains(&channel) {
                    (-12.0, -18.0)
                } else {
                    (-100.0, -100.0)
                }
            });
        }

        meters
    }

    /// Re-applies an audio preset to a channel's filter chain runner and DSP engine.
    pub fn apply_preset(&mut self, channel: ChannelId, preset: &Preset) -> Result<()> {
        self.dsp_engine.update_channel_preset(channel, preset);

        if channel == ChannelId::Mic {
            let conf = generate_mic_chain_conf(preset, &self.capabilities);
            if let Some(runner) = self.filter_chains.get_mut(&ChannelId::Mic) {
                let _ = runner.reload(&conf);
            }
        } else if [
            ChannelId::Game,
            ChannelId::Chat,
            ChannelId::Media,
            ChannelId::Aux,
        ]
        .contains(&channel)
        {
            let conf = generate_channel_chain_conf(channel, preset, &self.capabilities);
            if let Some(runner) = self.filter_chains.get_mut(&channel) {
                let _ = runner.reload(&conf);
            }
        }
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

    #[test]
    fn test_capabilities_and_meters() {
        let gm = GraphManager::new();
        let caps = gm.get_capabilities();
        assert!(caps.biquad_eq);

        let meters = gm.sample_meters();
        assert!(meters.contains_key(&ChannelId::Master));
        assert!(meters.contains_key(&ChannelId::Mic));
        let (peak, rms) = meters.get(&ChannelId::Mic).copied().unwrap();
        assert!(peak <= 0.0);
        assert!(rms <= 0.0);
    }

    #[test]
    fn test_mic_monitor_safety_checks() {
        let mut gm = GraphManager::new();
        // Without physical sink configured, enabling mic monitor must return an error
        assert!(gm.set_mic_monitor(true, 50).is_err());

        // Disabling mic monitor when none is active should succeed cleanly
        assert!(gm.set_mic_monitor(false, 0).is_ok());

        // Test feedback guard when physical output matches microphone input
        gm.state.original_default_sink = Some("device_mic".to_string());
        gm.state.original_default_source = Some("device_mic".to_string());
        let res = gm.set_mic_monitor(true, 50);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Feedback guard"));
    }

    #[test]
    fn test_parse_active_streams_discord_and_chrome() {
        let sample = r#"
Sink Input #42
	Driver: PipeWire
	Sink: 10
	application.name = "WEBRTC VoiceEngine"
	application.process.binary = "Discord"
	application.process.id = "12345"

Sink Input #43
	Driver: PipeWire
	Sink: 11
	application.name = "Google Chrome"
	application.process.binary = "chrome"
	application.process.id = "54321"

Sink Input #44
	Driver: PipeWire
	Sink: 12
	application.name = "Spotify"
	application.process.binary = "spotify"
"#;
        let mut map = std::collections::HashMap::new();
        map.insert(10, "voicegg_sink_chat".to_string());
        map.insert(11, "voicegg_sink_media".to_string());
        map.insert(12, "voicegg_sink_game".to_string());

        let streams = GraphManager::parse_sink_inputs_text(sample, &map);
        assert_eq!(streams.len(), 3);

        let discord = streams.iter().find(|s| s.id == 42).unwrap();
        assert_eq!(discord.binary_name, "Discord");
        assert_eq!(discord.current_channel, Some(ChannelId::Chat));

        let chrome = streams.iter().find(|s| s.id == 43).unwrap();
        assert_eq!(chrome.binary_name, "chrome");
        assert_eq!(chrome.current_channel, Some(ChannelId::Media));

        let spotify = streams.iter().find(|s| s.id == 44).unwrap();
        assert_eq!(spotify.binary_name, "spotify");
        assert_eq!(spotify.current_channel, Some(ChannelId::Game));
    }
}
