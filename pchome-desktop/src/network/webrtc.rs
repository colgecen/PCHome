use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

pub struct PeerConnection {
    state: Arc<RwLock<PeerState>>,
    local_desc: Arc<RwLock<Option<String>>>,
    remote_desc: Arc<RwLock<Option<String>>>,
    ice_candidates: Arc<RwLock<Vec<String>>>,
    data_channel_tx: mpsc::UnboundedSender<DataChannelMessage>,
    data_channel_rx: Arc<RwLock<mpsc::UnboundedReceiver<DataChannelMessage>>>,
    video_track_tx: mpsc::UnboundedSender<VideoFrame>,
}

#[derive(Debug, Clone)]
pub struct DataChannelMessage {
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
    pub is_keyframe: bool,
}

impl PeerConnection {
    pub async fn new() -> Result<(Self, mpsc::UnboundedReceiver<DataChannelMessage>)> {
        let (tx, rx) = mpsc::unbounded_channel();

        Ok((
            Self {
                state: Arc::new(RwLock::new(PeerState::New)),
                local_desc: Arc::new(RwLock::new(None)),
                remote_desc: Arc::new(RwLock::new(None)),
                ice_candidates: Arc::new(RwLock::new(Vec::new())),
                data_channel_tx: tx,
                data_channel_rx: Arc::new(RwLock::new(rx)),
                video_track_tx: mpsc::unbounded_channel().0,
            },
            mpsc::unbounded_channel().1,
        ))
    }

    pub async fn set_local_description(&self, sdp: String) -> Result<()> {
        log::info!("Setting local SDP description");
        *self.local_desc.write().await = Some(sdp);
        *self.state.write().await = PeerState::Connecting;
        Ok(())
    }

    pub async fn set_remote_description(&self, sdp: String) -> Result<()> {
        log::info!("Setting remote SDP description");
        *self.remote_desc.write().await = Some(sdp);
        self.start_connection().await
    }

    pub async fn add_ice_candidate(&self, candidate: String) -> Result<()> {
        log::info!("Adding ICE candidate: {}", candidate);
        self.ice_candidates.write().await.push(candidate);
        Ok(())
    }

    pub async fn create_data_channel(&self, label: &str) -> Result<()> {
        log::info!("Creating data channel: {}", label);
        Ok(())
    }

    pub async fn send(&self, msg: DataChannelMessage) -> Result<()> {
        self.data_channel_tx.send(msg)?;
        Ok(())
    }

    pub async fn send_video(&self, frame: VideoFrame) -> Result<()> {
        let _ = self.video_track_tx.send(frame);
        Ok(())
    }

    pub async fn state(&self) -> PeerState {
        *self.state.read().await
    }

    pub async fn local_description(&self) -> Option<String> {
        self.local_desc.read().await.clone()
    }

    pub async fn close(&self) -> Result<()> {
        log::info!("Closing peer connection");
        *self.state.write().await = PeerState::Disconnected;
        Ok(())
    }

    async fn start_connection(&self) -> Result<()> {
        log::info!("Starting WebRTC connection");
        *self.state.write().await = PeerState::Connected;
        Ok(())
    }
}
