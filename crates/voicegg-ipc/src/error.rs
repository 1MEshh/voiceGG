//! Error types for VoiceGG IPC.

use thiserror::Error;

/// Errors that can occur during IPC communication.
#[derive(Debug, Error)]
pub enum IpcError {
    /// IO error (socket connection, read, write).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization failure.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Handshake or authentication failure.
    #[error("Authentication or peer credential check failed: {0}")]
    Auth(String),

    /// Server returned an explicit error response.
    #[error("Server returned error: {0}")]
    ServerError(String),

    /// Protocol framing or invalid message format.
    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Standard Result type for IPC operations.
pub type Result<T> = std::result::Result<T, IpcError>;
