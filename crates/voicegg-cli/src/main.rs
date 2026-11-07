//! VoiceGG Command Line Interface (`voicegg`).

use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use voicegg_core::ChannelId;
use voicegg_ipc::{IpcClient, IpcRequest, IpcResponse};

#[derive(Parser)]
#[command(name = "voicegg")]
#[command(about = "High-performance PipeWire audio mixer and DSP controller")]
struct Cli {
    /// Hidden watchdog listening on stdin pipe
    #[arg(long = "janitor-watch", hide = true, num_args = 0..=2)]
    janitor_watch_flag: Option<Vec<String>>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Controller commands (volume, mute, routing, chatmix).
    Ctl(CtlArgs),
    /// Print daemon and audio graph status.
    Status {
        /// Output formatted as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Emergency panic reset of all virtual audio devices.
    Panic,
    /// Watchdog listening on daemon stdin pipe; cleans up graph if daemon terminates abruptly.
    #[command(name = "janitor-watch", hide = true)]
    JanitorWatch {
        /// Fallback physical sink name.
        #[arg(long)]
        phys_sink: Option<String>,
        /// Fallback physical source name.
        #[arg(long)]
        phys_source: Option<String>,
        /// Positional fallback arguments (e.g. `voicegg janitor-watch <sink> <source>`).
        #[arg(trailing_var_arg = true)]
        pos_args: Vec<String>,
    },
}

#[derive(Args)]
struct CtlArgs {
    #[command(subcommand)]
    action: CtlAction,
}

#[derive(Subcommand)]
enum CtlAction {
    /// Adjust channel volume.
    Volume {
        /// Channel to adjust (master, game, chat, media, aux, mic).
        channel: ChannelId,
        /// Volume in percent (0 - 150).
        volume: u8,
    },
    /// Mute or unmute channel.
    Mute {
        /// Channel to adjust.
        channel: ChannelId,
        /// Action: toggle, on, off.
        #[arg(default_value = "toggle")]
        state: String,
    },
    /// Set ChatMix balance (-100 to 100).
    Chatmix {
        /// Balance value (-100 to 100).
        #[arg(allow_hyphen_values = true)]
        value: i8,
    },
    /// Route an application to a specific channel.
    Route {
        /// Application binary name (e.g. firefox, discord, cs2).
        binary: String,
        /// Target channel.
        channel: ChannelId,
    },
    /// Toggle or adjust Live Voice Monitoring.
    Monitor {
        /// Action: toggle, on, off.
        #[arg(default_value = "toggle")]
        state: String,
        /// Volume in percent (0 - 100).
        #[arg(long, default_value_t = 50)]
        volume: u8,
    },
    /// Apply an equalizer preset to a channel.
    Preset {
        /// Channel to adjust (game, chat, media, mic).
        channel: ChannelId,
        /// Preset identifier (e.g. game_cs2, mic_broadcast).
        preset_id: String,
    },
    /// Cycle to the next preset for a channel.
    CyclePreset {
        /// Channel to cycle (game, chat, media, mic).
        #[arg(default_value = "game")]
        channel: ChannelId,
    },
    /// Mute or unmute an individual application stream.
    MuteApp {
        /// Application binary name.
        binary: String,
        /// Mute state (toggle, on, off).
        #[arg(default_value = "toggle")]
        state: String,
    },
    /// Reset an equalizer preset to factory defaults.
    ResetPreset {
        /// Channel to adjust (game, chat, media, aux, mic).
        channel: ChannelId,
        /// Preset identifier (e.g. game_flat, mic_broadcast).
        preset_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(args) = cli.janitor_watch_flag {
        let sink = args.first().cloned();
        let source = args.get(1).cloned();
        return run_janitor_watch(sink, source);
    }

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            println!("Usage: voicegg <COMMAND>");
            return Ok(());
        }
    };

    if let Commands::JanitorWatch { phys_sink, phys_source, pos_args } = command {
        let sink = phys_sink.or_else(|| pos_args.first().cloned());
        let source = phys_source.or_else(|| pos_args.get(1).cloned());
        return run_janitor_watch(sink, source);
    }

    if let Commands::Panic = command {
        if let Ok(mut client) = IpcClient::connect_default().await {
            let _ = client.request(IpcRequest::PanicReset).await;
        } else {
            voicegg_pw::GraphManager::cleanup_stale_modules();
        }
        println!("Audio graph reset to system defaults successfully.");
        return Ok(());
    }

    let mut client = match IpcClient::connect_default().await {
        Ok(c) => c,
        Err(e) => {
            bail!("Could not connect to voicegg-daemon. Is it running? (Error: {e})");
        }
    };

    match command {
        Commands::Panic | Commands::JanitorWatch { .. } => unreachable!(),
        Commands::Status { json } => match client.request(IpcRequest::GetStatus).await? {
            IpcResponse::Status(status) => {
                if json {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                } else {
                    println!("=== VoiceGG Status ===");
                    println!("ChatMix Balance: {}", status.config.chatmix);
                    println!(
                        "Mic Monitoring: {}",
                        if status.mic_monitor_enabled {
                            format!("ON ({}%)", status.mic_monitor_volume)
                        } else {
                            "OFF".to_string()
                        }
                    );
                    println!("Channels:");
                    for (ch, vol) in &status.config.volumes {
                        let muted = status.config.muted.get(ch).copied().unwrap_or(false);
                        println!(
                            "  - {:<8}: {:>3}% [{}]",
                            ch.display_name(),
                            vol,
                            if muted { "MUTED" } else { "ACTIVE" }
                        );
                    }
                    if !status.streams.is_empty() {
                        println!("Active Streams:");
                        for stream in &status.streams {
                            let ch = stream
                                .current_channel
                                .map(|c| c.display_name())
                                .unwrap_or("Unassigned");
                            println!(
                                "  - [{}] {} ({}) -> {}",
                                stream.id, stream.app_name, stream.binary_name, ch
                            );
                        }
                    }
                    if let Some(game) = status.active_game {
                        println!("Active Game: {}", game);
                    }
                }
            }
            other => println!("Unexpected response: {:?}", other),
        },
        Commands::Ctl(ctl) => match ctl.action {
            CtlAction::Volume { channel, volume } => {
                match client
                    .request(IpcRequest::SetVolume { channel, volume })
                    .await?
                {
                    IpcResponse::Success => println!("Set {} volume to {}%", channel, volume),
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
            CtlAction::Mute { channel, state } => {
                let mute_val = match state.to_lowercase().as_str() {
                    "on" | "true" | "1" => true,
                    "off" | "false" | "0" => false,
                    _ => {
                        // Query status first for toggle or any other command
                        match client.request(IpcRequest::GetStatus).await? {
                            IpcResponse::Status(s) => {
                                !s.config.muted.get(&channel).copied().unwrap_or(false)
                            }
                            _ => true,
                        }
                    }
                };

                match client
                    .request(IpcRequest::SetMute {
                        channel,
                        muted: mute_val,
                    })
                    .await?
                {
                    IpcResponse::Success => {
                        println!(
                            "{} is now {}",
                            channel,
                            if mute_val { "MUTED" } else { "UNMUTED" }
                        );
                    }
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
            CtlAction::Chatmix { value } => {
                match client.request(IpcRequest::SetChatMix { value }).await? {
                    IpcResponse::Success => println!("ChatMix balance set to {}", value),
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
            CtlAction::Route { binary, channel } => {
                match client
                    .request(IpcRequest::RouteApp {
                        binary_name: binary.clone(),
                        target_channel: channel,
                    })
                    .await?
                {
                    IpcResponse::Success => {
                        println!("Application '{}' routed to {}", binary, channel);
                    }
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
            CtlAction::Monitor { state, volume } => {
                let status = match client.request(IpcRequest::GetStatus).await? {
                    IpcResponse::Status(s) => *s,
                    _ => bail!("Failed to get current status"),
                };
                let current = status.mic_monitor_enabled;
                let target = match state.to_lowercase().as_str() {
                    "toggle" => !current,
                    "on" | "1" | "true" => true,
                    "off" | "0" | "false" => false,
                    _ => bail!("Invalid state: use 'toggle', 'on', or 'off'"),
                };
                match client
                    .request(IpcRequest::SetMicMonitor {
                        enabled: target,
                        volume,
                    })
                    .await?
                {
                    IpcResponse::Success => {
                        println!(
                            "Mic Monitoring is now {}",
                            if target { "ON" } else { "OFF" }
                        );
                    }
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
            CtlAction::Preset { channel, preset_id } => {
                let preset = match voicegg_core::Preset::get_preset_by_id(&preset_id) {
                    Some(p) => p,
                    None => bail!("Preset '{}' not found", preset_id),
                };
                match client.request(IpcRequest::SetPreset { channel, preset }).await? {
                    IpcResponse::Success => {
                        println!("Preset for {} set to '{}'", channel, preset_id);
                    }
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
            CtlAction::CyclePreset { channel } => {
                match client.request(IpcRequest::CyclePreset { channel }).await? {
                    IpcResponse::Success => {
                        println!("Cycled preset for {}", channel);
                    }
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
            CtlAction::MuteApp { binary, state } => {
                let muted = match state.to_lowercase().as_str() {
                    "on" | "true" | "1" => true,
                    "off" | "false" | "0" => false,
                    _ => true,
                };
                match client
                    .request(IpcRequest::SetAppMute {
                        binary_name: binary.clone(),
                        muted,
                    })
                    .await?
                {
                    IpcResponse::Success => {
                        println!(
                            "Application '{}' is now {}",
                            binary,
                            if muted { "MUTED" } else { "UNMUTED" }
                        );
                    }
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
            CtlAction::ResetPreset { channel, preset_id } => {
                match client
                    .request(IpcRequest::ResetPreset {
                        channel,
                        preset_id: preset_id.clone(),
                    })
                    .await?
                {
                    IpcResponse::Success => {
                        println!("Reset preset '{}' on {} to factory default", preset_id, channel);
                    }
                    IpcResponse::Error(e) => eprintln!("Error: {e}"),
                    _ => (),
                }
            }
        },
    }

    Ok(())
}

fn run_janitor_watch(phys_sink: Option<String>, phys_source: Option<String>) -> Result<()> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut line = String::new();

    loop {
        line.clear();
        match handle.read_line(&mut line) {
            Ok(0) => {
                // Pipe reached EOF without receiving "CLEAN" -> daemon terminated abruptly!
                eprintln!("[voicegg-janitor] Daemon disconnected abruptly (pipe EOF). Triggering emergency audio restoration!");
                emergency_cleanup_and_restore(phys_sink.as_deref(), phys_source.as_deref());
                std::process::exit(0);
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed == "CLEAN" {
                    // Normal clean shutdown of voicegg-daemon
                    std::process::exit(0);
                }
            }
            Err(e) => {
                eprintln!("[voicegg-janitor] Stdin read error: {e}. Restoring audio...");
                emergency_cleanup_and_restore(phys_sink.as_deref(), phys_source.as_deref());
                std::process::exit(1);
            }
        }
    }
}

fn emergency_cleanup_and_restore(phys_sink: Option<&str>, phys_source: Option<&str>) {
    // 1. Immediately switch PipeWire defaults back to physical hardware
    if let Some(sink) = phys_sink {
        let _ = std::process::Command::new("pactl")
            .args(["set-default-sink", sink])
            .status();
    }
    if let Some(source) = phys_source {
        let _ = std::process::Command::new("pactl")
            .args(["set-default-source", source])
            .status();
    }

    // 2. Unload all voicegg modules and kill orphaned DSP processes
    voicegg_pw::GraphManager::cleanup_stale_modules();

    // 3. Move active sink-inputs to physical sink
    if let Some(sink) = phys_sink {
        if let Ok(output) = std::process::Command::new("pactl").args(["list", "sink-inputs", "short"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(id_str) = line.split_whitespace().next() {
                    let _ = std::process::Command::new("pactl")
                        .args(["move-sink-input", id_str, sink])
                        .status();
                }
            }
        }
    }

    // 4. Move active source-outputs to physical source
    if let Some(source) = phys_source {
        if let Ok(output) = std::process::Command::new("pactl").args(["list", "source-outputs", "short"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(id_str) = line.split_whitespace().next() {
                    let _ = std::process::Command::new("pactl")
                        .args(["move-source-output", id_str, source])
                        .status();
                }
            }
        }
    }

    // 5. Scrub WirePlumber persistence files
    scrub_wireplumber();
}

fn scrub_wireplumber() {
    if let Some(data_home) = dirs::data_local_dir() {
        let wp_dir = data_home.join("wireplumber");
        if wp_dir.exists() {
            let targets = ["stream-properties", "default-nodes", "restore-stream"];
            for target in &targets {
                let path = wp_dir.join(target);
                if path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.contains("voicegg_") || content.contains("VoiceGG") {
                            let cleaned: Vec<&str> = content
                                .lines()
                                .filter(|line| !line.contains("voicegg_") && !line.contains("VoiceGG"))
                                .collect();
                            let _ = std::fs::write(&path, cleaned.join("\n"));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_janitor_watch_flag_parsing() {
        let cli = Cli::try_parse_from(["voicegg", "--janitor-watch", "my_sink", "my_source"])
            .expect("Parse failed");
        assert_eq!(
            cli.janitor_watch_flag,
            Some(vec!["my_sink".to_string(), "my_source".to_string()])
        );
    }

    #[test]
    fn test_janitor_watch_subcommand_parsing() {
        let cli = Cli::try_parse_from([
            "voicegg",
            "janitor-watch",
            "--phys-sink",
            "test_sink",
            "--phys-source",
            "test_source",
        ])
        .expect("Parse failed");
        match cli.command {
            Some(Commands::JanitorWatch {
                phys_sink,
                phys_source,
                ..
            }) => {
                assert_eq!(phys_sink.as_deref(), Some("test_sink"));
                assert_eq!(phys_source.as_deref(), Some("test_source"));
            }
            _ => panic!("Expected JanitorWatch subcommand"),
        }
    }

    #[test]
    fn test_ctl_actions_parsing() {
        let cli = Cli::try_parse_from(["voicegg", "ctl", "mute-app", "discord", "on"])
            .expect("Parse failed");
        match cli.command {
            Some(Commands::Ctl(ctl)) => match ctl.action {
                CtlAction::MuteApp { binary, state } => {
                    assert_eq!(binary, "discord");
                    assert_eq!(state, "on");
                }
                _ => panic!("Expected MuteApp"),
            },
            _ => panic!("Expected Ctl"),
        }

        let cli = Cli::try_parse_from(["voicegg", "ctl", "reset-preset", "game", "game_flat"])
            .expect("Parse failed");
        match cli.command {
            Some(Commands::Ctl(ctl)) => match ctl.action {
                CtlAction::ResetPreset { channel, preset_id } => {
                    assert_eq!(channel, ChannelId::Game);
                    assert_eq!(preset_id, "game_flat");
                }
                _ => panic!("Expected ResetPreset"),
            },
            _ => panic!("Expected Ctl"),
        }
    }
}
