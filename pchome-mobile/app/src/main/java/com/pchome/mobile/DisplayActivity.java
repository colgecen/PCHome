package com.pchome.mobile;

import android.os.Bundle;
import android.view.SurfaceViewRenderer;
import android.widget.TextView;

import androidx.appcompat.app.AppCompatActivity;

import org.webrtc.MediaStream;
import org.webrtc.SurfaceViewRenderer;
import org.webrtc.VideoTrack;

public class DisplayActivity extends AppCompatActivity implements WebRtcClient.WebRtcListener {
    private SurfaceViewRenderer surfaceView;
    private TextView streamStatus;
    private WebRtcClient webRtcClient;
    private SignalClient signalClient;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_display);

        surfaceView = findViewById(R.id.surface_view);
        streamStatus = findViewById(R.id.stream_status);

        surfaceView.init(null, null);
        surfaceView.setZOrderMediaOverlay(true);

        String pin = getIntent().getStringExtra("pin");
        signalClient = new SignalClient("ws://localhost:8080/ws", pin, new SignalClient.SignalListener() {
            @Override
            public void onConnected() {
                runOnUiThread(() -> streamStatus.setText("Connected, waiting for video..."));
            }

            @Override
            public void onDisconnected() {
                runOnUiThread(() -> streamStatus.setText(R.string.disconnected));
            }

            @Override
            public void onMessage(org.json.JSONObject message) {
                runOnUiThread(() -> handleSignalMessage(message));
            }

            @Override
            public void onError(String error) {
                runOnUiThread(() -> streamStatus.setText(R.string.error + ": " + error));
            }
        });
        signalClient.connect();
    }

    private void handleSignalMessage(org.json.JSONObject message) {
        try {
            String type = message.getString("type");
            if ("offer".equals(type)) {
                webRtcClient = new WebRtcClient(this, null, surfaceView);
                webRtcClient.setListener(this);
                webRtcClient.connect("ws://localhost:8080/ws",
                        getIntent().getStringExtra("pin"),
                        message);
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    @Override
    public void onStateChanged(WebRtcClient.ConnectionState state) {
        runOnUiThread(() -> {
            switch (state) {
                case CONNECTED:
                    streamStatus.setVisibility(View.GONE);
                    break;
                case DISCONNECTED:
                    streamStatus.setText(R.string.disconnected);
                    streamStatus.setVisibility(View.VISIBLE);
                    break;
                case FAILED:
                    streamStatus.setText(R.string.error);
                    streamStatus.setVisibility(View.VISIBLE);
                    break;
            }
        });
    }

    @Override
    public void onRemoteTrack(MediaStream stream) {
        runOnUiThread(() -> {
            streamStatus.setVisibility(View.GONE);
            if (stream.videoTracks.size() > 0) {
                VideoTrack videoTrack = stream.videoTracks.get(0);
                videoTrack.addSink(surfaceView);
            }
        });
    }

    @Override
    public void onDataChannelMessage(org.webrtc.DataChannel.Buffer buffer) {
    }

    @Override
    public void onError(String error) {
        runOnUiThread(() -> streamStatus.setText(R.string.error + ": " + error));
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        if (webRtcClient != null) {
            webRtcClient.disconnect();
        }
        if (signalClient != null) {
            signalClient.disconnect();
        }
        if (surfaceView != null) {
            surfaceView.release();
        }
    }
}
