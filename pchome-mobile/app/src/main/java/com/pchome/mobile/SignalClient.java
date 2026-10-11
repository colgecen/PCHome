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

/**
 * Sinyal sunucusuna WebSocket istemcisi (protokol v2).
 *
 * <p>Oda anahtarı kalıcı PC kimliğidir: {@code ?pc_id=...&role=mobile}.
 * Bağlantı açılınca {@code {"type":"join","pc_id":..,"password":..}} gönderilir;
 * desktop şifreyi kendi identity dosyasıyla karşılaştırıp {@code auth_ok} /
 * {@code auth_rejected} döner. Kimlik reddinde otomatik yeniden bağlanma
 * YAPILMAZ (şifre yanlışsa 10 kez denemenin anlamı yok).
 */
public class SignalClient {
    private static final String TAG = "SignalClient";
    private static final long RECONNECT_DELAY_MS = 2000;
    private static final int MAX_RECONNECT_ATTEMPTS = 10;

    private OkHttpClient httpClient;
    private WebSocket webSocket;
    private final String serverUrl;
    private final String pcId;
    private final String password;
    private boolean connected;
    private boolean authFailed;
    private boolean manualClose;
    private int reconnectAttempts;
    private Thread reconnectThread;

    public interface SignalListener {
        void onConnected();
        void onDisconnected();
        void onMessage(JSONObject message);
        void onError(String error);
    }

    private final SignalListener listener;

    public SignalClient(String serverUrl, String pcId, String password, SignalListener listener) {
        this.serverUrl = serverUrl;
        this.pcId = pcId == null ? "" : pcId.replaceAll("[^A-Za-z0-9]", "");
        this.password = password == null ? "" : password.replaceAll("[^0-9]", "");
        this.listener = listener;
        this.connected = false;
    }

    /** Sinyal URL'sine oda anahtarını ekler (test edilebilir saf fonksiyon). */
    public static String buildUrl(String serverUrl, String pcId) {
        String base = serverUrl.trim().replaceAll("/+$", "");
        String clean = pcId.replaceAll("[^A-Za-z0-9]", "");
        if (base.endsWith("/ws")) {
            return base + "?pc_id=" + clean + "&role=mobile";
        }
        return base + "/ws?pc_id=" + clean + "&role=mobile";
    }

    /** Kimlik doğrulama mesajı (test edilebilir saf fonksiyon). */
    public static JSONObject buildJoinMessage(String pcId, String password) throws JSONException {
        JSONObject join = new JSONObject();
        join.put("type", "join");
        join.put("pc_id", pcId);
        join.put("password", password);
        return join;
    }

    public void connect() {
        if (webSocket != null && connected) {
            return;
        }
        manualClose = false;
        authFailed = false;

        httpClient = new OkHttpClient.Builder()
                // Cloudflare ~100 sn'de boş soketi kapatır; periyodik ping
                // sinyal bağlantısını proxy arkasında canlı tutar.
                .pingInterval(20, TimeUnit.SECONDS)
                .build();

        String url = buildUrl(serverUrl, pcId);
        Request request = new Request.Builder().url(url).build();

        webSocket = httpClient.newWebSocket(request, new WebSocketListener() {
            @Override
            public void onOpen(@NonNull WebSocket ws, @NonNull Response response) {
                connected = true;
                reconnectAttempts = 0;
                Log.i(TAG, "WebSocket bağlandı, join gönderiliyor");
                try {
                    ws.send(buildJoinMessage(pcId, password).toString());
                } catch (JSONException e) {
                    Log.e(TAG, "join kurulamadı", e);
                }
                if (listener != null) listener.onConnected();
            }

            @Override
            public void onMessage(@NonNull WebSocket ws, @NonNull String text) {
                try {
                    JSONObject json = new JSONObject(text);
                    if (listener != null) listener.onMessage(json);
                } catch (JSONException e) {
                    Log.e(TAG, "Mesaj çözülemedi", e);
                }
            }

            @Override
            public void onClosed(@NonNull WebSocket ws, int code, @NonNull String reason) {
                connected = false;
                Log.i(TAG, "WebSocket kapandı: " + reason);
                if (listener != null) listener.onDisconnected();
                if (!manualClose && !authFailed) {
                    scheduleReconnect();
                }
            }

            @Override
            public void onFailure(@NonNull WebSocket ws, @NonNull Throwable t, Response response) {
                connected = false;
                Log.e(TAG, "WebSocket hatası: " + t.getMessage(), t);
                if (listener != null) listener.onError(t.getMessage());
                if (!manualClose && !authFailed) {
                    scheduleReconnect();
                }
            }
        });
    }

    /** Şifre reddedildi: yeniden deneme döngüsünü durdur. */
    public void setAuthFailed() {
        authFailed = true;
        if (reconnectThread != null) {
            reconnectThread.interrupt();
        }
    }

    public void disconnect() {
        manualClose = true;
        connected = false;
        if (reconnectThread != null) {
            reconnectThread.interrupt();
        }
        if (webSocket != null) {
            webSocket.close(1000, "istemci kapattı");
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
                Log.e(TAG, "Gönderim başarısız", e);
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
            while (reconnectAttempts < MAX_RECONNECT_ATTEMPTS && !connected
                    && !authFailed && !manualClose) {
                try {
                    Thread.sleep(RECONNECT_DELAY_MS);
                } catch (InterruptedException e) {
                    return;
                }
                reconnectAttempts++;
                Log.i(TAG, "Yeniden bağlanma denemesi " + reconnectAttempts);
                connect();
            }
        });
        reconnectThread.start();
    }

    private void scheduleReconnect() {
        scheduleRepeatedReconnect();
    }
}
