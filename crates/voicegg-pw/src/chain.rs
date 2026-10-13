//! PipeWire filter-chain configuration generation, process lifecycle management,
//! plugin capability detection, and runtime parameter modulation.

use crate::error::{PwError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use voicegg_core::{ChannelId, PluginCapabilities, Preset};

/// Standard search directories for audio plugins (LADSPA and LV2).
const PLUGIN_SEARCH_DIRS: &[&str] = &[
    "/usr/lib/ladspa",
    "/usr/lib64/ladspa",
    "/usr/local/lib/ladspa",
    "/usr/lib/lv2",
    "/usr/local/lib/lv2",
];

/// Scans standard system and user directories to discover available audio DSP plugins.
#[must_use]
pub fn detect_capabilities() -> PluginCapabilities {
    let mut rnnoise = false;
    let mut gate = false;
    let mut compressor = false;

    // Scan standard system paths
    for &dir_str in PLUGIN_SEARCH_DIRS {
        let path = Path::new(dir_str);
        if path.is_dir() {
            scan_plugin_directory(path, &mut rnnoise, &mut gate, &mut compressor, 0);
        }
    }

    // Scan user directories ~/.ladspa and ~/.lv2
    if let Some(home) = dirs::home_dir() {
        let user_ladspa = home.join(".ladspa");
        if user_ladspa.is_dir() {
            scan_plugin_directory(&user_ladspa, &mut rnnoise, &mut gate, &mut compressor, 0);
        }
        let user_lv2 = home.join(".lv2");
        if user_lv2.is_dir() {
            scan_plugin_directory(&user_lv2, &mut rnnoise, &mut gate, &mut compressor, 0);
        }
    }

    // Scan environment variable overrides if present
    if let Ok(ladspa_path) = std::env::var("LADSPA_PATH") {
        for p in ladspa_path.split(':') {
            let path = Path::new(p);
            if path.is_dir() {
                scan_plugin_directory(path, &mut rnnoise, &mut gate, &mut compressor, 0);
            }
        }
    }
    if let Ok(lv2_path) = std::env::var("LV2_PATH") {
        for p in lv2_path.split(':') {
            let path = Path::new(p);
            if path.is_dir() {
                scan_plugin_directory(path, &mut rnnoise, &mut gate, &mut compressor, 0);
            }
        }
    }

    PluginCapabilities {
        rnnoise,
        gate,
        compressor,
        biquad_eq: true, // Builtin in PipeWire filter-chain
    }
}

fn scan_plugin_directory(
    dir: &Path,
    rnnoise: &mut bool,
    gate: &mut bool,
    compressor: &mut bool,
    depth: usize,
) {
    if depth > 4 {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy().to_ascii_lowercase();

        if name_str.contains("librnnoise_ladspa")
            || name_str.contains("noise_suppressor_mono")
            || name_str.contains("rnnoise")
        {
            *rnnoise = true;
        }

        if name_str.contains("lsp-plugins") || name_str.contains("lsp_plugins") {
            *gate = true;
            *compressor = true;
        } else {
            if name_str.contains("gate") || name_str.contains("noisegate") {
                *gate = true;
            }
            if name_str.contains("compressor") || name_str.contains("sc4") {
                *compressor = true;
            }
        }

        if path.is_dir() {
            scan_plugin_directory(&path, rnnoise, gate, compressor, depth + 1);
        }
    }
}

/// Default frequencies for a 10-band equalizer.
pub const DEFAULT_EQ_FREQS: [f32; 10] = [
    31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Generates a PipeWire filter-chain `.conf` configuration string for the Microphone channel.
#[must_use]
pub fn generate_mic_chain_conf(preset: &Preset, capabilities: &PluginCapabilities) -> String {
    let mut nodes = Vec::new();
    let mut node_names: Vec<String> = Vec::new();

    // 1. Highpass (80 Hz): builtin bq_highpass
    nodes.push(
        r#"                    {
                        type    = builtin
                        name    = "hp_80"
                        label   = bq_highpass
                        control = { "Freq" = 80.0 "Q" = 0.707 }
                    }"#
        .to_string(),
    );
    node_names.push("hp_80".to_string());

    // 2. RNNoise (if capabilities.rnnoise && noise_canceller enabled)
    let nc_enabled = preset.noise_canceller.clone().unwrap_or_default().enabled;
    if capabilities.rnnoise && nc_enabled {
        let vad_val = preset
            .noise_canceller
            .as_ref()
            .map_or(50.0, |nc| (nc.vad_threshold * 100.0).clamp(0.0, 100.0));
        nodes.push(format!(
            r#"                    {{
                        type    = ladspa
                        name    = "rnnoise"
                        plugin  = "ladspa/librnnoise_ladspa"
                        label   = "noise_suppressor_mono"
                        control = {{ "VAD Threshold (%)" = {:.1} }}
                    }}"#,
            vad_val
        ));
        node_names.push("rnnoise".to_string());
    }

    // 3. Gate (if available & enabled)
    let gate_enabled = preset.noise_gate.as_ref().is_some_and(|g| g.enabled);
    if capabilities.gate && gate_enabled {
        let (open_thresh, close_thresh, attack, hold, release) =
            if let Some(ref g) = preset.noise_gate {
                (
                    g.threshold_db,
                    g.threshold_db - 5.0,
                    (g.attack_ms / 1000.0).max(0.001),
                    (g.hold_ms / 1000.0).max(0.001),
                    (g.release_ms / 1000.0).max(0.001),
                )
            } else {
                (-45.0, -50.0, 0.002, 0.050, 0.080)
            };
        nodes.push(format!(
            r#"                    {{
                        type    = builtin
                        name    = "gate"
                        label   = noisegate
                        control = {{
                            "Open Threshold"  = {:.1}
                            "Close Threshold" = {:.1}
                            "Attack (s)"       = {:.4}
                            "Hold (s)"         = {:.4}
                            "Release (s)"      = {:.4}
                        }}
                    }}"#,
            open_thresh, close_thresh, attack, hold, release
        ));
        node_names.push("gate".to_string());
    }

    // 4. 10-band EQ: 10 builtin biquads linked sequentially
    for (i, &default_freq) in DEFAULT_EQ_FREQS.iter().enumerate() {
        let name = format!("eq_band_{i}");
        let label = if i == 0 {
            "bq_lowshelf"
        } else if i == 9 {
            "bq_highshelf"
        } else {
            "bq_peaking"
        };

        let (freq, q, gain) = if let Some(band) = preset.eq.bands.get(i) {
            let g = if preset.eq.enabled && band.enabled {
                band.gain_db
            } else {
                0.0
            };
            (band.freq_hz, band.q, g)
        } else {
            (default_freq, 1.0, 0.0)
        };

        nodes.push(format!(
            r#"                    {{
                        type    = builtin
                        name    = "{name}"
                        label   = {label}
                        control = {{ "Freq" = {:.1} "Q" = {:.3} "Gain" = {:.2} }}
                    }}"#,
            freq, q, gain
        ));
        node_names.push(name);
    }

    // 5. Compressor (if available & enabled)
    let comp_enabled = preset.compressor.as_ref().is_some_and(|c| c.enabled);
    if capabilities.compressor && comp_enabled {
        nodes.push(
            r#"                    {
                        type    = builtin
                        name    = "compressor"
                        label   = clamp
                        control = { "Min" = -1.0 "Max" = 1.0 }
                    }"#
            .to_string(),
        );
        node_names.push("compressor".to_string());
    }

    // 6. Limiter / output clamp
    let ceiling = preset.limiter.as_ref().map_or(1.0, |l| {
        if l.enabled {
            10.0_f32.powf(l.ceiling_db / 20.0).clamp(0.01, 1.0)
        } else {
            1.0
        }
    });
    nodes.push(format!(
        r#"                    {{
                        type    = builtin
                        name    = "limiter"
                        label   = clamp
                        control = {{ "Min" = -{:.4} "Max" = {:.4} }}
                    }}"#,
        ceiling, ceiling
    ));
    node_names.push("limiter".to_string());

    // Generate links sequentially
    let mut links = Vec::new();
    for window in node_names.windows(2) {
        let from_name = &window[0];
        let to_name = &window[1];
        let out_port = if from_name == "rnnoise" {
            "Output"
        } else {
            "Out"
        };
        let in_port = if to_name == "rnnoise" { "Input" } else { "In" };
        links.push(format!(
            r#"                    {{ output = "{from_name}:{out_port}" input = "{to_name}:{in_port}" }}"#
        ));
    }

    let nodes_str = nodes.join("\n");
    let links_str = links.join("\n");

    format!(
        r#"context.modules = [
    {{ name = libpipewire-module-filter-chain
        flags = [ nofail ]
        args = {{
            node.description = "VoiceGG Microphone DSP"
            media.name       = "VoiceGG Microphone DSP"
            filter.graph = {{
                nodes = [
{nodes_str}
                ]
                links = [
{links_str}
                ]
            }}
            audio.channels = 1
            audio.position = [ MONO ]
            capture.props = {{
                node.name    = "voicegg_mic_filter_in"
                node.passive = true
            }}
            playback.props = {{
                node.name   = "voicegg_source_mic"
                media.class = Audio/Source
            }}
        }}
    }}
]
"#
    )
}

/// Generates a PipeWire filter-chain `.conf` configuration string for a playback channel (Game, Chat, Media, Aux).
#[must_use]
pub fn generate_channel_chain_conf(
    channel: ChannelId,
    preset: &Preset,
    _capabilities: &PluginCapabilities,
) -> String {
    let mut nodes = Vec::new();
    let mut node_names: Vec<String> = Vec::new();

    // 10-band EQ biquads
    for (i, &default_freq) in DEFAULT_EQ_FREQS.iter().enumerate() {
        let name = format!("eq_band_{i}");
        let label = if i == 0 {
            "bq_lowshelf"
        } else if i == 9 {
            "bq_highshelf"
        } else {
            "bq_peaking"
        };

        let (freq, q, gain) = if let Some(band) = preset.eq.bands.get(i) {
            let g = if preset.eq.enabled && band.enabled {
                band.gain_db
            } else {
                0.0
            };
            (band.freq_hz, band.q, g)
        } else {
            (default_freq, 1.0, 0.0)
        };

        nodes.push(format!(
            r#"                    {{
                        type    = builtin
                        name    = "{name}"
                        label   = {label}
                        control = {{ "Freq" = {:.1} "Q" = {:.3} "Gain" = {:.2} }}
                    }}"#,
            freq, q, gain
        ));
        node_names.push(name);
    }

    // Limiter
    let ceiling = preset.limiter.as_ref().map_or(1.0, |l| {
        if l.enabled {
            10.0_f32.powf(l.ceiling_db / 20.0).clamp(0.01, 1.0)
        } else {
            1.0
        }
    });
    nodes.push(format!(
        r#"                    {{
                        type    = builtin
                        name    = "limiter"
                        label   = clamp
                        control = {{ "Min" = -{:.4} "Max" = {:.4} }}
                    }}"#,
        ceiling, ceiling
    ));
    node_names.push("limiter".to_string());

    // Links between sequential bands and limiter
    let mut links = Vec::new();
    for window in node_names.windows(2) {
        let from_name = &window[0];
        let to_name = &window[1];
        links.push(format!(
            r#"                    {{ output = "{from_name}:Out" input = "{to_name}:In" }}"#
        ));
    }

    let nodes_str = nodes.join("\n");
    let links_str = links.join("\n");
    let sink_name = channel.pipewire_node_name();
    let sink_desc = channel.pipewire_description();

    format!(
        r#"context.modules = [
    {{ name = libpipewire-module-filter-chain
        flags = [ nofail ]
        args = {{
            node.description = "{sink_desc}"
            media.name       = "{sink_desc}"
            filter.graph = {{
                nodes = [
{nodes_str}
                ]
                links = [
{links_str}
                ]
            }}
            audio.channels = 2
            audio.position = [ FL FR ]
            capture.props = {{
                node.name   = "{sink_name}"
                media.class = Audio/Sink
            }}
            playback.props = {{
                node.passive  = true
                target.object = "voicegg_sink_master"
            }}
        }}
    }}
]
"#
    )
}

/// Finds the PipeWire node ID for a given node name using `pw-dump` or `pw-cli`.
pub fn find_node_id_by_name(target_name: &str) -> Result<u32> {
    if let Ok(output) = Command::new("pw-dump").arg("Node").output() {
        if output.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(nodes) = val.as_array() {
                    for node in nodes {
                        if let Some(info) = node.get("info") {
                            if let Some(props) = info.get("props") {
                                if let Some(name) = props.get("node.name").and_then(|n| n.as_str())
                                {
                                    if name == target_name {
                                        if let Some(id) = node.get("id").and_then(|i| i.as_u64()) {
                                            return Ok(id as u32);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: parse `pw-cli ls Node`
    if let Ok(output) = Command::new("pw-cli").args(["ls", "Node"]).output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut current_id = None;
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("id ") {
                    if let Some(id_part) = trimmed.strip_prefix("id ") {
                        let id_str = id_part.split(',').next().unwrap_or("").trim();
                        current_id = id_str.parse::<u32>().ok();
                    }
                } else if trimmed.starts_with("node.name = \"") {
                    let name = trimmed
                        .trim_start_matches("node.name = \"")
                        .trim_end_matches('"');
                    if name == target_name {
                        if let Some(id) = current_id {
                            return Ok(id);
                        }
                    }
                }
            }
        }
    }

    Err(PwError::NodeNotFound(format!(
        "Node with name '{target_name}' not found in PipeWire graph"
    )))
}

/// Modifies EQ band parameters on a running filter-chain node without reloading the process.
pub fn hot_update_eq_band(
    channel: ChannelId,
    band_idx: usize,
    freq: f32,
    q: f32,
    gain: f32,
) -> Result<()> {
    if band_idx >= 10 {
        return Err(PwError::Generic(format!(
            "Band index {band_idx} out of range (0..10)"
        )));
    }

    let node_name = if channel == ChannelId::Mic {
        "voicegg_mic_filter_in"
    } else {
        channel.pipewire_node_name()
    };

    let node_id = find_node_id_by_name(node_name)?;

    let param_json = format!(
        r#"{{"params":["eq_band_{band_idx}:Freq",{freq:.1},"eq_band_{band_idx}:Q",{q:.3},"eq_band_{band_idx}:Gain",{gain:.2}]}}"#
    );

    let output = Command::new("pw-cli")
        .args(["s", &node_id.to_string(), "Props", &param_json])
        .output()
        .map_err(|e| PwError::Generic(format!("Failed to execute pw-cli: {e}")))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(PwError::Generic(format!(
            "Failed to hot update EQ band {band_idx} on node {node_id}: {err}"
        )));
    }

    Ok(())
}

/// Spawns and manages a standalone `pipewire -c <conf_path>` process for an audio filter-chain.
#[derive(Debug)]
pub struct FilterChainRunner {
    name: String,
    conf_path: PathBuf,
    child: Option<std::process::Child>,
}

impl FilterChainRunner {
    /// Computes the directory used to store filter-chain configurations.
    pub fn config_dir() -> PathBuf {
        let base = dirs::runtime_dir()
            .unwrap_or_else(|| dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp")));
        base.join("voicegg")
    }

    /// Creates configuration file on disk and launches the filter-chain process.
    pub fn start(name: &str, conf_content: &str) -> Result<Self> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)
            .map_err(|e| PwError::Generic(format!("Failed to create voicegg runtime dir: {e}")))?;

        let conf_path = dir.join(format!("{name}.conf"));
        fs::write(&conf_path, conf_content).map_err(|e| {
            PwError::Generic(format!(
                "Failed to write filter-chain config to {}: {e}",
                conf_path.display()
            ))
        })?;

        tracing::info!(
            "Spawning filter-chain runner '{}' from {:?}",
            name,
            conf_path
        );

        let child = Command::new("pipewire")
            .args(["-c", conf_path.to_str().unwrap_or_default()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                PwError::Generic(format!("Failed to spawn pipewire -c for {name}: {e}"))
            })?;

        Ok(Self {
            name: name.to_string(),
            conf_path,
            child: Some(child),
        })
    }

    /// Replaces the configuration and re-executes the filter-chain process.
    pub fn reload(&mut self, conf_content: &str) -> Result<()> {
        self.stop();

        fs::write(&self.conf_path, conf_content).map_err(|e| {
            PwError::Generic(format!(
                "Failed to update filter-chain config {}: {e}",
                self.conf_path.display()
            ))
        })?;

        tracing::info!(
            "Reloading filter-chain runner '{}' from {:?}",
            self.name,
            self.conf_path
        );

        let child = Command::new("pipewire")
            .args(["-c", self.conf_path.to_str().unwrap_or_default()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                PwError::Generic(format!(
                    "Failed to reload pipewire -c for {}: {e}",
                    self.name
                ))
            })?;

        self.child = Some(child);
        Ok(())
    }

    /// Terminates the child process and cleans up the configuration file.
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_file(&self.conf_path);
    }
}

impl Drop for FilterChainRunner {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voicegg_core::{EqBand, EqConfig, FilterType};

    #[test]
    fn test_detect_capabilities_runs_without_panics() {
        let caps = detect_capabilities();
        assert!(caps.biquad_eq);
    }

    #[test]
    fn test_generate_channel_chain_conf_structure() {
        let preset = Preset {
            schema: 1,
            id: "test".into(),
            name: "Test Preset".into(),
            description: "".into(),
            category: voicegg_core::PresetCategory::Game,
            tags: vec![],
            eq: EqConfig {
                enabled: true,
                bands: vec![
                    EqBand {
                        freq_hz: 60.0,
                        gain_db: 4.5,
                        q: 1.0,
                        filter_type: FilterType::LowShelf,
                        enabled: true,
                    },
                    EqBand {
                        freq_hz: 1000.0,
                        gain_db: -2.0,
                        q: 1.2,
                        filter_type: FilterType::Peak,
                        enabled: true,
                    },
                ],
                bass_db: 0.0,
                voice_db: 0.0,
                treble_db: 0.0,
            },
            noise_canceller: None,
            compressor: None,
            noise_gate: None,
            limiter: None,
        };

        let caps = PluginCapabilities::default();
        let conf = generate_channel_chain_conf(ChannelId::Game, &preset, &caps);

        assert!(conf.contains("voicegg_sink_game"));
        assert!(conf.contains("voicegg_sink_master"));
        assert!(conf.contains("eq_band_0"));
        assert!(conf.contains("eq_band_9"));
        assert!(conf.contains("limiter"));
    }

    #[test]
    fn test_generate_mic_chain_conf_structure() {
        let preset = Preset {
            schema: 1,
            id: "test_mic".into(),
            name: "Test Mic".into(),
            description: "".into(),
            category: voicegg_core::PresetCategory::Mic,
            tags: vec![],
            eq: EqConfig::default(),
            noise_canceller: Some(voicegg_core::NoiseCancellerConfig {
                enabled: true,
                amount: 80.0,
                vad_threshold: 0.6,
            }),
            compressor: None,
            noise_gate: None,
            limiter: None,
        };

        let mut caps = PluginCapabilities::default();
        caps.rnnoise = true;

        let conf = generate_mic_chain_conf(&preset, &caps);
        assert!(conf.contains("voicegg_mic_filter_in"));
        assert!(conf.contains("voicegg_source_mic"));
        assert!(conf.contains("hp_80"));
        assert!(conf.contains("rnnoise"));
        assert!(conf.contains("noise_suppressor_mono"));
        assert!(conf.contains("eq_band_0"));
        assert!(conf.contains("limiter"));
    }
}
