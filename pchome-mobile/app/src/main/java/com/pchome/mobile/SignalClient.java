package com.pchome.mobile;

import android.util.Log;

import androidx.annotation.NonNull;

import org.json.JSONException;
import org.json.JSONObject;

import java.util.concurrent.TimeUnit;

import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.Response;
import okhttp3.WebSocket;
import okhttp3.WebSocketListener;

public class SignalClient {
    private static final String TAG = "SignalClient";
    private static final long RECONNECT_DELAY_MS = 2000;
    private static final int MAX_RECONNECT_ATTEMPTS = 10;

    private OkHttpClient httpClient;
    private WebSocket webSocket;
    private String serverUrl;
    private String pin;
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
        this.connected = false;
    }

    public void connect() {
        if (webSocket != null && connected) {
            return;
        }

        httpClient = new OkHttpClient.Builder()
                // Cloudflare closes idle sockets after ~100s; a periodic ping
                // keeps the signaling connection alive behind the proxy.
                .pingInterval(20, TimeUnit.SECONDS)
                .build();

        String url = serverUrl + "?pin=" + pin + "&role=mobile";
        Request request = new Request.Builder().url(url).build();

        webSocket = httpClient.newWebSocket(request, new WebSocketListener() {
            @Override
            public void onOpen(@NonNull WebSocket ws, @NonNull Response response) {
                connected = true;
                reconnectAttempts = 0;
                Log.i(TAG, "WebSocket connected");
                if (listener != null) listener.onConnected();
            }

            @Override
            public void onMessage(@NonNull WebSocket ws, @NonNull String text) {
                try {
                    JSONObject json = new JSONObject(text);
                    if (listener != null) listener.onMessage(json);
                } catch (JSONException e) {
                    Log.e(TAG, "Failed to parse message", e);
                }
            }

            @Override
            public void onClosed(@NonNull WebSocket ws, int code, @NonNull String reason) {
                connected = false;
                Log.i(TAG, "WebSocket closed: " + reason);
                if (listener != null) listener.onDisconnected();
                scheduleReconnect();
            }

            @Override
            public void onFailure(@NonNull WebSocket ws, @NonNull Throwable t, Response response) {
                connected = false;
                Log.e(TAG, "WebSocket error: " + t.getMessage(), t);
                if (listener != null) listener.onError(t.getMessage());
                scheduleReconnect();
            }
        });
    }

    public void disconnect() {
        connected = false;
        if (reconnectThread != null) {
            reconnectThread.interrupt();
        }
        if (webSocket != null) {
            webSocket.close(1000, "client disconnect");
            webSocket = null;
        }
        if (httpClient != null) {
            httpClient.dispatcher().executorService().shutdown();
        }
    }

    public boolean sendMessage(JSONObject message) {
        if (webSocket != null && connected) {
            try {
                webSocket.send(message.toString());
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

    private void scheduleRepeatedReconnect() {
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

    private void scheduleReconnect() {
        scheduleRepeatedReconnect();
    }
}
