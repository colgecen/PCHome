package com.PCHome.mobile;

import org.json.JSONException;
import org.json.JSONObject;
import org.webrtc.AudioSource;
import org.webrtc.AudioTrack;
import org.webrtc.DataChannel;
import org.webrtc.IceCandidate;
import org.webrtc.MediaConstraints;
import org.webrtc.MediaStream;
import org.webrtc.PeerConnection;
import org.webrtc.PeerConnectionFactory;
import org.webrtc.SdpObserver;
import org.webrtc.SessionDescription;
import org.webrtc.SurfaceViewRenderer;
import org.webrtc.VideoCapturer;
import org.webrtc.VideoSource;
import org.webrtc.VideoTrack;

import android.content.Context;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import org.webrtc.EglBase;

import java.util.ArrayList;
import java.util.List;

public class WebRtcClient {
    private static final String TAG = "WebRtcClient";

    public enum ConnectionState {
        NEW, CONNECTING, CONNECTED, DISCONNECTED, FAILED
    }

    private PeerConnectionFactory factory;
    private PeerConnection peerConnection;
    private EglBase eglBase;
    private VideoCapturer videoCapturer;
    private VideoSource videoSource;
    private VideoTrack localVideoTrack;
    private AudioSource audioSource;
    private AudioTrack localAudioTrack;
    private SurfaceViewRenderer localRenderer;
    private SurfaceViewRenderer remoteRenderer;
    private DataChannel dataChannel;
    private SignalClient signalClient;
    private ConnectionState state;
    private String localSdp;
    private final List<IceCandidate> iceCandidates = new ArrayList<>();
    private final List<IceCandidate> localIceCandidates = new ArrayList<>();
    private volatile boolean answerSent = false;
    private WebRtcListener listener;
    private Context appContext;
    // Native kitaplık bir süreçte bir kez ilklendirilir; factory/EGL ise
    // her tam kopuşta bırakılıp yeniden kurulur (sızıntı çökertiyordu).
    private static boolean sLibInitialized = false;
    // Sinyal dalgalanmasında el sıkışmayı öldürmemek için: medya tek kez
    // başlatılır; PeerConnection yalnız P2P gerçekten öldüğünde yeniden kurulur.
    private boolean mediaStarted = false;
    private volatile boolean offerReceived = false;
    private volatile int helloTries = 0;
    private volatile boolean closing = false;
    private volatile boolean authReceived = false;
    private volatile boolean p2pConnected = false;
    private volatile int recoverTries = 0;
    private static final int MAX_HELLO_TRIES = 4;
    private static final long HELLO_RETRY_MS = 8000;
    private static final int MAX_JOIN_TRIES = 20;
    private static final long JOIN_RETRY_MS = 3000;
    private static final int MAX_RECOVER_TRIES = 5;
    private static final long RECOVER_DELAY_MS = 5000;
    private Thread joinWatchdog;
    private volatile int pcGeneration = 0;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Runnable recoveryRunnable = this::recoverPeer;

    public interface WebRtcListener {
        void onStateChanged(ConnectionState state);
        void onRemoteTrack(MediaStream stream);
        void onDataChannelMessage(DataChannel.Buffer buffer);
        void onError(String error);
        void onDeviceInfo(JSONObject info);
        void onDisplayInfo(int width, int height);
    }

    public WebRtcClient(Context context, SurfaceViewRenderer localRenderer, SurfaceViewRenderer remoteRenderer) {
        this.appContext = context.getApplicationContext();
        this.localRenderer = localRenderer;
        this.remoteRenderer = remoteRenderer;
        this.state = ConnectionState.NEW;
        initPeerConnectionFactory(appContext);
    }

    private void initPeerConnectionFactory(Context context) {
        if (!sLibInitialized) {
            PeerConnectionFactory.InitializationOptions initOptions = PeerConnectionFactory.InitializationOptions.builder(context)
                    .setEnableInternalTracer(true)
                    .createInitializationOptions();
            PeerConnectionFactory.initialize(initOptions);
            sLibInitialized = true;
        }

        if (eglBase == null) {
            eglBase = EglBase.create();
        }
        if (factory == null) {
            factory = PeerConnectionFactory.builder()
                    .setVideoEncoderFactory(new org.webrtc.DefaultVideoEncoderFactory(eglBase.getEglBaseContext(), true, true))
                    .setVideoDecoderFactory(new org.webrtc.DefaultVideoDecoderFactory(eglBase.getEglBaseContext()))
                    .createPeerConnectionFactory();
        }
    }

    /**
     * Sinyal sunucusuna kalıcı PC kimliğiyle bağlanır. Akış:
     * join (otomatik) -&gt; auth_ok beklenir -&gt; hello -&gt; offer/answer.
     * Şifre reddedilirse yeniden deneme yapılmaz, hata dinleyiciye iletilir.
     */
    public void connect(String signalUrl, String pcId, String password) {
        closing = false;
        authReceived = false;
        p2pConnected = false;
        recoverTries = 0;
        state = ConnectionState.CONNECTING;
        notifyStateChanged();
        Diag.phase(appContext, "bağlan-başladı pc=" + pcId);
        handler.removeCallbacks(recoveryRunnable);
        // Yeniden denemede fabrika bırakılmış olabilir; gerekirse kur.
        if (factory == null || eglBase == null) {
            try {
                initPeerConnectionFactory(appContext);
            } catch (Exception e) {
                Log.e(TAG, "Factory kurulamadı: " + e.getMessage());
                state = ConnectionState.FAILED;
                notifyStateChanged();
                if (listener != null) listener.onError("Medya başlatılamadı: " + e.getMessage());
                return;
            }
        }
        mediaStarted = false;
        offerReceived = false;
        helloTries = 0;
        iceCandidates.clear();
        stopJoinWatchdog();

        String manufacturer = Build.MANUFACTURER == null ? "" : Build.MANUFACTURER.trim();
        String model = Build.MODEL == null ? "" : Build.MODEL.trim();
        String phoneName = model.isEmpty()
                ? manufacturer
                : (model.toLowerCase(java.util.Locale.ROOT)
                        .startsWith(manufacturer.toLowerCase(java.util.Locale.ROOT))
                    ? model
                    : (manufacturer + " " + model).trim());
        String phoneOs = "Android " + (Build.VERSION.RELEASE == null ? "" : Build.VERSION.RELEASE);
        signalClient = new SignalClient(signalUrl, pcId, password,
                new SignalClient.SignalListener() {
            @Override
            public void onConnected() {
                Diag.phase(appContext, "sinyal-açık");
                // Sinyal koptu-yeniden bağlandıysa medyaya DOKUNMA; join
                // otomatik gitti, hello auth_ok ile gidecek.
                if (!mediaStarted) {
                    mediaStarted = true;
                    startLocalMedia();
                    // PeerConnection, auth_ok sonrasında (join kabul edilince)
                    // temiz olarak kurulur.
                    Diag.phase(appContext, "medya-hazır");
                }
                startJoinWatchdog();
            }

            @Override
            public void onDisconnected() {
                Diag.phase(appContext, "sinyal-kapandı");
                state = ConnectionState.DISCONNECTED;
                notifyStateChanged();
                // P2P sağlıklıysa sinyal kopması görüntüyü etkilemez; yalnız
                // P2P de öldüyse kurtarma planla.
                if (!isP2pHealthy()) {
                    scheduleRecovery("sinyal-koptu", 3000);
                }
            }

            @Override
            public void onMessage(JSONObject message) {
                handleSignalMessage(message);
            }

            @Override
            public void onError(String error) {
                Diag.phase(appContext, "sinyal-hata: " + error);
                state = ConnectionState.FAILED;
                notifyStateChanged();
                if (listener != null) listener.onError(error);
                if (!isP2pHealthy()) {
                    scheduleRecovery("sinyal-hata", 5000);
                }
            }
        }, phoneName, phoneOs, BuildConfig.VERSION_NAME);
        signalClient.connect();
    }

    private void startLocalMedia() {
        MediaConstraints constraints = new MediaConstraints();
        constraints.mandatory.add(new MediaConstraints.KeyValuePair("maxWidth", "1920"));
        constraints.mandatory.add(new MediaConstraints.KeyValuePair("maxHeight", "1080"));
        constraints.mandatory.add(new MediaConstraints.KeyValuePair("maxFrameRate", "30"));

        videoCapturer = createVideoCapturer();
        if (videoCapturer != null) {
            videoSource = factory.createVideoSource(videoCapturer.isScreencast());
            videoCapturer.initialize(
                    org.webrtc.SurfaceTextureHelper.create("CaptureThread", eglBase.getEglBaseContext()),
                    null,
                    videoSource.getCapturerObserver()
            );
            videoCapturer.startCapture(1920, 1080, 30);
            localVideoTrack = factory.createVideoTrack("ARDAMSv0", videoSource);
            if (localRenderer != null) {
                localVideoTrack.addSink(localRenderer);
            }
        }

        // Only capture a local audio track when we actually have a media
        // projection (e.g. mirroring the phone). For pure remote-control of the
        // desktop we are a receiver and must NOT capture the mic.
        if (MediaProjectionHolder.hasProjection()) {
            audioSource = factory.createAudioSource(new MediaConstraints());
            localAudioTrack = factory.createAudioTrack("ARDAMSa0", audioSource);
        }
    }

    private VideoCapturer createVideoCapturer() {
        if (!MediaProjectionHolder.hasProjection()) {
            // No screen capture permission available: this peer only receives
            // a remote stream (e.g. the Desktop client) and should not capture.
            return null;
        }
        return new org.webrtc.ScreenCapturerAndroid(
                MediaProjectionHolder.getData(),
                new android.media.projection.MediaProjection.Callback() {
                    @Override
                    public void onStop() {
                        Log.d(TAG, "Screen capture stopped");
                    }
                });
    }

    private void createPeerConnection() {
        final int gen = ++pcGeneration;
        List<PeerConnection.IceServer> iceServers = new ArrayList<>();
        iceServers.add(PeerConnection.IceServer.builder("stun:stun.l.google.com:19302").createIceServer());

        PeerConnection.RTCConfiguration config = new PeerConnection.RTCConfiguration(iceServers);
        config.sdpSemantics = PeerConnection.SdpSemantics.UNIFIED_PLAN;

        peerConnection = factory.createPeerConnection(config, new PeerConnection.Observer() {
            @Override
            public void onIceCandidate(IceCandidate candidate) {
                if (gen != pcGeneration) return;
                Log.i(TAG, "onIceCandidate: " + candidate.sdp);
                // Answer gitmeden aday gönderme: masaüstü remote description
                // olmadan adayı reddedip bağlantıyı kaçırabiliyordu.
                if (!answerSent) {
                    localIceCandidates.add(candidate);
                    return;
                }
                sendLocalIceCandidate(candidate);
            }

            @Override
            public void onAddStream(MediaStream stream) {
                if (gen == pcGeneration && listener != null) listener.onRemoteTrack(stream);
            }

            @Override
            public void onTrack(org.webrtc.RtpTransceiver transceiver) {
                if (gen != pcGeneration) return;
                org.webrtc.MediaStreamTrack track = transceiver.getReceiver().track();
                Log.i(TAG, "onTrack fired: " + (track != null ? track.kind() : "null"));
                if (track instanceof VideoTrack) {
                    Diag.phase(appContext, "video-kare");
                    VideoTrack vt = (VideoTrack) track;
                    if (remoteRenderer != null) {
                        vt.addSink(remoteRenderer);
                    }
                    if (listener != null) {
                        listener.onRemoteTrack(null);
                    }
                }
            }

            @Override
            public void onDataChannel(DataChannel dc) {
                if (gen != pcGeneration) return;
                Diag.phase(appContext, "veri-kanalı");
                Log.i(TAG, "onDataChannel: label=" + dc.label() + " state=" + dc.state());
                final DataChannel mine = dc;
                dataChannel = dc;
                dataChannel.registerObserver(new DataChannel.Observer() {
                    @Override
                    public void onMessage(DataChannel.Buffer buffer) {
                        // answerPing reads its own duplicate; the listener must
                        // still see the original bytes (previously the buffer
                        // position was consumed and display_info arrived empty).
                        answerPing(new DataChannel.Buffer(buffer.data.duplicate(), buffer.binary));
                        if (listener != null) {
                            listener.onDataChannelMessage(
                                    new DataChannel.Buffer(buffer.data.duplicate(), buffer.binary));
                        }
                    }

                    @Override
                    public void onStateChange() {
                        if (gen != pcGeneration) return;
                        Log.i(TAG, "DataChannel state=" + mine.state());
                        if (mine.state() == DataChannel.State.OPEN) {
                            p2pConnected = true;
                            recoverTries = 0;
                            handler.removeCallbacks(recoveryRunnable);
                            state = ConnectionState.CONNECTED;
                            notifyStateChanged();
                        } else if (mine.state() == DataChannel.State.CLOSED) {
                            p2pConnected = false;
                            scheduleRecovery("veri-kanalı-kapandı", 1000);
                        }
                    }

                    @Override
                    public void onBufferedAmountChange(long previousAmount) {}
                });
            }

            @Override
            public void onIceConnectionChange(PeerConnection.IceConnectionState newState) {
                if (gen != pcGeneration) return;
                Diag.phase(appContext, "ice-durum: " + newState);
                if (newState == PeerConnection.IceConnectionState.CONNECTED) {
                    onTransportConnected();
                } else if (newState == PeerConnection.IceConnectionState.DISCONNECTED) {
                    p2pConnected = false;
                    state = ConnectionState.DISCONNECTED;
                    notifyStateChanged();
                    scheduleRecovery("ice-disconnected", 4000);
                } else if (newState == PeerConnection.IceConnectionState.FAILED) {
                    p2pConnected = false;
                    state = ConnectionState.FAILED;
                    notifyStateChanged();
                    scheduleRecovery("ice-failed", 1500);
                }
            }

            @Override
            public void onConnectionChange(PeerConnection.PeerConnectionState newState) {
                if (gen != pcGeneration) return;
                Diag.phase(appContext, "bağlantı-durumu: " + newState);
                if (newState == PeerConnection.PeerConnectionState.CONNECTED) {
                    onTransportConnected();
                } else if (newState == PeerConnection.PeerConnectionState.DISCONNECTED) {
                    p2pConnected = false;
                    scheduleRecovery("p2p-disconnected", 4000);
                } else if (newState == PeerConnection.PeerConnectionState.FAILED) {
                    p2pConnected = false;
                    scheduleRecovery("p2p-failed", 1500);
                }
            }

            @Override
            public void onIceGatheringChange(PeerConnection.IceGatheringState newState) {
                Log.d(TAG, "ICE gathering state: " + newState);
            }

            @Override
            public void onSignalingChange(PeerConnection.SignalingState newState) {
                Log.d(TAG, "Signaling state: " + newState);
            }

            @Override
            public void onIceConnectionReceivingChange(boolean receiving) {}
            @Override public void onAddTrack(org.webrtc.RtpReceiver receiver, org.webrtc.MediaStream[] streams) {}
            @Override public void onIceCandidatesRemoved(IceCandidate[] candidates) {}
            @Override public void onRemoveStream(MediaStream stream) {}
            @Override public void onRenegotiationNeeded() {}
        });

        // Recv-only transceiver is sufficient: this peer only renders the
        // desktop's outbound H.264 track and never captures local media
        // (Flow B was removed). Adding an empty local MediaStream would emit
        // extra m-lines and break SDP negotiation with the desktop offerer.
        try {
            peerConnection.addTransceiver(
                    org.webrtc.MediaStreamTrack.MediaType.MEDIA_TYPE_VIDEO,
                    new org.webrtc.RtpTransceiver.RtpTransceiverInit(
                            org.webrtc.RtpTransceiver.RtpTransceiverDirection.RECV_ONLY));
        } catch (Exception e) {
            Log.e(TAG, "addTransceiver failed", e);
        }
    }

    private void onTransportConnected() {
        if (closing) {
            return;
        }
        p2pConnected = true;
        recoverTries = 0;
        handler.removeCallbacks(recoveryRunnable);
        state = ConnectionState.CONNECTED;
        notifyStateChanged();
    }

    private void createAnswer() {
        if (peerConnection == null) {
            return;
        }
        peerConnection.createAnswer(new SdpObserver() {
            @Override
            public void onCreateSuccess(SessionDescription sessionDescription) {
                localSdp = sessionDescription.description;
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "local answer SDP:\n" + sessionDescription.description);
                }
                peerConnection.setLocalDescription(new SdpObserver() {
                    @Override
                    public void onSetSuccess() {
                        Diag.phase(appContext, "cevap-gönderildi");
                        try {
                            JSONObject msg = new JSONObject();
                            msg.put("type", "answer");
                            msg.put("sdp", sessionDescription.description);
                            if (signalClient != null) {
                                signalClient.sendMessage(msg);
                            }
                        } catch (JSONException e) {
                            Log.e(TAG, "Failed to send answer", e);
                        }
                        answerSent = true;
                        for (IceCandidate candidate : localIceCandidates) {
                            sendLocalIceCandidate(candidate);
                        }
                        localIceCandidates.clear();
                    }

                    @Override public void onCreateSuccess(SessionDescription s) {}
                    @Override public void onSetFailure(String s) { Log.e(TAG, "Set local desc failed: " + s); }
                    @Override public void onCreateFailure(String s) {}
                }, sessionDescription);
            }

            @Override public void onSetSuccess() {}
            @Override public void onCreateFailure(String s) { Log.e(TAG, "Create answer failed: " + s); }
            @Override public void onSetFailure(String s) { Log.e(TAG, "Set local desc failed: " + s); }
        }, new MediaConstraints());
    }

    private void setRemoteDescription(JSONObject sdp) {
        try {
            String type = sdp.getString("type");
            String description = sdp.getString("sdp");
            SessionDescription sessionDescription = new SessionDescription(
                    SessionDescription.Type.fromCanonicalForm(type),
                    description
            );
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "remote " + type + " SDP:\n" + description);
            }
            peerConnection.setRemoteDescription(new SdpObserver() {
                @Override public void onSetSuccess() {
                    if (type.equals("answer")) {
                        drainIceCandidates();
                    } else if (type.equals("offer")) {
                        // Answerer tarafı da offer uygulandıktan sonra
                        // masaüstünden gelen adayları boşaltmalı; aksi
                        // hâlde adaylar sonsuza kadar kuyrukta kalıyordu.
                        drainIceCandidates();
                        createAnswer();
                    }
                }
                @Override public void onCreateSuccess(SessionDescription sessionDescription) {}
                @Override public void onSetFailure(String s) { Log.e(TAG, "Set remote desc failed: " + s); }
                @Override public void onCreateFailure(String s) {}
            }, sessionDescription);
        } catch (JSONException e) {
            Log.e(TAG, "Failed to parse remote SDP", e);
        }
    }

    private void sendHello() {
        if (closing || signalClient == null || !signalClient.isConnected()) {
            return;
        }
        try {
            JSONObject hello = new JSONObject();
            hello.put("type", "hello");
            hello.put("role", "mobile");
            boolean sent = signalClient.sendMessage(hello);
            Log.i(TAG, "hello sent to server: " + sent);
            Diag.phase(appContext, "hello-gönderildi");
            if (sent) {
                helloTries++;
                scheduleHelloRetry();
            }
        } catch (JSONException e) {
            Log.e(TAG, "Failed to send hello", e);
        }
    }

    /**
     * Offer 8 sn içinde gelmezse hello'yu en fazla 4 kez yinele.
     * (Sinyal/karşı taraf yarışı kaybederse el sıkışma sonsuza takılmaz.)
     */
    private void scheduleHelloRetry() {
        new Thread(() -> {
            try {
                Thread.sleep(HELLO_RETRY_MS);
            } catch (InterruptedException e) {
                return;
            }
            if (!closing && !offerReceived && helloTries < MAX_HELLO_TRIES
                    && signalClient != null && signalClient.isConnected()
                    && !isP2pHealthy()) {
                Log.i(TAG, "Offer gelmedi, hello yineleniyor (" + helloTries + ")");
                sendHello();
            }
        }).start();
    }

    /**
     * Sinyal açıldıktan sonra auth_ok gelmezse join'i yineler. Mobil önce
     * bağlanıp masaüstü sonra gelirse ilk join kaybolabiliyordu.
     */
    private void startJoinWatchdog() {
        stopJoinWatchdog();
        joinWatchdog = new Thread(() -> {
            for (int i = 0; i < MAX_JOIN_TRIES && !closing && !authReceived; i++) {
                try {
                    Thread.sleep(JOIN_RETRY_MS);
                } catch (InterruptedException e) {
                    return;
                }
                if (closing || authReceived || signalClient == null || !signalClient.isConnected()) {
                    continue;
                }
                Log.i(TAG, "auth_ok gelmedi, join yineleniyor (" + (i + 1) + ")");
                signalClient.sendJoin();
            }
            if (!closing && !authReceived && listener != null) {
                listener.onError("PC'den kimlik yanıtı gelmedi — PC uygulaması açık mı?");
            }
        });
        joinWatchdog.start();
    }

    private void stopJoinWatchdog() {
        if (joinWatchdog != null) {
            joinWatchdog.interrupt();
            joinWatchdog = null;
        }
    }

    private void handleSignalMessage(JSONObject message) {
        try {
            String type = message.getString("type");
            switch (type) {
                case "auth_ok":
                    Log.i(TAG, "Kimlik doğrulandı");
                    Diag.phase(appContext, "auth-ok");
                    authReceived = true;
                    stopJoinWatchdog();
                    if (listener != null) listener.onDeviceInfo(message);
                    // P2P zaten sağlıklıysa hello gönderme: masaüstünün yeni
                    // bir oturum kurmasına ve çalışan görüntüyü kesmesine yol
                    // açma. Sağlıklı değilse temiz bir PeerConnection kur.
                    if (!isP2pHealthy()) {
                        // Ölü bir PeerConnection'a yeni offer uygulanamaz;
                        // her yeni el sıkışmasında temiz bir tane kur.
                        resetPeerConnection();
                        offerReceived = false;
                        helloTries = 0;
                        sendHello();
                    }
                    // Cihaz kartı + çözünürlük bilgisini de iste (yedeklilik).
                    try {
                        JSONObject req = new JSONObject();
                        req.put("type", "get_device_info");
                        if (signalClient != null) signalClient.sendMessage(req);
                    } catch (JSONException ignore) {
                    }
                    break;
                case "auth_rejected":
                    String reason = message.optString("reason", "şifre hatalı");
                    Log.w(TAG, "Kimlik reddedildi: " + reason);
                    Diag.phase(appContext, "auth-red: " + reason);
                    authReceived = true;
                    stopJoinWatchdog();
                    handler.removeCallbacks(recoveryRunnable);
                    if (signalClient != null) {
                        signalClient.setAuthFailed();
                        signalClient.disconnect();
                    }
                    state = ConnectionState.FAILED;
                    notifyStateChanged();
                    if (listener != null) listener.onError("Bağlantı reddedildi: " + reason);
                    break;
                case "device_info":
                    if (listener != null) listener.onDeviceInfo(message);
                    break;
                case "display_info":
                    if (listener != null) {
                        listener.onDisplayInfo(
                                message.optInt("width", 1920),
                                message.optInt("height", 1080));
                    }
                    break;
                case "offer":
                    offerReceived = true;
                    Diag.phase(appContext, "offer-alındı");
                    if (peerConnection != null) {
                        setRemoteDescription(message);
                        // Answer/ICE bir sebeple tamamlanmazsa oturumu
                        // sıfırdan kur; kullanıcı kara ekranda kalmasın.
                        scheduleRecovery("offer-cevap-zaman-aşımı", 12000);
                    }
                    break;
                case "answer":
                    if (peerConnection != null) {
                        setRemoteDescription(message);
                    }
                    break;
                case "ice-candidate":
                    if (peerConnection == null) {
                        break;
                    }
                    String sdp = message.getString("candidate");
                    String sdpMid = message.getString("sdpMid");
                    int sdpMLineIndex = message.getInt("sdpMLineIndex");
                    IceCandidate candidate = new IceCandidate(sdpMid, sdpMLineIndex, sdp);
                    if (peerConnection.getRemoteDescription() != null) {
                        peerConnection.addIceCandidate(candidate);
                    } else {
                        iceCandidates.add(candidate);
                    }
                    break;
                default:
                    break;
            }
        } catch (JSONException e) {
            Log.e(TAG, "Failed to handle signal message", e);
        }
    }

    private void sendLocalIceCandidate(IceCandidate candidate) {
        try {
            JSONObject msg = new JSONObject();
            msg.put("type", "ice-candidate");
            msg.put("candidate", candidate.sdp);
            msg.put("sdpMid", candidate.sdpMid);
            msg.put("sdpMLineIndex", candidate.sdpMLineIndex);
            if (signalClient != null) signalClient.sendMessage(msg);
        } catch (JSONException e) {
            Log.e(TAG, "Failed to send ICE candidate", e);
        }
    }

    private void drainIceCandidates() {
        if (peerConnection == null) {
            iceCandidates.clear();
            return;
        }
        for (IceCandidate candidate : iceCandidates) {
            peerConnection.addIceCandidate(candidate);
        }
        iceCandidates.clear();
    }

    /// Reply to desktop RTT probes: `{"type":"ping","t":<epoch_ms>}` gets a
    /// `{"type":"pong","t":<same t>}` back over the same control channel.
    private void answerPing(DataChannel.Buffer buffer) {
        try {
            if (buffer.data == null) return;
            byte[] raw = new byte[buffer.data.remaining()];
            buffer.data.get(raw);
            JSONObject msg = new JSONObject(new String(raw, java.nio.charset.StandardCharsets.UTF_8));
            if ("ping".equals(msg.optString("type"))) {
                JSONObject pong = new JSONObject();
                pong.put("type", "pong");
                pong.put("t", msg.optLong("t"));
                sendControl(pong.toString());
            }
        } catch (Exception e) {
            Log.d(TAG, "answerPing ignored non-JSON message");
        }
    }

    /**
     * Elle görüntü yenileme: PeerConnection'ı sıfırdan kurup yeni offer ister.
     * Masaüstü her hello'da taze oturum açar; donan video burada kurtarılır.
     */
    public void refreshStream() {
        if (closing) {
            return;
        }
        Diag.phase(appContext, "elle-yenile");
        handler.removeCallbacks(recoveryRunnable);
        recoverTries = 0;
        offerReceived = false;
        helloTries = 0;
        state = ConnectionState.CONNECTING;
        notifyStateChanged();
        resetPeerConnection();
        startReauth();
    }

    private boolean isP2pHealthy() {
        return !closing
                && p2pConnected
                && peerConnection != null
                && dataChannel != null
                && dataChannel.state() == DataChannel.State.OPEN;
    }

    private void scheduleRecovery(String reason, long delayMs) {
        if (closing) {
            return;
        }
        Log.i(TAG, "kurtarma planlandı: " + reason + " (" + delayMs + "ms)");
        handler.removeCallbacks(recoveryRunnable);
        handler.postDelayed(recoveryRunnable, delayMs);
    }

    private void resetPeerConnection() {
        try {
            if (peerConnection != null) {
                peerConnection.close();
            }
        } catch (Exception e) {
            Log.w(TAG, "eski PeerConnection kapatılamadı: " + e.getMessage());
        }
        peerConnection = null;
        dataChannel = null;
        p2pConnected = false;
        answerSent = false;
        iceCandidates.clear();
        localIceCandidates.clear();
        if (factory != null && !closing) {
            createPeerConnection();
        }
    }

    /** ICE/DTLS öldüyse PeerConnection'ı sıfırdan kurup yeni offer ister. */
    private void recoverPeer() {
        if (closing || isP2pHealthy()) {
            return;
        }
        if (recoverTries >= MAX_RECOVER_TRIES) {
            if (listener != null) {
                listener.onError("Görüntü bağlantısı kurulamadı (birkaç deneme)");
            }
            return;
        }
        recoverTries++;
        Log.i(TAG, "PeerConnection yeniden kuruluyor (" + recoverTries + "/" + MAX_RECOVER_TRIES + ")");
        Diag.phase(appContext, "p2p-kurtarma-" + recoverTries);
        resetPeerConnection();
        offerReceived = false;
        helloTries = 0;
        // Masaüstü yeniden başladıysa sinyal oturumu hâlâ açık olabilir ama
        // masaüstü tarafındaki kimlik durumu sıfırlanmıştır. Önce join ile
        // yeniden doğrula; auth_ok gelince hello kendiliğinden gider.
        startReauth();
        handler.postDelayed(recoveryRunnable, RECOVER_DELAY_MS);
    }

    /** Sinyal oturumu açıkken kimliği tazeler ve join'i yineler. */
    private void startReauth() {
        authReceived = false;
        if (signalClient != null && signalClient.isConnected()) {
            signalClient.sendJoin();
            startJoinWatchdog();
        }
        // Sinyal kapalıysa SignalClient kendi yeniden bağlanışında join atar;
        // o sırada onConnected join gözcüsünü de başlatır.
    }

    public void disconnect() {
        closing = true;
        authReceived = true;
        pcGeneration++; // eski PeerConnection callback'lerini geçersiz kıl
        stopJoinWatchdog();
        handler.removeCallbacks(recoveryRunnable);
        Diag.phase(appContext, "kapatıldı");
        try {
            if (videoCapturer != null) {
                try {
                    videoCapturer.stopCapture();
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                }
            }
        } catch (Exception e) {
            Log.w(TAG, "Capturer durdurulamadı: " + e.getMessage());
        }
        try {
            if (peerConnection != null) {
                peerConnection.close();
            }
        } catch (Exception e) {
            Log.w(TAG, "PeerConnection kapatılamadı: " + e.getMessage());
        }
        try {
            if (signalClient != null) {
                signalClient.disconnect();
            }
        } catch (Exception e) {
            Log.w(TAG, "Signal kapatılamadı: " + e.getMessage());
        }
        // Medya kaynaklarını bırak: bırakılmazsa EGL bağlamları birikip
        // birkaç denemede uygulama çöküyordu ("başa atma").
        try {
            if (localVideoTrack != null) {
                localVideoTrack.dispose();
            }
        } catch (Exception e) {
            Log.w(TAG, "VideoTrack bırakılamadı: " + e.getMessage());
        }
        try {
            if (videoSource != null) {
                videoSource.dispose();
            }
        } catch (Exception e) {
            Log.w(TAG, "VideoSource bırakılamadı: " + e.getMessage());
        }
        try {
            if (localAudioTrack != null) {
                localAudioTrack.dispose();
            }
        } catch (Exception e) {
            Log.w(TAG, "AudioTrack bırakılamadı: " + e.getMessage());
        }
        try {
            if (audioSource != null) {
                audioSource.dispose();
            }
        } catch (Exception e) {
            Log.w(TAG, "AudioSource bırakılamadı: " + e.getMessage());
        }
        try {
            if (factory != null) {
                factory.dispose();
            }
        } catch (Exception e) {
            Log.w(TAG, "Factory bırakılamadı: " + e.getMessage());
        }
        peerConnection = null;
        dataChannel = null;
        localVideoTrack = null;
        videoSource = null;
        videoCapturer = null;
        localAudioTrack = null;
        audioSource = null;
        factory = null;
        // NOT: eglBase burada BIRAKILMAZ. SurfaceViewRenderer aynı EGL
        // bağlamını paylaşır; yeniden bağlanmada da aynı kök bağlam kullanılır.
        // Aktivite kapanırken releaseGlobals() ile bırakılır.
        mediaStarted = false;
        p2pConnected = false;
        state = ConnectionState.DISCONNECTED;
        notifyStateChanged();
    }

    /** SurfaceViewRenderer'ın factory ile aynı EGL bağlamını paylaşması için. */
    public EglBase.Context getEglBaseContext() {
        return eglBase == null ? null : eglBase.getEglBaseContext();
    }

    /** Aktivite kapanırken EGL kök bağlamını bırakır (disconnect'ten sonra). */
    public void releaseGlobals() {
        try {
            if (eglBase != null) {
                eglBase.release();
            }
        } catch (Exception e) {
            Log.w(TAG, "EGL bırakılamadı: " + e.getMessage());
        }
        eglBase = null;
    }

    public ConnectionState getState() {
        return state;
    }

    public void setListener(WebRtcListener listener) {
        this.listener = listener;
    }

    /// Send a UTF-8 JSON control message over the `control` DataChannel
    /// (unordered/unreliable). No buffering: the bytes are transmitted
    /// immediately for lowest input latency.
    public void sendControl(String json) {
        if (dataChannel != null && dataChannel.state() == DataChannel.State.OPEN) {
            byte[] bytes = json.getBytes(java.nio.charset.StandardCharsets.UTF_8);
            DataChannel.Buffer buffer = new DataChannel.Buffer(
                    java.nio.ByteBuffer.wrap(bytes), false);
            dataChannel.send(buffer);
        } else {
            Log.d(TAG, "sendControl ignored (channel="
                    + (dataChannel == null ? "null" : dataChannel.state()) + "): " + json);
        }
    }

    private void notifyStateChanged() {
        if (listener != null) {
            listener.onStateChanged(state);
        }
    }
}
