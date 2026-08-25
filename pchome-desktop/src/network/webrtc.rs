use anyhow::Result;
use bytes::Bytes;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

use crate::control::ControlHandler;
use crate::network::ConnectionManager;

/// WebRTC offerer. Captures the desktop screen into an H.264 video track and
/// exchanges control messages with the mobile client over an unordered,
/// unreliable DataChannel named `control`.
pub struct WebRtcEngine {
    pc: Arc<RTCPeerConnection>,
    video_track: Arc<TrackLocalStaticSample>,
    data_channel: Arc<webrtc::data_channel::RTCDataChannel>,
    connection: Arc<ConnectionManager>,
    control: Arc<ControlHandler>,
    state: crate::state::SharedState,
    width: u32,
    height: u32,
    fps: u32,
}

impl WebRtcEngine {
    pub async fn build(
        connection: Arc<ConnectionManager>,
        control: Arc<ControlHandler>,
        state: crate::state::SharedState,
        width: u32,
        height: u32,
        fps: u32,
    ) -> Result<Arc<Self>> {
        let api = APIBuilder::new().build();
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let pc = Arc::new(api.new_peer_connection(config).await?);

        let video_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: "video/H264".to_string(),
                clock_rate: 90_000,
                channels: 0,
                sdp_fmtp_line:
                    "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f"
                        .to_string(),
                rtcp_feedback: vec![],
            },
            "video".to_string(),
            "pchome".to_string(),
        ));
        pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        // Unordered + unreliable DataChannel for lowest-latency control input.
        let dc_init = RTCDataChannelInit {
            ordered: Some(false),
            max_retransmits: Some(0),
            ..Default::default()
        };
        let dc = Arc::clone(&pc.create_data_channel("control", Some(dc_init)).await?);

        let conn_ice = connection.clone();
        pc.on_ice_candidate(Box::new(move |candidate: Option<webrtc::ice_transport::ice_candidate::RTCIceCandidate>| {
            let conn = conn_ice.clone();
            Box::pin(async move {
                if let Some(cand) = candidate {
                    if let Ok(init) = cand.to_json() {
                        let msg = json!({
                            "type": "ice-candidate",
                            "candidate": init.candidate,
                            "sdpMid": init.sdp_mid.unwrap_or_default(),
                            "sdpMLineIndex": init.sdp_mline_index.unwrap_or(0),
                        });
                        conn.send_json(&msg).await;
                    }
                }
            })
        }));

        let conn_open = connection.clone();
        let (w, h) = (width, height);
        dc.on_open(Box::new(move || {
            let conn = conn_open.clone();
            Box::pin(async move {
                conn.send_json(&json!({
                    "type": "display_info",
                    "width": w,
                    "height": h,
                }))
                .await;
            })
        }));

        let control_msg = control.clone();
        dc.on_message(Box::new(move |msg| {
            let control = control_msg.clone();
            Box::pin(async move {
                let data: Bytes = msg.data;
                if let Ok(v) = serde_json::from_slice::<Value>(&data) {
                    control.handle(&v).await;
                }
            })
        }));

        Ok(Arc::new(Self {
            pc,
            video_track,
            data_channel: dc,
            connection,
            control,
            state,
            width,
            height,
            fps,
        }))
    }

    /// Drives the signaling exchange: waits for the mobile `hello`, then offers,
    /// and applies the returned `answer` + `ice-candidate`s. Also emits a
    /// periodic ping over the control DataChannel so the GUI can display RTT.
    pub async fn run(self: Arc<Self>) -> Result<()> {
        self.spawn_ping_loop();
        let incoming = self.connection.incoming();
        let mut rx = incoming.lock().await;
        while let Some(msg) = rx.recv().await {
            let typ = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match typ {
                "hello" => {
                    if let Err(e) = self.create_offer().await {
                        log::warn!("create_offer failed: {}", e);
                    }
                }
                "answer" => {
                    if let Some(sdp) = msg.get("sdp").and_then(|v| v.as_str()) {
                        match RTCSessionDescription::answer(sdp.to_string()) {
                            Ok(ans) => {
                                if let Err(e) = self.pc.set_remote_description(ans).await {
                                    log::warn!("set_remote_description failed: {}", e);
                                }
                            }
                            Err(e) => log::warn!("invalid answer: {}", e),
                        }
                    }
                }
                "ice-candidate" => {
                    let candidate = msg.get("candidate").and_then(|v| v.as_str());
                    let sdp_mid = msg.get("sdpMid").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let sdp_mline_index = msg
                        .get("sdpMLineIndex")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as u16);
                    if let Some(candidate) = candidate {
                        let init = RTCIceCandidateInit {
                            candidate: candidate.to_string(),
                            sdp_mid,
                            sdp_mline_index,
                            username_fragment: None,
                        };
                        if let Err(e) = self.pc.add_ice_candidate(init).await {
                            log::warn!("add_ice_candidate failed: {}", e);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn create_offer(&self) -> Result<()> {
        let offer = self.pc.create_offer(None).await?;
        self.pc.set_local_description(offer).await?;
        if let Some(local) = self.pc.local_description().await {
            self.connection
                .send_json(&json!({ "type": "offer", "sdp": local.sdp }))
                .await;
        }
        Ok(())
    }

    /// Sends `{"type":"ping","t":<epoch_ms>}` every 5s while the control
    /// DataChannel is open; the mobile peer answers with `pong` and the
    /// desktop computes RTT into `AppState.ping_ms`.
    fn spawn_ping_loop(self: &Arc<Self>) {
        let engine = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if engine.data_channel.ready_state() != webrtc::data_channel::data_channel_state::RTCDataChannelState::Open {
                    continue;
                }
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let msg = json!({ "type": "ping", "t": now }).to_string();
                if let Err(e) = engine.data_channel.send(&bytes::Bytes::from(msg.into_bytes())).await {
                    log::debug!("ping send failed: {}", e);
                }
            }
        });
    }

    /// Push one encoded H.264 frame (Annex-B) into the outbound video track.
    pub async fn push_video_sample(&self, data: &[u8], _keyframe: bool) -> Result<()> {
        let sample = Sample {
            data: Bytes::from(data.to_vec()),
            timestamp: SystemTime::now(),
            duration: Duration::from_secs_f64(1.0 / self.fps.max(1) as f64),
            packet_timestamp: 0,
            prev_dropped_packets: 0,
            prev_padding_packets: 0,
        };
        self.video_track.write_sample(&sample).await?;
        Ok(())
    }
}
