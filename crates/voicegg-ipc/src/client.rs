//! IPC Client for CLI and GUI frontends.

use crate::error::{IpcError, Result};
use crate::protocol::{IpcRequest, IpcResponse};
use futures_util::{SinkExt, StreamExt};
use std::path::Path;
use tokio::net::UnixStream;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

/// Client for connecting to the VoiceGG daemon over Unix domain sockets.
pub struct IpcClient {
    writer: FramedWrite<tokio::net::unix::OwnedWriteHalf, LinesCodec>,
    reader: FramedRead<tokio::net::unix::OwnedReadHalf, LinesCodec>,
}

impl IpcClient {
    /// Connects to the daemon at the specified socket path.
    pub async fn connect<P: AsRef<Path>>(socket_path: P) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = stream.into_split();
        let writer = FramedWrite::new(write_half, LinesCodec::new());
        let reader = FramedRead::new(read_half, LinesCodec::new());
        Ok(Self { writer, reader })
    }

    /// Connects using the standard user runtime directory.
    pub async fn connect_default() -> Result<Self> {
        Self::connect(crate::default_socket_path()).await
    }

    /// Sends a request to the daemon and awaits its response.
    pub async fn request(&mut self, req: IpcRequest) -> Result<IpcResponse> {
        let serialized = serde_json::to_string(&req)?;
        self.writer
            .send(serialized)
            .await
            .map_err(|e| IpcError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, e)))?;

        let response_line = self
            .reader
            .next()
            .await
            .ok_or_else(|| IpcError::Protocol("Connection closed before response".to_string()))?
            .map_err(|e| IpcError::Protocol(e.to_string()))?;

        let res: IpcResponse = serde_json::from_str(&response_line)?;
        Ok(res)
    }
}
