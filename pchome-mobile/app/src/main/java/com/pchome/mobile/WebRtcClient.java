package com.pchome.mobile;

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
import android.util.Log;
import android.view.SurfaceViewRenderer;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class WebRtcClient {
    private static final String TAG = "WebRtcClient";

    public enum ConnectionState {
        NEW, CONNECTING, CONNECTED, DISCONNECTED, FAILED
    }

    private PeerConnectionFactory factory;
    private PeerConnection peerConnection;
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
    private WebRtcListener listener;

    public interface WebRtcListener {
        void onStateChanged(ConnectionState state);
        void onRemoteTrack(MediaStream stream);
        void onDataChannelMessage(DataChannel.Buffer buffer);
        void onError(String error);
    }

    public WebRtcClient(Context context, SurfaceViewRenderer localRenderer, SurfaceViewRenderer remoteRenderer) {
        this.localRenderer = localRenderer;
        this.remoteRenderer = remoteRenderer;
        this.state = ConnectionState.NEW;
        initPeerConnectionFactory(context);
    }

    private void initPeerConnectionFactory(Context context) {
        PeerConnectionFactory.InitializationOptions initOptions = PeerConnectionFactory.InitializationOptions.builder(context)
                .setEnableInternalTracer(true)
                .createInitializationOptions();
        PeerConnectionFactory.initialize(initOptions);

        factory = PeerConnectionFactory.builder()
                .setVideoEncoderFactory(new org.webrtc.DefaultVideoEncoderFactory(null, true, true))
                .setVideoDecoderFactory(new org.webrtc.DefaultVideoDecoderFactory(null))
                .createPeerConnectionFactory();
    }

    public void connect(String signalUrl, String pin, JSONObject sdpOffer) {
        state = ConnectionState.CONNECTING;
        notifyStateChanged();

        signalClient = new SignalClient(signalUrl, pin, new SignalClient.SignalListener() {
            @Override
            public void onConnected() {
                startLocalMedia();
                createPeerConnection();
                if (sdpOffer != null) {
                    setRemoteDescription(sdpOffer);
                } else {
                    createOffer();
                }
            }

            @Override
            public void onDisconnected() {
                state = ConnectionState.DISCONNECTED;
                notifyStateChanged();
            }

            @Override
            public void onMessage(JSONObject message) {
                handleSignalMessage(message);
            }

            @Override
            public void onError(String error) {
                state = ConnectionState.FAILED;
                notifyStateChanged();
                if (listener != null) listener.onError(error);
            }
        });
        signalClient.connect();
    }

    private void startLocalMedia() {
        MediaConstraints constraints = new MediaConstraints();
        constraints.mandatory.add(new MediaConstraints.KeyValuePair("maxWidth", "1920"));
        constraints.mandatory.add(new MediaConstraints.KeyValuePair("maxHeight", "1080"));
        constraints.mandatory.add(new MediaConstraints.KeyValuePair("maxFrameRate", "30"));

        videoCapturer = createVideoCapturer();
        videoSource = factory.createVideoSource(videoCapturer.isScreencast());
        videoCapturer.initialize(
                new org.webrtc.SurfaceTextureHelper("CaptureThread", factory.getInternalVideoEncoderFactory().get().getEglContext()),
                null,
                videoSource.getCapturerObserver()
        );
        videoCapturer.startCapture(1920, 1080, 30);
        localVideoTrack = factory.createVideoTrack("ARDAMSv0", videoSource);
        localVideoTrack.addSink(localRenderer);

        audioSource = factory.createAudioSource(new MediaConstraints());
        localAudioTrack = factory.createAudioTrack("ARDAMSa0", audioSource);
    }

    private VideoCapturer createVideoCapturer() {
        return new org.webrtc.ScreenCapturerAndroid(
                org.webrtc.ScreenCapturerAndroid.SCREEN_DISPLAY_NAME,
                null
        );
    }

    private void createPeerConnection() {
        List<PeerConnection.IceServer> iceServers = new ArrayList<>();
        iceServers.add(PeerConnection.IceServer.builder("stun:stun.l.google.com:19302").createIceServer());

        PeerConnection.RTCConfiguration config = new PeerConnection.RTCConfiguration(iceServers);
        config.sdpSemantics = PeerConnection.SdpSemantics.UNIFIED_PLAN;

        peerConnection = factory.createPeerConnection(config, new PeerConnection.Observer() {
            @Override
            public void onIceCandidate(IceCandidate candidate) {
                try {
                    JSONObject msg = new JSONObject();
                    msg.put("type", "ice-candidate");
                    msg.put("candidate", candidate.sdp);
                    msg.put("sdpMid", candidate.sdpMid);
                    msg.put("sdpMLineIndex", candidate.sdpMLineIndex);
                    signalClient.sendMessage(msg);
                } catch (JSONException e) {
                    Log.e(TAG, "Failed to send ICE candidate", e);
                }
            }

            @Override
            public void onAddStream(MediaStream stream) {
                if (listener != null) listener.onRemoteTrack(stream);
            }

            @Override
            public void onDataChannel(DataChannel dc) {
                dataChannel = dc;
                dataChannel.registerObserver(new DataChannel.Observer() {
                    @Override
                    public void onMessage(DataChannel.Buffer buffer) {
                        if (listener != null) listener.onDataChannelMessage(buffer);
                    }
                });
            }

            @Override
            public void onIceConnectionChange(PeerConnection.IceConnectionState newState) {
                if (newState == PeerConnection.IceConnectionState.CONNECTED) {
                    state = ConnectionState.CONNECTED;
                    notifyStateChanged();
                } else if (newState == PeerConnection.IceConnectionState.DISCONNECTED) {
                    state = ConnectionState.DISCONNECTED;
                    notifyStateChanged();
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

            @Override public void onAddTrack(org.webrtc.RtpReceiver receiver, org.webrtc.MediaStream[] streams) {}
            @Override public void onConnectionChange(PeerConnection.PeerConnectionState newState) {}
            @Override public void onIceCandidatesRemoved(IceCandidate[] candidates) {}
            @Override public void onRemoveStream(MediaStream stream) {}
            @Override public void onRenegotiationNeeded() {}
            @Override public void onTrack(org.webrtc.RtpTransceiver transceiver) {}
        });

        MediaStream stream = factory.createLocalMediaStream("ARDAMS");
        stream.addTrack(localVideoTrack);
        stream.addTrack(localAudioTrack);
        peerConnection.addStream(stream);
    }

    private void createOffer() {
        MediaConstraints constraints = new MediaConstraints();
        constraints.mandatory.add(new MediaConstraints.KeyValuePair("OfferToReceiveAudio", "true"));
        constraints.mandatory.add(new MediaConstraints.KeyValuePair("OfferToReceiveVideo", "true"));

        peerConnection.createOffer(new SdpObserver() {
            @Override
            public void onCreateSuccess(SessionDescription sessionDescription) {
                localSdp = sessionDescription.description;
                peerConnection.setLocalDescription(this, sessionDescription);

                try {
                    JSONObject msg = new JSONObject();
                    msg.put("type", "offer");
                    msg.put("sdp", sessionDescription.description);
                    signalClient.sendMessage(msg);
                } catch (JSONException e) {
                    Log.e(TAG, "Failed to send offer", e);
                }
            }

            @Override public void onSetSuccess() {}
            @Override public void onCreateFailure(String s) { Log.e(TAG, "Create offer failed: " + s); }
            @Override public void onSetFailure(String s) { Log.e(TAG, "Set local desc failed: " + s); }
        }, constraints);
    }

    private void setRemoteDescription(JSONObject sdp) {
        try {
            String type = sdp.getString("type");
            String description = sdp.getString("sdp");
            SessionDescription sessionDescription = new SessionDescription(
                    SessionDescription.Type.fromCanonicalForm(type),
                    description
            );
            peerConnection.setRemoteDescription(new SdpObserver() {
                @Override public void onSetSuccess() {
                    if (type.equals("answer")) {
                        drainIceCandidates();
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

    private void handleSignalMessage(JSONObject message) {
        try {
            String type = message.getString("type");
            switch (type) {
                case "answer":
                    setRemoteDescription(message);
                    break;
                case "ice-candidate":
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
            }
        } catch (JSONException e) {
            Log.e(TAG, "Failed to handle signal message", e);
        }
    }

    private void drainIceCandidates() {
        for (IceCandidate candidate : iceCandidates) {
            peerConnection.addIceCandidate(candidate);
        }
        iceCandidates.clear();
    }

    public void disconnect() {
        if (videoCapturer != null) {
            try {
                videoCapturer.stopCapture();
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
        }
        if (peerConnection != null) {
            peerConnection.close();
        }
        if (signalClient != null) {
            signalClient.disconnect();
        }
        state = ConnectionState.DISCONNECTED;
        notifyStateChanged();
    }

    public ConnectionState getState() {
        return state;
    }

    public void setListener(WebRtcListener listener) {
        this.listener = listener;
    }

    private void notifyStateChanged() {
        if (listener != null) {
            listener.onStateChanged(state);
        }
    }
}
