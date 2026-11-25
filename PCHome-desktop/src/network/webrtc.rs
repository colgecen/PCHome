use anyhow::Result;
use bytes::Bytes;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::rtp_transceiver::RTCPFeedback;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;
use webrtc_ice::network_type::NetworkType;

use crate::control::ControlHandler;
use crate::network::ConnectionManager;

/// WebRTC offerer. Captures the desktop screen into an H.264 video track and
/// exchanges control messages with the mobile client over two DataChannels:
/// `control` (unordered/unreliable, pointer movement) and `input` (reliable
/// ordered, keyboard/clicks). Separating them keeps typing snappy even while
/// the pointer is streaming at a high rate.
///
/// One instance is created per mobile `hello`. The old instance is closed and
/// dropped before a new one is built, because DTLS/SRTP/ICE state cannot be
/// reused across phone reconnects.
pub struct WebRtcEngine {
    pc: Arc<RTCPeerConnection>,
    video_track: Arc<TrackLocalStaticSample>,
    data_channel: Arc<webrtc::data_channel::RTCDataChannel>,
    input_channel: Arc<webrtc::data_channel::RTCDataChannel>,
    connection: Arc<ConnectionManager>,
    control: Arc<ControlHandler>,
    state: crate::state::SharedState,
    width: u32,
    height: u32,
    fps: u32,
    generation: u64,
    last_sample: tokio::sync::Mutex<Option<Instant>>,
    /// ICE candidates that arrived before the answer/remote description.
    pending_ice: tokio::sync::Mutex<Vec<RTCIceCandidateInit>>,
    closed: AtomicBool,
}

fn h264_capability() -> RTCRtpCodecCapability {
    RTCRtpCodecCapability {
        mime_type: "video/H264".to_string(),
        clock_rate: 90_000,
        channels: 0,
        sdp_fmtp_line: "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f"
            .to_string(),
        // PLI/FIR let the phone ask for a fresh keyframe after loss or when it
        // just started decoding; NACK improves recovery on lossy links.
        rtcp_feedback: vec![
            RTCPFeedback {
                typ: "nack".to_string(),
                parameter: String::new(),
            },
            RTCPFeedback {
                typ: "nack".to_string(),
                parameter: "pli".to_string(),
            },
            RTCPFeedback {
                typ: "ccm".to_string(),
                parameter: "fir".to_string(),
            },
            RTCPFeedback {
                typ: "goog-remb".to_string(),
                parameter: String::new(),
            },
        ],
    }
}

impl WebRtcEngine {
    pub async fn build(
        connection: Arc<ConnectionManager>,
        control: Arc<ControlHandler>,
        state: crate::state::SharedState,
        width: u32,
        height: u32,
        fps: u32,
        generation: u64,
    ) -> Result<Arc<Self>> {
        // Telefon uygulaması yeniden açıldığında sıra numarası baştan
        // başlayabilir; yeni oturumda eski sıra numarası kalmasın. Önceki
        // oturumdan basılı tuş/buton kaldıysa da bırak.
        control.reset_pointer_seq();
        control.release_all_input().await;
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_codec(
                RTCRtpCodecParameters {
                    capability: h264_capability(),
                    payload_type: 102,
                    ..Default::default()
                },
                RTPCodecType::Video,
            )
            .map_err(|e| anyhow::anyhow!("failed to register H264 codec: {}", e))?;
        let mut setting_engine = SettingEngine::default();
        // Restrict ICE to IPv4 UDP only. Binding to IPv6 link-local (fe80::)
        // addresses fails with EINVAL on Linux and only produces spurious
        // warnings; link-local can't route across the signal server anyway.
        setting_engine.set_network_types(vec![NetworkType::Udp4]);
        // Sanal/VPN arayüzlerini ICE'den çıkar. Docker/Tailscale adayları
        // telefonun yanlış (yüksek gecikmeli) yoldan bağlanmasına yol
        // açabiliyordu; Wi-Fi/Ethernet ve onun üzerinden STUN yeterli.
        setting_engine.set_interface_filter(Box::new(|name: &str| {
            !(name == "lo"
                || name.starts_with("docker")
                || name.starts_with("veth")
                || name.starts_with("br-")
                || name.starts_with("virbr")
                || name.starts_with("tailscale")
                || name.starts_with("tun")
                || name.starts_with("wg"))
        }));
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_setting_engine(setting_engine)
            .build();
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let pc = Arc::new(api.new_peer_connection(config).await?);

        let video_track = Arc::new(TrackLocalStaticSample::new(
            h264_capability(),
            "video".to_string(),
            "pchome".to_string(),
        ));
        pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        // Unordered + unreliable DataChannel for lowest-latency pointer input.
        let dc_init = RTCDataChannelInit {
            ordered: Some(false),
            max_retransmits: Some(0),
            ..Default::default()
        };
        let dc = Arc::clone(&pc.create_data_channel("control", Some(dc_init)).await?);

        // Reliable, ordered channel for keyboard/clicks. Putting these on a
        // separate SCTP stream prevents lost/reordered key events and stops a
        // flood of pointer moves from delaying typing.
        let input_init = RTCDataChannelInit {
            ordered: Some(true),
            ..Default::default()
        };
        let input_dc = Arc::clone(&pc.create_data_channel("input", Some(input_init)).await?);

        let conn_ice = connection.clone();
        pc.on_ice_candidate(Box::new(
            move |candidate: Option<webrtc::ice_transport::ice_candidate::RTCIceCandidate>| {
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
            },
        ));

        let conn_open = connection.clone();
        let state_open = state.clone();
        let dc_open = Arc::clone(&dc);
        let (w, h) = (width, height);
        dc.on_open(Box::new(move || {
            let conn = conn_open.clone();
            let st = state_open.clone();
            let dc = Arc::clone(&dc_open);
            Box::pin(async move {
                st.peer_generation.store(generation, Ordering::SeqCst);
                st.peer_connected.store(true, Ordering::SeqCst);
                st.peer_since.store(now_epoch(), Ordering::SeqCst);
                st.mode_explicit.store(false, Ordering::SeqCst);
                st.push_event(
                    crate::i18n::t(
                        "device connected (data channel open)",
                        "cihaz bağlandı (veri kanalı açık)",
                    )
                    .to_string(),
                );
                log::info!(
                    "control data channel opened (session {}, {}x{})",
                    generation,
                    w,
                    h
                );
                let info = json!({
                    "type": "display_info",
                    "width": w,
                    "height": h,
                });
                // Signal yolu kopsa bile telefon gerçek çözünürlüğü
                // veri kanalından alsın.
                let _ = dc.send(&Bytes::from(info.to_string().into_bytes())).await;
                conn.send_json(&info).await;
            })
        }));

        let state_close = state.clone();
        let control_close = control.clone();
        dc.on_close(Box::new(move || {
            let st = state_close.clone();
            let control = control_close.clone();
            Box::pin(async move {
                // Only the session that owns the current generation may clear
                // the connected flag or release keys; an old engine closing
                // late must not affect a newly established connection.
                if st.peer_generation.load(Ordering::SeqCst) == generation {
                    control.release_all_input().await;
                    st.peer_connected.store(false, Ordering::SeqCst);
                    st.push_event(
                        crate::i18n::t(
                            "device disconnected (data channel closed)",
                            "cihaz ayrıldı (veri kanalı kapandı)",
                        )
                        .to_string(),
                    );
                    log::info!("control data channel closed (session {})", generation);
                }
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

        let input_control = control.clone();
        input_dc.on_message(Box::new(move |msg| {
            let control = input_control.clone();
            Box::pin(async move {
                let data: Bytes = msg.data;
                if let Ok(v) = serde_json::from_slice::<Value>(&data) {
                    control.handle(&v).await;
                }
            })
        }));
        let input_open_dc = Arc::clone(&input_dc);
        let input_info = json!({
            "type": "display_info",
            "width": w,
            "height": h,
        })
        .to_string();
        input_dc.on_open(Box::new(move || {
            let dc = Arc::clone(&input_open_dc);
            let info = input_info.clone();
            Box::pin(async move {
                // Güvenilir kanaldan da gönder: imleç modu asla kaybolmasın.
                let _ = dc.send(&Bytes::from(info.into_bytes())).await;
                log::info!("reliable input data channel opened");
            })
        }));
        let input_close_control = control.clone();
        let input_close_state = state.clone();
        input_dc.on_close(Box::new(move || {
            let control = input_close_control.clone();
            let st = input_close_state.clone();
            Box::pin(async move {
                if st.peer_generation.load(Ordering::SeqCst) == generation {
                    control.release_all_input().await;
                }
                log::info!("reliable input data channel closed; keyboard falls back to control");
            })
        }));

        let engine = Arc::new(Self {
            pc,
            video_track,
            data_channel: dc,
            input_channel: input_dc,
            connection,
            control,
            state,
            width,
            height,
            fps,
            generation,
            last_sample: tokio::sync::Mutex::new(None),
            pending_ice: tokio::sync::Mutex::new(Vec::new()),
            closed: AtomicBool::new(false),
        });
        engine.spawn_ping_loop();
        Ok(engine)
    }

    /// Builds the SDP offer and sends it to the phone. Called once per session
    /// right after the phone's `hello`.
    pub async fn create_offer(&self) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            anyhow::bail!("engine closed");
        }
        log::info!("WebRTC: creating offer (session {})", self.generation);
        let offer = self.pc.create_offer(None).await?;
        self.pc.set_local_description(offer).await?;
        if let Some(local) = self.pc.local_description().await {
            log::debug!("local offer SDP:\n{}", local.sdp);
            self.connection
                .send_json(&json!({ "type": "offer", "sdp": local.sdp }))
                .await;
        }
        Ok(())
    }

    pub async fn set_answer(&self, sdp: &str) {
        log::info!("WebRTC: received answer, setting remote description");
        log::debug!("remote answer SDP:\n{}", sdp);
        match RTCSessionDescription::answer(sdp.to_string()) {
            Ok(ans) => match self.pc.set_remote_description(ans).await {
                Ok(()) => {
                    let pending: Vec<RTCIceCandidateInit> = {
                        let mut buf = self.pending_ice.lock().await;
                        std::mem::take(&mut *buf)
                    };
                    if !pending.is_empty() {
                        log::info!("WebRTC: {} bekleyen ICE adayı uygulanıyor", pending.len());
                    }
                    for candidate in pending {
                        if let Err(e) = self.pc.add_ice_candidate(candidate).await {
                            log::warn!("buffered add_ice_candidate failed: {}", e);
                        }
                    }
                }
                Err(e) => log::warn!("set_remote_description failed: {}", e),
            },
            Err(e) => log::warn!("invalid answer: {}", e),
        }
    }

    pub async fn add_ice_candidate(
        &self,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    ) {
        let init = RTCIceCandidateInit {
            candidate,
            sdp_mid,
            sdp_mline_index,
            username_fragment: None,
        };
        // Cevap gelmeden gelen adayları kaybetme. Kontrol ve kuyruğa
        // ekleme aynı kilit altında yapılır; set_answer ile yarışırsa aday
        // ya kuyrukta kalır ya da hemen uygulanır.
        let ready: Option<Vec<RTCIceCandidateInit>> = {
            let mut buf = self.pending_ice.lock().await;
            buf.push(init);
            if self.pc.remote_description().await.is_some() {
                Some(std::mem::take(&mut *buf))
            } else {
                None
            }
        };
        if let Some(candidates) = ready {
            for candidate in candidates {
                if let Err(e) = self.pc.add_ice_candidate(candidate).await {
                    log::warn!("add_ice_candidate failed: {}", e);
                }
            }
        }
    }

    /// True while this session can carry video/control. Used by the signaling
    /// supervisor to decide whether a repeated `hello` needs a fresh engine.
    pub fn is_alive(&self) -> bool {
        !self.closed.load(Ordering::SeqCst)
            && self.data_channel.ready_state()
                == webrtc::data_channel::data_channel_state::RTCDataChannelState::Open
            && self.pc.connection_state() == RTCPeerConnectionState::Connected
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub async fn close(&self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        log::info!("WebRTC: closing session {}", self.generation);
        if self.state.peer_generation.load(Ordering::SeqCst) == self.generation {
            self.control.release_all_input().await;
        }
        if let Err(e) = self.pc.close().await {
            log::debug!("peer connection close failed: {}", e);
        }
    }

    /// Sends `{"type":"ping","t":<epoch_ms>}` every 5s while the control
    /// DataChannel is open; the mobile peer answers with `pong` and the
    /// desktop computes RTT into `AppState.ping_ms`.
    fn spawn_ping_loop(self: &Arc<Self>) {
        let engine = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if engine.closed.load(Ordering::SeqCst) {
                    break;
                }
                if engine.data_channel.ready_state()
                    != webrtc::data_channel::data_channel_state::RTCDataChannelState::Open
                {
                    continue;
                }
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let msg = json!({ "type": "ping", "t": now }).to_string();
                let ping_channel = if engine.input_channel.ready_state()
                    == webrtc::data_channel::data_channel_state::RTCDataChannelState::Open
                {
                    &engine.input_channel
                } else {
                    &engine.data_channel
                };
                if let Err(e) = ping_channel
                    .send(&bytes::Bytes::from(msg.into_bytes()))
                    .await
                {
                    log::debug!("ping send failed: {}", e);
                }
            }
        });
    }

    /// Push one encoded H.264 access unit (Annex-B) into the outbound video
    /// track. The RTP timestamp advances by the *real* elapsed time, so a
    /// frame-rate mismatch never makes playback run fast/slow or grow latency.
    pub async fn push_video_sample(&self, data: &[u8], _keyframe: bool) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Ok(());
        }
        let nominal = Duration::from_secs_f64(1.0 / self.fps.max(1) as f64);
        let duration = {
            let mut last = self.last_sample.lock().await;
            let elapsed = last.map(|t| t.elapsed()).unwrap_or(nominal);
            *last = Some(Instant::now());
            elapsed.clamp(Duration::from_millis(5), Duration::from_millis(250))
        };
        let sample = Sample {
            data: Bytes::from(data.to_vec()),
            timestamp: SystemTime::now(),
            duration,
            packet_timestamp: 0,
            prev_dropped_packets: 0,
            prev_padding_packets: 0,
        };
        self.video_track.write_sample(&sample).await?;
        Ok(())
    }
}

/// Şu anki zaman (epoch saniye, GUI'deki bağlı-cihaz zamanları için).
fn now_epoch() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}
