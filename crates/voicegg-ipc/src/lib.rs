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
pub use protocol::{IpcEvent, IpcRequest, IpcResponse, SystemStatus};
pub use server::{IpcServer, RequestHandler};

use std::path::PathBuf;

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
