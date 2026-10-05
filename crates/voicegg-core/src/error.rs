//! Error types for voicegg-core.

use thiserror::Error;

/// Error enumeration for core operations.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Invalid parameter range.
    #[error("Value out of valid range: {0}")]
    OutOfRange(String),

    /// Missing or invalid configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Preset parse or validation failure.
    #[error("Invalid preset: {0}")]
    InvalidPreset(String),
}

/// Standard Result type for core operations.
pub type Result<T> = std::result::Result<T, CoreError>;
