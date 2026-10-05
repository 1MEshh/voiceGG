//! VoiceGG Command Line Interface (`voicegg`).

use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use voicegg_core::ChannelId;
use voicegg_ipc::{IpcClient, IpcRequest, IpcResponse};

#[derive(Parser)]
#[command(name = "voicegg")]
#[command(about = "High-performance PipeWire audio mixer and DSP controller")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
        /// Balance value.
        value: i8,
    },
    /// Route an application to a specific channel.
    Route {
        /// Application binary name (e.g. firefox, discord, cs2).
        binary: String,
        /// Target channel.
        channel: ChannelId,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut client = match IpcClient::connect_default().await {
        Ok(c) => c,
        Err(e) => {
            bail!("Could not connect to voicegg-daemon. Is it running? (Error: {e})");
        }
    };

    match cli.command {
        Commands::Status { json } => match client.request(IpcRequest::GetStatus).await? {
            IpcResponse::Status(status) => {
                if json {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                } else {
                    println!("=== VoiceGG Status ===");
                    println!("ChatMix Balance: {}", status.config.chatmix);
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
        Commands::Panic => match client.request(IpcRequest::PanicReset).await? {
            IpcResponse::Success => println!("Audio graph reset to system defaults successfully."),
            IpcResponse::Error(e) => eprintln!("Error resetting graph: {e}"),
            _ => (),
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
        },
    }

    Ok(())
}
