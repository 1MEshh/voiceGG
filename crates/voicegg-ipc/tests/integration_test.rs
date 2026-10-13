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

    // Test shutdown request
    let res = client
        .request(IpcRequest::Shutdown)
        .await
        .expect("Shutdown request failed");
    assert_eq!(res, IpcResponse::Success);

    server_handle.abort();
    let _ = std::fs::remove_dir_all(&temp_dir);
}
