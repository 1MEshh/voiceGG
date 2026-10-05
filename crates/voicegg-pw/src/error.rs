//! Error types for PipeWire operations.

use thiserror::Error;

/// Error enumeration for PipeWire subsystem.
#[derive(Debug, Error)]
pub enum PwError {
    /// PipeWire initialization failure.
    #[error("Failed to initialize PipeWire context: {0}")]
    InitFailed(String),

    /// Connection to PipeWire daemon failed.
    #[error("Failed to connect to PipeWire daemon: {0}")]
    ConnectionFailed(String),

    /// Device or node was not found in the graph.
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Metadata routing update error.
    #[error("Routing error: {0}")]
    RoutingFailed(String),

    /// Generic PipeWire error.
    #[error("PipeWire error: {0}")]
    Generic(String),
}

/// Standard Result type for PipeWire operations.
pub type Result<T> = std::result::Result<T, PwError>;
