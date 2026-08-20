package com.pchome.mobile;

import org.java_websocket.client.WebSocketClient;
import org.java_websocket.handshake.ServerHandshake;
import org.json.JSONException;
import org.json.JSONObject;

import java.net.URI;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

public class SignalClient {
    private static final String TAG = "SignalClient";
    private static final long RECONNECT_DELAY_MS = 2000;
    private static final int MAX_RECONNECT_ATTEMPTS = 10;

    private WebSocketClient webSocketClient;
    private String serverUrl;
    private String pin;
    private BlockingQueue<JSONObject> messageQueue;
    private boolean connected;
    private int reconnectAttempts;
    private Thread reconnectThread;

    public interface SignalListener {
        void onConnected();
        void onDisconnected();
        void onMessage(JSONObject message);
        void onError(String error);
    }

    private SignalListener listener;

    public SignalClient(String serverUrl, String pin, SignalListener listener) {
        this.serverUrl = serverUrl;
        this.pin = pin;
        this.listener = listener;
        this.messageQueue = new LinkedBlockingQueue<>();
        this.connected = false;
    }

    public void connect() {
        if (webSocketClient != null && webSocketClient.isOpen()) {
            return;
        }

        try {
            URI uri = new URI(serverUrl + "?pin=" + pin);
            webSocketClient = new WebSocketClient(uri) {
                @Override
                public void onOpen(ServerHandshake handshakedata) {
                    connected = true;
                    reconnectAttempts = 0;
                    Log.i(TAG, "WebSocket connected");
                    if (listener != null) listener.onConnected();
                }

                @Override
                public void onMessage(String message) {
                    try {
                        JSONObject json = new JSONObject(message);
                        if (listener != null) listener.onMessage(json);
                    } catch (JSONException e) {
                        Log.e(TAG, "Failed to parse message", e);
                    }
                }

                @Override
                public void onClose(int code, String reason, boolean remote) {
                    connected = false;
                    Log.i(TAG, "WebSocket closed: " + reason);
                    if (listener != null) listener.onDisconnected();
                    scheduleReconnect();
                }

                @Override
                public void onError(Exception ex) {
                    Log.e(TAG, "WebSocket error", ex);
                    if (listener != null) listener.onError(ex.getMessage());
                }
            };

            webSocketClient.connectBlocking();
        } catch (Exception e) {
            Log.e(TAG, "Connection failed", e);
            scheduleReconnect();
        }
    }

    public void disconnect() {
        connected = false;
        if (reconnectThread != null) {
            reconnectThread.interrupt();
        }
        if (webSocketClient != null) {
            webSocketClient.close();
        }
    }

    public boolean sendMessage(JSONObject message) {
        if (webSocketClient != null && connected) {
            try {
                webSocketClient.send(message.toString());
                return true;
            } catch (Exception e) {
                Log.e(TAG, "Send failed", e);
            }
        }
        return false;
    }

    public boolean isConnected() {
        return connected;
    }

    private void scheduleReconnect() {
        if (reconnectThread != null && reconnectThread.isAlive()) {
            return;
        }

        reconnectThread = new Thread(() -> {
            while (reconnectAttempts < MAX_RECONNECT_ATTEMPTS && !connected) {
                try {
                    Thread.sleep(RECONNECT_DELAY_MS);
                } catch (InterruptedException e) {
                    return;
                }
                reconnectAttempts++;
                Log.i(TAG, "Reconnecting attempt " + reconnectAttempts);
                connect();
            }
        });
        reconnectThread.start();
    }
}
