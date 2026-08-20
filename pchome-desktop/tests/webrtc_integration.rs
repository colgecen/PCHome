use anyhow::Result;
use pchome_desktop::network::webrtc::{PeerConnection, PeerState, VideoFrame, DataChannelMessage};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_webrtc_peer_connection_lifecycle() -> Result<()> {
    let (pc, _rx) = PeerConnection::new().await?;
    assert_eq!(pc.state().await, PeerState::New);

    let sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\n".to_string();
    pc.set_local_description(sdp.clone()).await?;
    assert!(pc.local_description().await.is_some());

    pc.set_remote_description(sdp).await?;
    assert_eq!(pc.state().await, PeerState::Connected);

    pc.close().await?;
    assert_eq!(pc.state().await, PeerState::Disconnected);
    Ok(())
}

#[tokio::test]
async fn test_webrtc_data_channel_send_receive() -> Result<()> {
    let (pc, mut rx) = PeerConnection::new().await?;
    pc.create_data_channel("test").await?;

    let msg = DataChannelMessage {
        payload: b"hello".to_vec(),
        timestamp: now_micros(),
    };

    pc.send(msg.clone()).await?;

    let received = rx.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap().payload, b"hello");
    Ok(())
}

#[tokio::test]
async fn test_webrtc_ice_candidate_collection() -> Result<()> {
    let (pc, _rx) = PeerConnection::new().await?;

    pc.add_ice_candidate("candidate:1 1 UDP 2130706431 192.168.1.2 54400 typ host".to_string())
        .await?;

    pc.set_local_description("dummy".to_string()).await?;
    Ok(())
}

#[tokio::test]
async fn test_webrtc_video_track() -> Result<()> {
    let (pc, _rx) = PeerConnection::new().await?;

    let frame = VideoFrame {
        data: vec![0u8; 1000],
        width: 640,
        height: 480,
        timestamp: now_micros(),
        is_keyframe: true,
    };

    pc.send_video(frame).await?;
    Ok(())
}

fn now_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}
