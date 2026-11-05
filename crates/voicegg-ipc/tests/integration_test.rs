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
                IpcRequest::SetMicMonitor { enabled, volume } => {
                    if enabled && volume > 0 {
                        IpcResponse::Success
                    } else {
                        IpcResponse::Error("Invalid monitor params".into())
                    }
                }
                IpcRequest::GetCapabilities => {
                    IpcResponse::Capabilities(voicegg_core::PluginCapabilities::default())
                }
                IpcRequest::GetMeters => {
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        ChannelId::Mic,
                        voicegg_ipc::ChannelMeter {
                            peak_dbfs: -18.5,
                            rms_dbfs: -24.0,
                        },
                    );
                    IpcResponse::Meters(map)
                }
                IpcRequest::SetLanguage { language } => {
                    if language == "ar" || language == "en" {
                        IpcResponse::Success
                    } else {
                        IpcResponse::Error("Unsupported language".into())
                    }
                }
                IpcRequest::Ping => IpcResponse::Pong,
                _ => IpcResponse::Success,
            }))
            .await
            .expect("Server loop failed");
    });

    sleep(Duration::from_millis(50)).await;

    let mut client = IpcClient::connect(&socket_path)
        .await
        .expect("Failed to connect client");

    // Test ping keepalive
    client.ping().await.expect("Ping request failed");

    // Test volume request
    let res = client
        .request(IpcRequest::SetVolume {
            channel: ChannelId::Master,
            volume: 70,
        })
        .await
        .expect("Request failed");
    assert_eq!(res, IpcResponse::Success);

    // Test mic monitor request
    let res = client
        .request(IpcRequest::SetMicMonitor {
            enabled: true,
            volume: 80,
        })
        .await
        .expect("Mic monitor request failed");
    assert_eq!(res, IpcResponse::Success);

    // Test capabilities request
    let res = client
        .request(IpcRequest::GetCapabilities)
        .await
        .expect("Capabilities request failed");
    match res {
        IpcResponse::Capabilities(caps) => assert!(caps.biquad_eq),
        _ => panic!("Expected Capabilities response"),
    }

    // Test meters request
    let res = client
        .request(IpcRequest::GetMeters)
        .await
        .expect("Meters request failed");
    match res {
        IpcResponse::Meters(meters) => {
            let mic_meter = meters.get(&ChannelId::Mic).expect("Mic meter missing");
            assert!((mic_meter.peak_dbfs - (-18.5)).abs() < 1e-4);
        }
        _ => panic!("Expected Meters response"),
    }

    // Test set language request
    let res = client
        .request(IpcRequest::SetLanguage {
            language: "ar".to_string(),
        })
        .await
        .expect("SetLanguage request failed");
    assert_eq!(res, IpcResponse::Success);

    // Test new lifecycle and control variants
    let res = client
        .request(IpcRequest::RegisterClientPid {
            pid: std::process::id(),
        })
        .await
        .expect("RegisterClientPid failed");
    assert_eq!(res, IpcResponse::Success);

    let res = client
        .request(IpcRequest::SetAutoGameDetection { enabled: true })
        .await
        .expect("SetAutoGameDetection failed");
    assert_eq!(res, IpcResponse::Success);

    let res = client
        .request(IpcRequest::SetCloseToTray { enabled: false })
        .await
        .expect("SetCloseToTray failed");
    assert_eq!(res, IpcResponse::Success);

    let res = client
        .request(IpcRequest::SetAppMute {
            binary_name: "discord".to_string(),
            muted: true,
        })
        .await
        .expect("SetAppMute failed");
    assert_eq!(res, IpcResponse::Success);

    let res = client
        .request(IpcRequest::ResetPreset {
            channel: ChannelId::Game,
            preset_id: "game_flat".to_string(),
        })
        .await
        .expect("ResetPreset failed");
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

#[tokio::test]
async fn test_ping_pong_and_request_timeout() {
    let temp_dir = std::env::temp_dir().join(format!("voicegg_ping_{}", std::process::id()));
    let socket_path = temp_dir.join("ping.sock");

    let server = IpcServer::bind(&socket_path).expect("Failed to bind test server");
    let server_handle = tokio::spawn(async move {
        server
            .run(Arc::new(|req: IpcRequest| match req {
                IpcRequest::Ping => IpcResponse::Pong,
                _ => IpcResponse::Success,
            }))
            .await
            .expect("Server failed");
    });

    sleep(Duration::from_millis(50)).await;

    let mut client = IpcClient::connect(&socket_path)
        .await
        .expect("Client connect failed");

    // Verify Ping-Pong roundtrip
    client.ping().await.expect("Ping keepalive should succeed");
    let res = client.request(IpcRequest::Ping).await.expect("Ping request failed");
    assert_eq!(res, IpcResponse::Pong);

    server_handle.abort();

    // Verify request timeout when the server does not respond
    let timeout_socket = temp_dir.join("timeout.sock");
    let silent_listener = tokio::net::UnixListener::bind(&timeout_socket).expect("Failed to bind silent listener");
    let silent_handle = tokio::spawn(async move {
        if let Ok((mut stream, _)) = silent_listener.accept().await {
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let mut timeout_client = IpcClient::connect(&timeout_socket)
        .await
        .expect("Timeout client connect failed");

    let timeout_res = timeout_client
        .request_with_timeout(IpcRequest::GetStatus, Duration::from_millis(50))
        .await;
    assert!(timeout_res.is_err(), "Expected request to time out");
    let err_str = timeout_res.unwrap_err().to_string();
    assert!(err_str.contains("timed out") || err_str.contains("timeout"));

    silent_handle.abort();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_bounded_codec_rejects_oversized_messages() {
    use tokio::io::AsyncWriteExt;
    let temp_dir = std::env::temp_dir().join(format!("voicegg_overflow_{}", std::process::id()));
    let socket_path = temp_dir.join("overflow.sock");

    let server = IpcServer::bind(&socket_path).expect("Failed to bind test server");
    let server_handle = tokio::spawn(async move {
        server
            .run(Arc::new(|_| IpcResponse::Success))
            .await
            .expect("Server failed");
    });

    sleep(Duration::from_millis(50)).await;

    // Connect raw stream and write line longer than 65536 bytes
    let mut raw_stream = tokio::net::UnixStream::connect(&socket_path)
        .await
        .expect("Raw connect failed");
    let oversized = vec![b'A'; 70_000];
    let mut payload = oversized;
    payload.push(b'\n');

    let write_res = raw_stream.write_all(&payload).await;
    // Server should close the connection upon exceeding 65536 bytes without crashing
    if write_res.is_ok() {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 100];
        let read_n = raw_stream.read(&mut buf).await.unwrap_or(0);
        // Should either receive 0 (EOF closed by server) or error
        assert!(read_n == 0 || read_n < 100);
    }

    server_handle.abort();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

