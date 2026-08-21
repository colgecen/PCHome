package com.pchome.mobile;

import android.content.Intent;
import android.os.Bundle;
import android.view.View;
import android.widget.Button;
import android.widget.TextView;

import androidx.appcompat.app.AppCompatActivity;

import org.json.JSONException;
import org.json.JSONObject;

public class PinActivity extends AppCompatActivity {
    private TextView pinText;
    private TextView statusText;
    private Button connectButton;
    private SignalClient signalClient;
    private WebRtcClient webRtcClient;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_pin);

        pinText = findViewById(R.id.pin);
        statusText = findViewById(R.id.status);
        connectButton = findViewById(R.id.connect_button);

        connectButton.setOnClickListener(v -> {
            if (connectButton.getText().toString().equals(getString(R.string.connect))) {
                connectToDesktop();
            } else {
                disconnect();
            }
        });
    }

    private void connectToDesktop() {
        statusText.setText(R.string.connecting);
        connectButton.setEnabled(false);

        String rawPin = pinText.getText().toString().replaceAll("[^0-9]", "");
        signalClient = new SignalClient("ws://localhost:8080/ws", rawPin,
                new SignalClient.SignalListener() {
                    @Override
                    public void onConnected() {
                        runOnUiThread(() -> {
                            statusText.setText(R.string.connected);
                            connectButton.setText(R.string.disconnect);
                            connectButton.setEnabled(true);
                        });
                    }

                    @Override
                    public void onDisconnected() {
                        runOnUiThread(() -> {
                            statusText.setText(R.string.disconnected);
                            connectButton.setText(R.string.connect);
                            connectButton.setEnabled(true);
                        });
                    }

                    @Override
                    public void onMessage(JSONObject message) {
                        runOnUiThread(() -> handleSignalMessage(message));
                    }

                    @Override
                    public void onError(String error) {
                        runOnUiThread(() -> {
                            statusText.setText(R.string.error + ": " + error);
                            connectButton.setText(R.string.connect);
                            connectButton.setEnabled(true);
                        });
                    }
                });

        signalClient.connect();
    }

    private void handleSignalMessage(JSONObject message) {
        try {
            String type = message.getString("type");
            if ("offer".equals(type)) {
                startTouchpadActivity();
            }
        } catch (JSONException e) {
            e.printStackTrace();
        }
    }

    private void startTouchpadActivity() {
        Intent intent = new Intent(this, TouchpadActivity.class);
        startActivity(intent);
    }

    private void disconnect() {
        if (signalClient != null) {
            signalClient.disconnect();
        }
        if (webRtcClient != null) {
            webRtcClient.disconnect();
        }
        statusText.setText(R.string.disconnected);
        connectButton.setText(R.string.connect);
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        disconnect();
    }
}
