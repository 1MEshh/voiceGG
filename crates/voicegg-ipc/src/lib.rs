#![forbid(unsafe_code)]

//! Inter-Process Communication (IPC) library for VoiceGG.
//!
//! Provides a secure, local-only Unix domain socket server and client
//! for communication between `voicegg-daemon`, `voicegg-cli`, and the GUI.

pub mod client;
pub mod error;
pub mod protocol;
pub mod server;

pub use client::IpcClient;
pub use error::{IpcError, Result};
pub use protocol::{ChannelMeter, IpcEvent, IpcRequest, IpcResponse, SystemStatus};
pub use server::{IpcServer, RequestHandler};

use std::path::PathBuf;

/// Securely retrieves the current process effective UID without unsafe code.
pub fn current_process_uid() -> std::io::Result<u32> {
    use std::os::unix::fs::MetadataExt;
    if let Ok(meta) = std::fs::metadata("/proc/self") {
        return Ok(meta.uid());
    }
    if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                let mut parts = rest.split_whitespace();
                if let Some(uid_str) = parts.next() {
                    if let Ok(uid) = uid_str.parse::<u32>() {
                        return Ok(uid);
                    }
                }
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Unable to determine current process UID",
    ))
}

/// Computes the default Unix domain socket path in $XDG_RUNTIME_DIR.
#[must_use]
pub fn default_socket_path() -> PathBuf {
    if let Some(runtime_dir) = dirs::runtime_dir() {
        runtime_dir.join("voicegg").join("daemon.sock")
    } else {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("voicegg")
            .join("daemon.sock")
    }
}
