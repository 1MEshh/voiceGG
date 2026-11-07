//! IPC Server for VoiceGG daemon.

use crate::error::{IpcError, Result};
use crate::protocol::{IpcRequest, IpcResponse};
use futures_util::{SinkExt, StreamExt};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

/// Handler trait for processing incoming requests from clients.
pub trait RequestHandler: Send + Sync + 'static {
    /// Handles an incoming request and returns a response.
    fn handle_request(&self, req: IpcRequest) -> IpcResponse;
}

impl<F> RequestHandler for F
where
    F: Fn(IpcRequest) -> IpcResponse + Send + Sync + 'static,
{
    fn handle_request(&self, req: IpcRequest) -> IpcResponse {
        self(req)
    }
}

/// Unix domain socket server running inside the daemon.
pub struct IpcServer {
    socket_path: PathBuf,
    listener: UnixListener,
}

impl IpcServer {
    /// Binds the server to the given socket path, ensuring 0600 permissions.
    pub fn bind<P: AsRef<Path>>(socket_path: P) -> Result<Self> {
        let path = socket_path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            // Set 0700 permissions on runtime directory
            let mut perms = fs::metadata(parent)?.permissions();
            perms.set_mode(0o700);
            fs::set_permissions(parent, perms)?;
        }

        // Clean up stale socket if it exists, or abort if another daemon is active
        if path.exists() {
            if std::os::unix::net::UnixStream::connect(&path).is_ok() {
                return Err(IpcError::ServerError(
                    "Another voicegg-daemon instance is already running on this socket".into(),
                ));
            }
            let _ = fs::remove_file(&path);
        }

        let listener = UnixListener::bind(&path)?;

        // Enforce 0600 on the socket
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;

        Ok(Self {
            socket_path: path,
            listener,
        })
    }

    /// Runs the request acceptance loop.
    pub async fn run<H: RequestHandler>(self, handler: Arc<H>) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    let handler_clone = Arc::clone(&handler);
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, handler_clone).await {
                            tracing::debug!("IPC client connection closed: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("IPC accept error: {}", e);
                }
            }
        }
    }

    async fn handle_connection<H: RequestHandler>(
        stream: UnixStream,
        handler: Arc<H>,
    ) -> Result<()> {
        let peer_cred = stream.peer_cred().map_err(IpcError::Io)?;
        let peer_uid = peer_cred.uid();
        let expected_uid = crate::current_process_uid().map_err(IpcError::Io)?;
        if peer_uid != expected_uid {
            tracing::warn!("Unauthorized IPC client: UID {peer_uid} != expected {expected_uid}");
            return Err(IpcError::ServerError(format!(
                "Unauthorized client UID: {peer_uid}"
            )));
        }

        let (read_half, write_half) = stream.into_split();
        let mut reader = FramedRead::new(read_half, LinesCodec::new_with_max_length(65_536));
        let mut writer = FramedWrite::new(write_half, LinesCodec::new_with_max_length(65_536));

        while let Some(line) = reader.next().await {
            let line = line.map_err(|e| IpcError::Protocol(e.to_string()))?;
            let req: IpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let err_resp = IpcResponse::Error(format!("Malformed request JSON: {e}"));
                    let serialized = serde_json::to_string(&err_resp)?;
                    writer.send(serialized).await.map_err(|err| {
                        IpcError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, err))
                    })?;
                    continue;
                }
            };

            let resp = handler.handle_request(req);
            let serialized = serde_json::to_string(&resp)?;
            writer.send(serialized).await.map_err(|err| {
                IpcError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, err))
            })?;
        }

        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}
