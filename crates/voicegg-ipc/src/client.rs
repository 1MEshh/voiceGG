//! IPC Client for CLI and GUI frontends.

use crate::error::{IpcError, Result};
use crate::protocol::{IpcRequest, IpcResponse, RequestConfig};
use futures_util::{SinkExt, StreamExt};
use std::path::Path;
use std::time::Duration;
use tokio::net::UnixStream;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

/// Client for connecting to the VoiceGG daemon over Unix domain sockets.
pub struct IpcClient {
    writer: FramedWrite<tokio::net::unix::OwnedWriteHalf, LinesCodec>,
    reader: FramedRead<tokio::net::unix::OwnedReadHalf, LinesCodec>,
    request_config: RequestConfig,
}

impl IpcClient {
    /// Connects to the daemon at the specified socket path.
    pub async fn connect<P: AsRef<Path>>(socket_path: P) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        let peer_cred = stream.peer_cred().map_err(IpcError::Io)?;
        let peer_uid = peer_cred.uid();
        let expected_uid = crate::current_process_uid().map_err(IpcError::Io)?;
        if peer_uid != expected_uid {
            return Err(IpcError::ServerError(format!(
                "Daemon peer UID mismatch: expected {expected_uid}, received {peer_uid}"
            )));
        }

        let (read_half, write_half) = stream.into_split();
        let writer = FramedWrite::new(write_half, LinesCodec::new_with_max_length(65_536));
        let reader = FramedRead::new(read_half, LinesCodec::new_with_max_length(65_536));
        Ok(Self {
            writer,
            reader,
            request_config: RequestConfig::default(),
        })
    }

    /// Connects using the standard user runtime directory.
    pub async fn connect_default() -> Result<Self> {
        Self::connect(crate::default_socket_path()).await
    }

    /// Returns the active request configuration.
    #[must_use]
    pub fn request_config(&self) -> RequestConfig {
        self.request_config
    }

    /// Updates the request configuration.
    pub fn set_request_config(&mut self, config: RequestConfig) {
        self.request_config = config;
    }

    /// Configures the request timeout duration.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.request_config.timeout_ms = timeout.as_millis() as u64;
    }

    /// Sends a request with an explicit timeout duration.
    pub async fn request_with_timeout(
        &mut self,
        req: IpcRequest,
        timeout: Duration,
    ) -> Result<IpcResponse> {
        let serialized = serde_json::to_string(&req)?;
        self.writer
            .send(serialized)
            .await
            .map_err(|e| IpcError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, e)))?;

        let read_future = self.reader.next();
        let response_line = tokio::time::timeout(timeout, read_future)
            .await
            .map_err(|_| {
                IpcError::Protocol(format!(
                    "Request timed out after {}ms",
                    timeout.as_millis()
                ))
            })?
            .ok_or_else(|| IpcError::Protocol("Connection closed before response".to_string()))?
            .map_err(|e| IpcError::Protocol(e.to_string()))?;

        let res: IpcResponse = serde_json::from_str(&response_line)?;
        Ok(res)
    }

    /// Sends a request to the daemon and awaits its response using the configured timeout threshold.
    pub async fn request(&mut self, req: IpcRequest) -> Result<IpcResponse> {
        self.request_with_timeout(req, self.request_config.duration()).await
    }

    /// Sends a keepalive ping request to verify daemon responsiveness.
    pub async fn ping(&mut self) -> Result<()> {
        match self.request(IpcRequest::Ping).await? {
            IpcResponse::Pong => Ok(()),
            other => Err(IpcError::Protocol(format!("Expected Pong, got {other:?}"))),
        }
    }
}
