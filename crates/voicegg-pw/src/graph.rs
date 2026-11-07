//! PipeWire and PulseAudio audio graph manager.

use crate::chain::{
    detect_capabilities, generate_channel_chain_conf, generate_mic_chain_conf, FilterChainRunner,
};
use crate::device::{ActiveStream, AudioDevice, DeviceType};
use crate::dsp_engine::DspEngine;
use crate::error::{PwError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use voicegg_core::{ChannelId, PluginCapabilities, Preset};

/// Helper to create a `Command` invoking `pactl` with `LC_ALL=C`
/// to guarantee standard English output across all locales.
fn pactl_cmd() -> Command {
    let mut cmd = Command::new("pactl");
    cmd.env("LC_ALL", "C");
    cmd
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct GraphState {
    original_default_sink: Option<String>,
    original_default_source: Option<String>,
    loaded_modules: Vec<u32>,
    master_output_module: Option<u32>,
    mic_input_module: Option<u32>,
    #[serde(default)]
    mic_monitor_module: Option<u32>,
    #[serde(default)]
    stream_mix_module: Option<u32>,
    #[serde(default)]
    active_hardware_sink: Option<String>,
    #[serde(default)]
    active_hardware_source: Option<String>,
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
    active_hardware_sink: Option<String>,
    active_hardware_source: Option<String>,
    streamer_mode: bool,
    mic_monitor_volume: u8,
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
            active_hardware_sink: None,
            active_hardware_source: None,
            streamer_mode: false,
            mic_monitor_volume: 50,
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

    /// Returns the currently active hardware output sink.
    #[must_use]
    pub fn active_output_sink(&self) -> Option<&str> {
        self.active_hardware_sink.as_deref()
    }

    /// Returns the currently active hardware microphone source.
    #[must_use]
    pub fn active_input_source(&self) -> Option<&str> {
        self.active_hardware_source.as_deref()
    }

    /// Returns the original system default output sink recorded at startup.
    #[must_use]
    pub fn original_default_sink(&self) -> Option<&str> {
        self.state.original_default_sink.as_deref()
    }

    /// Returns the original system default input source recorded at startup.
    #[must_use]
    pub fn original_default_source(&self) -> Option<&str> {
        self.state.original_default_source.as_deref()
    }

    /// Returns whether streamer mode is currently enabled.
    #[must_use]
    pub fn is_streamer_mode(&self) -> bool {
        self.streamer_mode
    }

    /// Terminates any orphaned `parec` or `pacat` worker processes spawned by VoiceGG
    /// by inspecting `/proc/<pid>/comm` and `/proc/<pid>/cmdline`.
    fn kill_orphaned_dsp_processes() {
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let pid_str = file_name.to_string_lossy();
                if !pid_str.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                let comm_path = entry.path().join("comm");
                let cmdline_path = entry.path().join("cmdline");
                if let Ok(comm) = fs::read_to_string(&comm_path) {
                    let comm_trimmed = comm.trim();
                    if comm_trimmed == "parec" || comm_trimmed == "pacat" {
                        if let Ok(cmdline_bytes) = fs::read(&cmdline_path) {
                            let cmdline = String::from_utf8_lossy(&cmdline_bytes);
                            if cmdline.contains("VoiceGG") || cmdline.contains("voicegg_") {
                                let _ = Command::new("kill")
                                    .args(["-9", &pid_str])
                                    .status();
                            }
                        }
                    }
                }
            }
        }
    }

    /// Emergency restoration of system audio without requiring an active `GraphManager` instance.
    /// Can be invoked by watchdog/janitor, CLI recovery, and signal crash handlers.
    pub fn emergency_restore_system_audio() {
        Self::restore_and_unload_all(None, None);
    }

    /// Removes any stale loaded VoiceGG modules from PipeWire and restores defaults.
    pub fn cleanup_stale_modules() {
        Self::restore_and_unload_all(None, None);
    }

    /// Unified routine to restore system audio, migrate all streams, unload modules,
    /// reset quantum, and scrub WirePlumber persistent state.
    fn restore_and_unload_all(
        preferred_sink: Option<&str>,
        preferred_source: Option<&str>,
    ) {
        // 1. Stop parec/pacat processes spawned by VoiceGG
        Self::kill_orphaned_dsp_processes();

        // 2. Resolve verified physical sink and source
        let phys_sink = Self::resolve_physical_sink(preferred_sink);
        let phys_source = Self::resolve_physical_source(preferred_source);

        // 3. Set physical defaults FIRST so WirePlumber never dumps streams onto dying sinks
        if let Some(ref sink) = phys_sink {
            tracing::info!("Restoring system default sink to '{}'", sink);
            let _ = pactl_cmd().args(["set-default-sink", sink]).output();
        }
        if let Some(ref source) = phys_source {
            tracing::info!("Restoring system default source to '{}'", source);
            let _ = pactl_cmd().args(["set-default-source", source]).output();
        }

        // 4. Migrate ALL active sink-inputs and source-outputs to physical hardware
        if let Some(ref sink) = phys_sink {
            Self::migrate_sink_inputs_to(sink);
        }
        if let Some(ref source) = phys_source {
            Self::migrate_source_outputs_to(source);
        }

        // 5. Unload all voicegg modules in reverse order
        let state_path = Self::state_file_path();
        if state_path.exists() {
            if let Ok(content) = fs::read_to_string(&state_path) {
                if let Ok(old_state) = serde_json::from_str::<GraphState>(&content) {
                    for &id in old_state.loaded_modules.iter().rev() {
                        let _ = pactl_cmd().args(["unload-module", &id.to_string()]).output();
                    }
                }
            }
            let _ = fs::remove_file(&state_path);
        }

        // Final sweep for any dangling VoiceGG modules
        if let Ok(output) = pactl_cmd().args(["list", "modules", "short"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("voicegg_") || line.contains("VoiceGG") {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if let Some(id_str) = parts.first() {
                        if let Ok(id) = id_str.trim().parse::<u32>() {
                            tracing::info!("Sweep: unloading orphaned module {id}");
                            let _ = pactl_cmd().args(["unload-module", &id.to_string()]).output();
                        }
                    }
                }
            }
        }

        // 6. Reset PipeWire quantum
        let _ = Command::new("pw-metadata")
            .args(["-n", "settings", "0", "clock.force-quantum", "0"])
            .output();

        // 7. Scrub WirePlumber persistence
        Self::scrub_wireplumber_state(phys_sink.as_deref(), phys_source.as_deref());
    }

    /// Resolves a genuine physical hardware sink that exists and does not belong to VoiceGG.
    fn resolve_physical_sink(preferred: Option<&str>) -> Option<String> {
        let is_valid_phys = |name: &str| -> bool {
            !name.is_empty()
                && !name.starts_with("voicegg_")
                && !name.contains("voicegg_")
                && !name.ends_with(".monitor")
        };

        if let Some(pref) = preferred {
            if is_valid_phys(pref) {
                return Some(pref.to_string());
            }
        }

        // Check saved state file
        let state_path = Self::state_file_path();
        if let Ok(content) = fs::read_to_string(&state_path) {
            if let Ok(state) = serde_json::from_str::<GraphState>(&content) {
                if let Some(sink) = state.active_hardware_sink.or(state.original_default_sink) {
                    if is_valid_phys(&sink) {
                        return Some(sink);
                    }
                }
            }
        }

        // Check get-default-sink
        if let Ok(out) = pactl_cmd().arg("get-default-sink").output() {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if is_valid_phys(&name) {
                    return Some(name);
                }
            }
        }

        // Scan list sinks short
        if let Ok(output) = pactl_cmd().args(["list", "sinks", "short"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut fallback = None;
            for line in text.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    let name = parts[1].trim();
                    if is_valid_phys(name) {
                        if name.starts_with("alsa_output.") || name.starts_with("bluez_output.") {
                            return Some(name.to_string());
                        }
                        if fallback.is_none() {
                            fallback = Some(name.to_string());
                        }
                    }
                }
            }
            if let Some(f) = fallback {
                return Some(f);
            }
        }

        None
    }

    /// Resolves a genuine physical hardware source (mic) that exists and does not belong to VoiceGG.
    fn resolve_physical_source(preferred: Option<&str>) -> Option<String> {
        let is_valid_phys = |name: &str| -> bool {
            !name.is_empty()
                && !name.starts_with("voicegg_")
                && !name.contains("voicegg_")
                && !name.ends_with(".monitor")
                && !name.starts_with("alsa_output")
        };

        if let Some(pref) = preferred {
            if is_valid_phys(pref) {
                return Some(pref.to_string());
            }
        }

        // Check saved state file
        let state_path = Self::state_file_path();
        if let Ok(content) = fs::read_to_string(&state_path) {
            if let Ok(state) = serde_json::from_str::<GraphState>(&content) {
                if let Some(src) = state.active_hardware_source.or(state.original_default_source) {
                    if is_valid_phys(&src) {
                        return Some(src);
                    }
                }
            }
        }

        // Check get-default-source
        if let Ok(out) = pactl_cmd().arg("get-default-source").output() {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if is_valid_phys(&name) {
                    return Some(name);
                }
            }
        }

        // Scan list sources short
        if let Ok(output) = pactl_cmd().args(["list", "sources", "short"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut fallback = None;
            for line in text.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    let name = parts[1].trim();
                    if is_valid_phys(name) {
                        if name.starts_with("alsa_input.") || name.starts_with("bluez_input.") {
                            return Some(name.to_string());
                        }
                        if fallback.is_none() {
                            fallback = Some(name.to_string());
                        }
                    }
                }
            }
            if let Some(f) = fallback {
                return Some(f);
            }
        }

        None
    }

    /// Moves all active sink inputs attached to VoiceGG sinks back to the target physical sink.
    fn migrate_sink_inputs_to(target_sink: &str) {
        let sink_map = Self::get_sink_id_to_name_map();
        if let Ok(output) = pactl_cmd().args(["list", "sink-inputs"]).output() {
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

                if input_id > 0
                    && (input_sink.starts_with("voicegg_")
                        || input_sink.contains("voicegg_")
                        || block.contains("voicegg_sink_"))
                {
                    tracing::info!(
                        "Migrating sink-input {} from '{}' to '{}'",
                        input_id,
                        input_sink,
                        target_sink
                    );
                    let _ = pactl_cmd()
                        .args(["move-sink-input", &input_id.to_string(), target_sink])
                        .output();
                }
            }
        }
    }

    /// Moves all active source outputs attached to VoiceGG sources back to the target physical source.
    fn migrate_source_outputs_to(target_source: &str) {
        let source_map = Self::get_source_id_to_name_map();
        if let Ok(output) = pactl_cmd().args(["list", "source-outputs"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for block in text.split("Source Output #") {
                if block.trim().is_empty() {
                    continue;
                }
                let mut output_id = 0;
                let mut output_source = String::new();

                for line in block.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Source: ") {
                        let s_str = trimmed.trim_start_matches("Source: ").trim();
                        if let Ok(s_id) = s_str.parse::<u32>() {
                            if let Some(name) = source_map.get(&s_id) {
                                output_source = name.clone();
                            } else {
                                output_source = s_str.to_string();
                            }
                        } else {
                            output_source = s_str.to_string();
                        }
                    }
                }
                if let Some(first_line) = block.lines().next() {
                    output_id = first_line.trim().parse().unwrap_or(0);
                }

                if output_id > 0
                    && (output_source.starts_with("voicegg_")
                        || output_source.contains("voicegg_")
                        || block.contains("voicegg_source_"))
                {
                    tracing::info!(
                        "Migrating source-output {} from '{}' to '{}'",
                        output_id,
                        output_source,
                        target_source
                    );
                    let _ = pactl_cmd()
                        .args(["move-source-output", &output_id.to_string(), target_source])
                        .output();
                }
            }
        }
    }

    /// Scrubs WirePlumber's persistent state files (`stream-properties` and `default-nodes`)
    /// removing any target overrides pointing to VoiceGG virtual sinks/sources.
    pub fn scrub_wireplumber_state(phys_sink: Option<&str>, phys_source: Option<&str>) {
        let base_dir = dirs::state_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local/state")))
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let wp_dir = base_dir.join("wireplumber");
        Self::scrub_wireplumber_in_dir(&wp_dir, phys_sink, phys_source);
    }

    /// Scrubs WirePlumber state files in a specified directory (usable for testing and production).
    pub fn scrub_wireplumber_in_dir(
        wp_dir: &Path,
        phys_sink: Option<&str>,
        phys_source: Option<&str>,
    ) {
        if !wp_dir.exists() {
            return;
        }

        // 1. Scrub default-nodes
        let default_nodes_path = wp_dir.join("default-nodes");
        if default_nodes_path.exists() {
            if let Ok(content) = fs::read_to_string(&default_nodes_path) {
                let mut cleaned_lines = Vec::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("default.configured.audio.sink=") {
                        if trimmed.contains("voicegg_") || trimmed.contains("VoiceGG") {
                            if let Some(sink) = phys_sink {
                                cleaned_lines.push(format!("default.configured.audio.sink={sink}"));
                            }
                        } else {
                            cleaned_lines.push(line.to_string());
                        }
                    } else if trimmed.starts_with("default.configured.audio.source=") {
                        if trimmed.contains("voicegg_") || trimmed.contains("VoiceGG") {
                            if let Some(src) = phys_source {
                                cleaned_lines.push(format!("default.configured.audio.source={src}"));
                            }
                        } else {
                            cleaned_lines.push(line.to_string());
                        }
                    } else if trimmed.contains("voicegg_") || trimmed.contains("VoiceGG") {
                        // Drop any numbered or extra voicegg defaults (e.g. default.configured.audio.sink.0=voicegg_sink_game)
                        continue;
                    } else {
                        cleaned_lines.push(line.to_string());
                    }
                }
                let new_content = cleaned_lines.join("\n") + "\n";
                let _ = fs::write(&default_nodes_path, new_content);
            }
        }

        // 2. Scrub stream-properties
        let stream_props_path = wp_dir.join("stream-properties");
        if stream_props_path.exists() {
            if let Ok(content) = fs::read_to_string(&stream_props_path) {
                let mut cleaned_lines = Vec::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty()
                        || trimmed.starts_with('[')
                        || trimmed.starts_with('#')
                        || trimmed.starts_with(';')
                    {
                        cleaned_lines.push(line.to_string());
                        continue;
                    }

                    if let Some((key, val)) = trimmed.split_once('=') {
                        // If the key is for a VoiceGG device itself, drop it entirely
                        if key.contains("voicegg_") || key.contains("VoiceGG") {
                            continue;
                        }

                        // If the value JSON contains voicegg, scrub the "target" property
                        if val.contains("voicegg_") || val.contains("VoiceGG") {
                            if let Ok(mut json_val) = serde_json::from_str::<serde_json::Value>(val)
                            {
                                if let Some(obj) = json_val.as_object_mut() {
                                    if let Some(target) =
                                        obj.get("target").and_then(|t| t.as_str())
                                    {
                                        if target.contains("voicegg_")
                                            || target.contains("VoiceGG")
                                        {
                                            obj.remove("target");
                                        }
                                    }
                                }
                                if let Ok(compact_json) = serde_json::to_string(&json_val) {
                                    cleaned_lines.push(format!("{key}={compact_json}"));
                                    continue;
                                }
                            }
                        }

                        cleaned_lines.push(line.to_string());
                    } else {
                        cleaned_lines.push(line.to_string());
                    }
                }
                let new_content = cleaned_lines.join("\n") + "\n";
                let _ = fs::write(&stream_props_path, new_content);
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
            format!("sink_properties=media.class=Audio/Source/Virtual device.description='{desc}'")
        } else {
            format!("sink_properties=device.description='{desc}'")
        };
        let name_arg = format!("sink_name={name}");

        let output = pactl_cmd()
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

    fn load_remap_source(&mut self, master: &str, name: &str, desc: &str) -> Result<u32> {
        let master_arg = format!("master={master}");
        let name_arg = format!("source_name={name}");
        let prop_arg = format!("source_properties=device.description='{desc}'");

        let output = pactl_cmd()
            .args([
                "load-module",
                "module-remap-source",
                &master_arg,
                &name_arg,
                &prop_arg,
            ])
            .output()
            .map_err(|e| {
                PwError::Generic(format!("Failed to execute pactl load-module remap-source: {e}"))
            })?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(PwError::Generic(format!(
                "Failed to create virtual remap source '{name}': {err}"
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

    fn load_loopback(&mut self, source: &str, sink: &str, node_name: &str) -> Result<u32> {
        // SAFETY GUARD: Check for cycles before creating loopback
        if self.cycle_detector.would_cycle(source, sink) {
            return Err(PwError::Generic(format!(
                "Feedback loop prevented: linking '{source}' to '{sink}' would create an audio feedback loop"
            )));
        }

        let src_arg = format!("source={source}");
        let sink_arg = format!("sink={sink}");
        let sink_input_props = format!(
            "sink_input_properties=media.role='filter' node.passive='true' node.name='{node_name}'"
        );
        let source_output_props = format!(
            "source_output_properties=media.role='filter' node.passive='true' node.name='{node_name}'"
        );

        let output = pactl_cmd()
            .args([
                "load-module",
                "module-loopback",
                &src_arg,
                &sink_arg,
                "latency_msec=5",
                "sink_dont_move=true",
                "source_dont_move=true",
                &sink_input_props,
                &source_output_props,
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
        if self.state.original_default_sink.is_none() {
            if let Ok(out) = pactl_cmd().arg("get-default-sink").output() {
                if out.status.success() {
                    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !name.is_empty() && !name.starts_with("voicegg_") {
                        self.state.original_default_sink = Some(name.clone());
                        self.active_hardware_sink = Some(name.clone());
                        self.state.active_hardware_sink = Some(name);
                    }
                }
            }
        }

        let needs_sink_fallback = match &self.state.original_default_sink {
            None => true,
            Some(s) => s.is_empty() || s.starts_with("voicegg_"),
        };
        if needs_sink_fallback {
            if let Ok(devices) = self.list_devices() {
                if let Some(dev) = devices
                    .iter()
                    .find(|d| {
                        d.device_type == DeviceType::Sink
                            && (d.name.starts_with("alsa_output.")
                                || d.name.starts_with("bluez_output."))
                    })
                    .or_else(|| devices.iter().find(|d| d.device_type == DeviceType::Sink))
                {
                    tracing::info!("Selected physical hardware sink: {}", dev.name);
                    self.state.original_default_sink = Some(dev.name.clone());
                    self.active_hardware_sink = Some(dev.name.clone());
                    self.state.active_hardware_sink = Some(dev.name.clone());
                }
            }
        }

        // Detect physical default source
        if self.state.original_default_source.is_none() {
            if let Ok(out) = pactl_cmd().arg("get-default-source").output() {
                if out.status.success() {
                    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !name.is_empty()
                        && !name.starts_with("voicegg_")
                        && !name.ends_with(".monitor")
                    {
                        self.state.original_default_source = Some(name.clone());
                        self.active_hardware_source = Some(name.clone());
                        self.state.active_hardware_source = Some(name);
                    }
                }
            }
        }

        let needs_source_fallback = match &self.state.original_default_source {
            None => true,
            Some(s) => s.is_empty() || s.starts_with("voicegg_") || s.ends_with(".monitor"),
        };
        if needs_source_fallback {
            if let Ok(devices) = self.list_devices() {
                if let Some(dev) = devices
                    .iter()
                    .find(|d| {
                        d.device_type == DeviceType::Source
                            && (d.name.starts_with("alsa_input.")
                                || d.name.starts_with("bluez_input."))
                    })
                    .or_else(|| devices.iter().find(|d| d.device_type == DeviceType::Source))
                {
                    tracing::info!("Selected physical hardware source: {}", dev.name);
                    self.state.original_default_source = Some(dev.name.clone());
                    self.active_hardware_source = Some(dev.name.clone());
                    self.state.active_hardware_source = Some(dev.name.clone());
                }
            }
        }

        // 2. Create the virtual sinks reliably using load_null_sink
        self.load_null_sink("voicegg_sink_master", "VoiceGG Master Output", false)?;
        self.load_null_sink("voicegg_sink_game", "VoiceGG Game Audio", false)?;
        self.load_null_sink("voicegg_sink_chat", "VoiceGG Voice Chat", false)?;
        self.load_null_sink("voicegg_sink_media", "VoiceGG Media Audio", false)?;
        self.load_null_sink("voicegg_sink_aux", "VoiceGG Aux Audio", false)?;

        // Virtual Microphone: dedicated isolated playback sink + remap-source
        self.load_null_sink("voicegg_mic_sink", "VoiceGG Microphone Internal Sink", false)?;
        self.load_remap_source(
            "voicegg_mic_sink.monitor",
            "voicegg_source_mic",
            "VoiceGG Processed Microphone",
        )?;

        // Only create stream mix if streamer_mode is true
        if self.streamer_mode {
            self.load_null_sink(
                "voicegg_stream_mix_sink",
                "VoiceGG Stream Mix Internal Sink",
                false,
            )?;
            let mod_id = self.load_remap_source(
                "voicegg_stream_mix_sink.monitor",
                "voicegg_source_stream_mix",
                "VoiceGG Stream Mix",
            )?;
            self.state.stream_mix_module = Some(mod_id);
        }

        // 3. Link voicegg_sink_master.monitor to active hardware sink with loopback
        let target_sink = self
            .active_hardware_sink
            .as_ref()
            .or(self.state.original_default_sink.as_ref())
            .cloned();

        if let Some(sink) = target_sink {
            tracing::info!("Connecting VoiceGG Master to physical sink '{}'", sink);
            if let Ok(id) = self.load_loopback(
                "voicegg_sink_master.monitor",
                &sink,
                "voicegg_loopback_master",
            ) {
                self.state.master_output_module = Some(id);
            }
        }

        // 4. Initialize dsp_engine and spawn stereo DSP workers for playback channels
        self.dsp_engine = DspEngine::new();
        for ch in [
            ChannelId::Game,
            ChannelId::Chat,
            ChannelId::Media,
            ChannelId::Aux,
        ] {
            let src = format!("{}.monitor", ch.pipewire_node_name());
            self.dsp_engine
                .spawn_playback_worker(ch, &src, "voicegg_sink_master");
        }

        // 5. Spawn mic capture DSP worker directly to isolated voicegg_mic_sink with verified physical source
        let physical_source = self
            .active_hardware_source
            .as_ref()
            .or(self.state.original_default_source.as_ref())
            .cloned();

        self.dsp_engine.spawn_mic_worker(
            physical_source.as_deref(),
            Some("voicegg_mic_sink"),
        );

        // 6. Set VoiceGG as system default sink & source for instant routing
        let _ = pactl_cmd().args(["set-default-sink", "voicegg_sink_game"]).output();
        let _ = pactl_cmd().args(["set-default-source", "voicegg_source_mic"]).output();

        tracing::info!("VoiceGG virtual devices created reliably. System defaults updated to VoiceGG.");
        Ok(())
    }

    /// Destroys all VoiceGG virtual devices and restores original default audio devices.
    pub fn teardown_virtual_devices(&mut self) -> Result<()> {
        tracing::info!("Tearing down VoiceGG virtual devices...");

        // 1. Stop all DSP engine workers first so parec/pacat streams close
        self.dsp_engine.stop_all();

        // 2. Clear filter chains
        self.filter_chains.clear();

        // 3. Run unified restoration pipeline
        let pref_sink = self
            .active_hardware_sink
            .as_deref()
            .or(self.state.original_default_sink.as_deref());
        let pref_source = self
            .active_hardware_source
            .as_deref()
            .or(self.state.original_default_source.as_deref());

        Self::restore_and_unload_all(pref_sink, pref_source);

        self.state.loaded_modules.clear();
        self.state.master_output_module = None;
        self.state.mic_input_module = None;
        self.state.mic_monitor_module = None;
        self.state.stream_mix_module = None;
        self.cycle_detector.clear();

        tracing::info!("VoiceGG teardown complete. System audio restored to original state.");
        Ok(())
    }

    /// Discovers physical hardware audio devices (sinks and sources), filtering out VoiceGG virtual devices.
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        let mut devices = Vec::new();

        let default_sink = self
            .active_hardware_sink
            .clone()
            .or_else(|| self.state.original_default_sink.clone())
            .or_else(|| {
                pactl_cmd().arg("get-default-sink").output().ok().and_then(|out| {
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

        let default_source = self
            .active_hardware_source
            .clone()
            .or_else(|| self.state.original_default_source.clone())
            .or_else(|| {
                pactl_cmd().arg("get-default-source").output().ok().and_then(|out| {
                    if out.status.success() {
                        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !name.is_empty()
                            && !name.starts_with("voicegg_")
                            && !name.ends_with(".monitor")
                        {
                            Some(name)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            });

        // Output Sinks
        if let Ok(output) = pactl_cmd().args(["list", "sinks"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            devices.extend(Self::parse_pactl_devices(
                &text,
                DeviceType::Sink,
                default_sink.as_deref(),
            ));
        }

        // Input Sources
        if let Ok(output) = pactl_cmd().args(["list", "sources"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            devices.extend(Self::parse_pactl_devices(
                &text,
                DeviceType::Source,
                default_source.as_deref(),
            ));
        }

        Ok(devices)
    }

    /// Parses pactl device list text, extracting genuine IDs and attributes.
    pub(crate) fn parse_pactl_devices(
        text: &str,
        device_type: DeviceType,
        default_dev_name: Option<&str>,
    ) -> Vec<AudioDevice> {
        let mut devices = Vec::new();
        let prefix = match device_type {
            DeviceType::Sink => "Sink #",
            DeviceType::Source => "Source #",
        };

        let mut current_id: Option<u32> = None;
        let mut current_name = String::new();
        let mut current_desc = String::new();

        let finalize_device = |devices: &mut Vec<AudioDevice>,
                               id: u32,
                               name: String,
                               mut desc: String| {
            if !name.is_empty() && !name.starts_with("voicegg_") && !name.ends_with(".monitor") {
                if desc.is_empty() {
                    desc = name.clone();
                }
                let is_default = default_dev_name.map_or(false, |d| d == name);
                devices.push(AudioDevice {
                    id,
                    name,
                    description: desc,
                    device_type,
                    is_default,
                });
            }
        };

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(prefix) {
                if let Some(id) = current_id {
                    finalize_device(
                        &mut devices,
                        id,
                        std::mem::take(&mut current_name),
                        std::mem::take(&mut current_desc),
                    );
                }
                let num_str = trimmed.trim_start_matches(prefix).trim();
                current_id = num_str.parse().ok();
            } else if current_id.is_some() {
                if trimmed.starts_with("Name: ") {
                    current_name = trimmed.trim_start_matches("Name: ").trim().to_string();
                } else if trimmed.starts_with("Description: ") {
                    current_desc = trimmed
                        .trim_start_matches("Description: ")
                        .trim()
                        .to_string();
                }
            }
        }

        if let Some(id) = current_id {
            finalize_device(&mut devices, id, current_name, current_desc);
        }

        devices
    }

    fn get_sink_id_to_name_map() -> std::collections::HashMap<u32, String> {
        let mut map = std::collections::HashMap::new();
        if let Ok(output) = pactl_cmd().args(["list", "sinks", "short"]).output() {
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

    fn get_source_id_to_name_map() -> std::collections::HashMap<u32, String> {
        let mut map = std::collections::HashMap::new();
        if let Ok(output) = pactl_cmd().args(["list", "sources", "short"]).output() {
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
        let output = pactl_cmd()
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

            // Exclude internal VoiceGG infrastructure, loopback streams, and audio DSP workers
            let app_lower = app_name.to_ascii_lowercase();
            let bin_lower = binary_name.to_ascii_lowercase();
            let media_lower = media_name.to_ascii_lowercase();
            let node_lower = node_name.to_ascii_lowercase();
            let is_internal = node_lower.contains("loopback")
                || media_lower.contains("loopback")
                || app_lower.contains("loopback")
                || bin_lower.contains("loopback")
                || bin_lower.contains("pacat")
                || bin_lower.contains("parec")
                || app_lower.contains("pacat")
                || app_lower.contains("parec")
                || media_lower.contains("pacat")
                || media_lower.contains("parec")
                || app_lower.contains("voicegg")
                || bin_lower.contains("voicegg")
                || media_lower.contains("voicegg")
                || node_lower.contains("voicegg")
                || block.contains("media.role = \"filter\"")
                || block.contains("node.link-group = \"loopback-");

            if is_internal {
                continue;
            }

            // Fallbacks for empty or generic names
            if app_name.is_empty() {
                if !node_name.is_empty() {
                    app_name = node_name.clone();
                } else if !media_name.is_empty() {
                    app_name = media_name.clone();
                } else if !binary_name.is_empty() {
                    app_name = binary_name.clone();
                }
            }
            let bin_is_generic = binary_name.is_empty()
                || binary_name.eq_ignore_ascii_case("unknown")
                || binary_name.eq_ignore_ascii_case("electron")
                || binary_name.eq_ignore_ascii_case("wine64-preloader")
                || binary_name.eq_ignore_ascii_case("wine-preloader");
            if bin_is_generic {
                if !app_name.is_empty()
                    && !app_name.eq_ignore_ascii_case("AudioStream")
                    && !app_name.eq_ignore_ascii_case("Playback")
                {
                    binary_name = app_name.clone();
                } else if !node_name.is_empty() {
                    binary_name = node_name.clone();
                }
            }

            // Recognize Discord
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

            // Recognize Chrome / Chromium
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
        let output = pactl_cmd()
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

    /// Routes any active application matching the executable binary name, application name, or stream ID to the specified channel.
    pub fn route_app_by_name(&self, binary: &str, target: ChannelId) -> Result<()> {
        let streams = self.list_active_streams()?;
        for stream in streams {
            let is_match = stream.binary_name.eq_ignore_ascii_case(binary)
                || stream.app_name.eq_ignore_ascii_case(binary)
                || stream.id.to_string() == binary
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
        let output = pactl_cmd()
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

    /// Sets the volume for all active streams matching the binary name, application name, or stream ID.
    pub fn set_app_volume_by_name(&self, binary: &str, volume: u8) -> Result<()> {
        if let Ok(streams) = self.list_active_streams() {
            for stream in streams {
                if stream.binary_name.eq_ignore_ascii_case(binary)
                    || stream.app_name.eq_ignore_ascii_case(binary)
                    || stream.id.to_string() == binary
                {
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
        let _ = pactl_cmd().args([cmd, node_name, &vol_str]).output();
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
        let _ = pactl_cmd().args([cmd, node_name, mute_str]).output();
        Ok(())
    }

    /// Changes the hardware output sink that VoiceGG Master forwards to.
    /// Strictly validates that the target is connected and never overwrites `original_default_sink`.
    pub fn set_output_sink(&mut self, sink_name: &str) -> Result<()> {
        let target = if sink_name == "default" || sink_name.is_empty() {
            self.state
                .original_default_sink
                .clone()
                .unwrap_or_default()
        } else {
            sink_name.to_string()
        };

        if target.is_empty() || target.starts_with("voicegg_") || target.ends_with(".monitor") {
            return Err(PwError::Generic(format!(
                "Invalid target output sink: '{target}'"
            )));
        }

        // Strict hardware validation: verify target exists in list_devices()
        let devices = self.list_devices().unwrap_or_default();
        let exists = devices
            .iter()
            .any(|d| d.device_type == DeviceType::Sink && d.name == target);
        if !exists {
            tracing::warn!(
                "Target output sink '{}' not found in connected devices; keeping active device",
                target
            );
            return Err(PwError::Generic(format!(
                "Device '{target}' is not currently connected"
            )));
        }

        if let Some(ref current) = self.active_hardware_sink {
            if current == &target && self.state.master_output_module.is_some() {
                return Ok(());
            }
        }

        if let Some(mod_id) = self.state.master_output_module.take() {
            let _ = pactl_cmd()
                .args(["unload-module", &mod_id.to_string()])
                .output();
            self.state.loaded_modules.retain(|&id| id != mod_id);
        }

        tracing::info!("Switching VoiceGG Master output to '{}'", target);
        let mod_id = self.load_loopback(
            "voicegg_sink_master.monitor",
            &target,
            "voicegg_loopback_master",
        )?;
        self.state.master_output_module = Some(mod_id);
        self.active_hardware_sink = Some(target.clone());
        self.state.active_hardware_sink = Some(target);
        self.save_state();

        // Re-route mic sidetone loopback automatically if mic monitor is currently active
        if self.state.mic_monitor_module.is_some() {
            let vol = self.mic_monitor_volume;
            let _ = self.set_mic_monitor(true, vol);
        }

        Ok(())
    }

    /// Changes the hardware input microphone source feeding into VoiceGG Mic.
    /// Strictly validates that the target is connected and never overwrites `original_default_source`.
    pub fn set_input_source(&mut self, source_name: &str) -> Result<()> {
        let target = if source_name == "default" || source_name.is_empty() {
            self.state
                .original_default_source
                .clone()
                .unwrap_or_default()
        } else {
            source_name.to_string()
        };

        if target.is_empty()
            || target.ends_with(".monitor")
            || target.starts_with("alsa_output")
            || target.starts_with("voicegg_")
        {
            return Err(PwError::Generic(format!(
                "Cannot use '{target}' as microphone source: feedback safety protection."
            )));
        }

        // Strict hardware validation: verify target exists in list_devices()
        let devices = self.list_devices().unwrap_or_default();
        let exists = devices
            .iter()
            .any(|d| d.device_type == DeviceType::Source && d.name == target);
        if !exists {
            tracing::warn!(
                "Target microphone source '{}' not found in connected devices; keeping active source",
                target
            );
            return Err(PwError::Generic(format!(
                "Device '{target}' is not currently connected"
            )));
        }

        self.active_hardware_source = Some(target.clone());
        self.state.active_hardware_source = Some(target.clone());
        tracing::info!("Switching VoiceGG Mic input to '{}'", target);
        self.dsp_engine
            .spawn_mic_worker(Some(&target), Some("voicegg_mic_sink"));
        self.save_state();
        Ok(())
    }

    /// Enables or disables real-time microphone sidetone monitoring with feedback protection.
    /// When already enabled and volume changes, dynamically adjusts volume on the existing sink-input
    /// via `pactl set-sink-input-volume` to prevent clicks, pops, and audio dropouts.
    pub fn set_mic_monitor(&mut self, enabled: bool, volume: u8) -> Result<()> {
        let clamped_vol = volume.min(100);
        self.mic_monitor_volume = clamped_vol;

        if enabled {
            let output_sink = match self
                .active_hardware_sink
                .as_ref()
                .or(self.state.original_default_sink.as_ref())
            {
                Some(s) if !s.is_empty() => s.clone(),
                _ => {
                    return Err(PwError::Generic(
                        "No physical output sink available for microphone monitoring".into(),
                    ));
                }
            };

            // Feedback guard: physical output sink cannot match physical input mic
            if let Some(ref input_mic) = self
                .active_hardware_source
                .as_ref()
                .or(self.state.original_default_source.as_ref())
            {
                if &output_sink == *input_mic {
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

            // If mic monitor loopback is already active, adjust volume dynamically on sink-input!
            if let Some(mod_id) = self.state.mic_monitor_module {
                if let Some(input_id) =
                    Self::find_loopback_sink_input_id(mod_id, "voicegg_loopback_mic_monitor")
                {
                    let vol_str = format!("{clamped_vol}%");
                    let res = pactl_cmd()
                        .args(["set-sink-input-volume", &input_id.to_string(), &vol_str])
                        .output();
                    if let Ok(out) = res {
                        if out.status.success() {
                            return Ok(());
                        }
                    }
                }
            }

            // If not already active or if dynamic volume update failed, reload loopback
            if let Some(old_id) = self.state.mic_monitor_module.take() {
                let _ = pactl_cmd().args(["unload-module", &old_id.to_string()]).output();
                self.state.loaded_modules.retain(|&id| id != old_id);
            }

            let mod_id = self.load_loopback(
                "voicegg_source_mic",
                &output_sink,
                "voicegg_loopback_mic_monitor",
            )?;

            // Set initial volume on the newly created sink-input
            if let Some(input_id) =
                Self::find_loopback_sink_input_id(mod_id, "voicegg_loopback_mic_monitor")
            {
                let vol_str = format!("{clamped_vol}%");
                let _ = pactl_cmd()
                    .args(["set-sink-input-volume", &input_id.to_string(), &vol_str])
                    .output();
            }

            self.state.mic_monitor_module = Some(mod_id);
            self.save_state();
            Ok(())
        } else {
            if let Some(old_id) = self.state.mic_monitor_module.take() {
                let _ = pactl_cmd().args(["unload-module", &old_id.to_string()]).output();
                self.state.loaded_modules.retain(|&id| id != old_id);
            }
            self.save_state();
            Ok(())
        }
    }

    /// Finds the sink input ID corresponding to a loaded loopback module ID or node name.
    fn find_loopback_sink_input_id(module_id: u32, node_name: &str) -> Option<u32> {
        let output = pactl_cmd().args(["list", "sink-inputs"]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mod_marker = format!("Owner Module: {module_id}");
        let node_marker = format!("node.name = \"{node_name}\"");

        for block in text.split("Sink Input #") {
            if block.trim().is_empty() {
                continue;
            }
            if block.contains(&mod_marker) || block.contains(&node_marker) {
                if let Some(first_line) = block.lines().next() {
                    if let Ok(id) = first_line.trim().parse::<u32>() {
                        return Some(id);
                    }
                }
            }
        }
        None
    }

    /// Dynamically enables or disables streamer mode.
    /// When enabled, creates the `voicegg_source_stream_mix` virtual source.
    /// When disabled, unloads the module immediately so it does not clutter the system.
    pub fn set_streamer_mode(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            if self.state.stream_mix_module.is_none() {
                let _ = self.load_null_sink(
                    "voicegg_stream_mix_sink",
                    "VoiceGG Stream Mix Internal Sink",
                    false,
                )?;
                let mod_id = self.load_remap_source(
                    "voicegg_stream_mix_sink.monitor",
                    "voicegg_source_stream_mix",
                    "VoiceGG Stream Mix",
                )?;
                self.state.stream_mix_module = Some(mod_id);
                self.save_state();
            }
        } else {
            if let Some(mod_id) = self.state.stream_mix_module.take() {
                let _ = pactl_cmd().args(["unload-module", &mod_id.to_string()]).output();
                self.state.loaded_modules.retain(|&id| id != mod_id);
            }
            if let Ok(output) = pactl_cmd().args(["list", "modules", "short"]).output() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    if line.contains("voicegg_stream_mix") {
                        if let Some(id_str) = line.split('\t').next() {
                            if let Ok(id) = id_str.trim().parse::<u32>() {
                                let _ = pactl_cmd().args(["unload-module", &id.to_string()]).output();
                                self.state.loaded_modules.retain(|&m_id| m_id != id);
                            }
                        }
                    }
                }
            }
            self.save_state();
        }
        self.streamer_mode = enabled;
        Ok(())
    }

    /// Returns detected DSP capabilities.
    #[must_use]
    pub fn get_capabilities(&self) -> PluginCapabilities {
        self.capabilities
    }

    /// Samples peak and RMS meter levels (in dBFS) across channels in O(1) memory
    /// directly from `DspEngine` with zero subprocess calls.
    #[must_use]
    pub fn sample_meters(&self) -> std::collections::HashMap<ChannelId, (f32, f32)> {
        let mut meters = std::collections::HashMap::new();

        let engine_meters = self.dsp_engine.sample_meters();
        for (ch, meter) in engine_meters {
            meters.insert(ch, (meter.peak_dbfs, meter.rms_dbfs));
        }

        for channel in [
            ChannelId::Master,
            ChannelId::Game,
            ChannelId::Chat,
            ChannelId::Media,
            ChannelId::Aux,
            ChannelId::Mic,
            ChannelId::StreamMix,
        ] {
            meters.entry(channel).or_insert((-100.0, -100.0));
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

    #[test]
    fn test_parse_pactl_devices_multi_device_ids_and_defaults() {
        let sample = r#"
Sink #0
	State: SUSPENDED
	Name: alsa_output.pci-0000_00_1f.3.analog-stereo
	Description: Built-in Audio Analog Stereo
	Driver: PipeWire

Sink #15
	State: RUNNING
	Name: alsa_output.usb-fifine_Microphone-00.analog-stereo
	Description: Fifine USB Headset
	Driver: PipeWire

Sink #22
	State: IDLE
	Name: voicegg_sink_game
	Description: VoiceGG Game Audio
	Driver: PipeWire

Sink #35
	State: IDLE
	Name: alsa_output.pci-0000_01_00.1.hdmi-stereo
	Description: HDMI Audio
	Driver: PipeWire
"#;
        let devices = GraphManager::parse_pactl_devices(
            sample,
            DeviceType::Sink,
            Some("alsa_output.usb-fifine_Microphone-00.analog-stereo"),
        );

        assert_eq!(devices.len(), 3); // voicegg_sink_game must be filtered out!

        let dev0 = &devices[0];
        assert_eq!(dev0.id, 0);
        assert_eq!(dev0.name, "alsa_output.pci-0000_00_1f.3.analog-stereo");
        assert_eq!(dev0.description, "Built-in Audio Analog Stereo");
        assert!(!dev0.is_default);

        let dev1 = &devices[1];
        assert_eq!(dev1.id, 15);
        assert_eq!(dev1.name, "alsa_output.usb-fifine_Microphone-00.analog-stereo");
        assert_eq!(dev1.description, "Fifine USB Headset");
        assert!(dev1.is_default); // Matches default device name!

        let dev2 = &devices[2];
        assert_eq!(dev2.id, 35);
        assert_eq!(dev2.name, "alsa_output.pci-0000_01_00.1.hdmi-stereo");
        assert_eq!(dev2.description, "HDMI Audio");
        assert!(!dev2.is_default);
    }

    #[test]
    fn test_scrub_wireplumber_state_in_temp_dir() {
        let temp_dir = std::env::temp_dir().join(format!("voicegg_test_wp_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        // 1. Create mock default-nodes
        let default_nodes = r#"[default-nodes]
default.configured.audio.sink=voicegg_sink_game
default.configured.audio.sink.0=voicegg_sink_game
default.configured.audio.sink.1=alsa_output.pci-0000_00_1f.3.analog-stereo
default.configured.audio.source=voicegg_source_mic
default.configured.audio.source.0=voicegg_source_mic
default.configured.audio.source.1=alsa_input.usb-fifine-00.analog-stereo
"#;
        let dn_path = temp_dir.join("default-nodes");
        fs::write(&dn_path, default_nodes).unwrap();

        // 2. Create mock stream-properties
        let stream_props = r#"[stream-properties]
Output/Audio:application.name:Brave={"channelVolumes":[1.0, 1.0], "target":"voicegg_sink_media", "mute":false}
Output/Audio:application.name:Spotify={"volume":1.0, "target":"voicegg_sink_game"}
Audio/Sink:node.name:voicegg_sink_master={"volume":1.0, "channelMap":["FL", "FR"]}
Audio/Source/Virtual:node.name:voicegg_source_mic={"volume":1.0}
Output/Audio:application.name:Discord={"channelVolumes":[0.8, 0.8], "target":"alsa_output.pci-0000_00_1f.3.analog-stereo", "mute":false}
"#;
        let sp_path = temp_dir.join("stream-properties");
        fs::write(&sp_path, stream_props).unwrap();

        // Run scrubbing
        GraphManager::scrub_wireplumber_in_dir(
            &temp_dir,
            Some("alsa_output.pci-0000_00_1f.3.analog-stereo"),
            Some("alsa_input.usb-fifine-00.analog-stereo"),
        );

        // Verify default-nodes
        let scrubbed_dn = fs::read_to_string(&dn_path).unwrap();
        assert!(scrubbed_dn.contains("default.configured.audio.sink=alsa_output.pci-0000_00_1f.3.analog-stereo"));
        assert!(scrubbed_dn.contains("default.configured.audio.source=alsa_input.usb-fifine-00.analog-stereo"));
        assert!(!scrubbed_dn.contains("voicegg_sink_game"));
        assert!(!scrubbed_dn.contains("voicegg_source_mic"));

        // Verify stream-properties
        let scrubbed_sp = fs::read_to_string(&sp_path).unwrap();
        assert!(!scrubbed_sp.contains("voicegg_sink_master"));
        assert!(!scrubbed_sp.contains("voicegg_source_mic"));
        assert!(!scrubbed_sp.contains("voicegg_sink_media"));
        assert!(!scrubbed_sp.contains("voicegg_sink_game"));
        // Brave and Spotify should still exist but without target
        assert!(scrubbed_sp.contains("application.name:Brave"));
        assert!(scrubbed_sp.contains("application.name:Spotify"));
        // Discord's physical target must remain untouched
        assert!(scrubbed_sp.contains("target\":\"alsa_output.pci-0000_00_1f.3.analog-stereo"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_device_separation_and_state() {
        let mut gm = GraphManager::new();
        gm.state.original_default_sink = Some("phys_sink_orig".to_string());
        gm.state.original_default_source = Some("phys_source_orig".to_string());
        gm.active_hardware_sink = Some("phys_sink_active".to_string());
        gm.active_hardware_source = Some("phys_source_active".to_string());

        assert_eq!(gm.original_default_sink(), Some("phys_sink_orig"));
        assert_eq!(gm.original_default_source(), Some("phys_source_orig"));
        assert_eq!(gm.active_output_sink(), Some("phys_sink_active"));
        assert_eq!(gm.active_input_source(), Some("phys_source_active"));
    }

    #[test]
    fn test_unplugged_device_validation_rejects_invalid() {
        let mut gm = GraphManager::new();
        // Target starting with voicegg_ must be rejected
        assert!(gm.set_output_sink("voicegg_sink_game").is_err());
        assert!(gm.set_input_source("voicegg_source_mic").is_err());
        // Target ending with .monitor must be rejected
        assert!(gm.set_output_sink("alsa_output.pci.monitor").is_err());
        assert!(gm.set_input_source("alsa_input.pci.monitor").is_err());
        // Non-existent target must be rejected
        assert!(gm.set_output_sink("non_existent_unplugged_dac_12345").is_err());
        assert!(gm.set_input_source("non_existent_unplugged_mic_12345").is_err());
    }

    #[test]
    fn test_streamer_mode_state() {
        let mut gm = GraphManager::new();
        assert!(!gm.is_streamer_mode());
        gm.streamer_mode = true;
        assert!(gm.is_streamer_mode());
    }
}
