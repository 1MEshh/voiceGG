use std::sync::Arc;
use tokio::time::{sleep, Duration};
use voicegg_core::ChannelId;
use voicegg_ipc::{IpcClient, IpcRequest, IpcResponse, IpcServer};

#[tokio::test]
async fn test_client_server_lifecycle() {
    let temp_dir = std::env::temp_dir().join(format!("voicegg_test_{}", std::process::id()));
    let socket_path = temp_dir.join("test.sock");

    let server = IpcServer::bind(&socket_path).expect("Failed to bind test IPC server");

    let server_handle = tokio::spawn(async move {
        server
            .run(Arc::new(|req: IpcRequest| match req {
                IpcRequest::Shutdown => IpcResponse::Success,
                IpcRequest::SetVolume { volume, .. } => {
                    if volume > 100 {
                        IpcResponse::Error("Volume too high".into())
                    } else {
                        IpcResponse::Success
                    }
                }
                _ => IpcResponse::Success,
            }))
            .await
            .expect("Server loop failed");
    });

    sleep(Duration::from_millis(50)).await;

    let mut client = IpcClient::connect(&socket_path)
        .await
        .expect("Failed to connect client");

    // Test volume request
    let res = client
        .request(IpcRequest::SetVolume {
            channel: ChannelId::Master,
            volume: 70,
        })
        .await
        .expect("Request failed");
    assert_eq!(res, IpcResponse::Success);

    // Test shutdown request
    let res = client
        .request(IpcRequest::Shutdown)
        .await
        .expect("Shutdown request failed");
    assert_eq!(res, IpcResponse::Success);

    server_handle.abort();
    let _ = std::fs::remove_dir_all(&temp_dir);
}
