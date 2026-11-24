package com.PCHome.mobile;

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
    private final String deviceName;
    private final String deviceOs;
    private final String appVersion;
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
        this(serverUrl, pcId, password, listener, "", "", "");
    }

    public SignalClient(String serverUrl, String pcId, String password, SignalListener listener,
                        String deviceName, String deviceOs, String appVersion) {
        this.serverUrl = serverUrl;
        this.pcId = pcId == null ? "" : pcId.replaceAll("[^A-Za-z0-9]", "");
        this.password = password == null ? "" : password.replaceAll("[^0-9]", "");
        this.listener = listener;
        this.deviceName = deviceName == null ? "" : deviceName.trim();
        this.deviceOs = deviceOs == null ? "" : deviceOs.trim();
        this.appVersion = appVersion == null ? "" : appVersion.trim();
        this.connected = false;
    }

    /**
     * Kullanıcının yazdığı sunucu adresini normalize eder. `ws://` korunur
     * (yerel/TLS'siz sunucular için); şemasız adresler varsayılan olarak
     * `wss://` kabul edilir.
     */
    public static String normalizeServer(String server) {
        String v = server == null ? "" : server.trim();
        if (v.startsWith("https://")) {
            v = "wss://" + v.substring(8);
        } else if (v.startsWith("http://")) {
            v = "ws://" + v.substring(7);
        } else if (!v.startsWith("ws://") && !v.startsWith("wss://")) {
            // Şemasız yerel/özel ağ adresleri varsayılan olarak ws:// olsun;
            // dış adresler wss:// kalır.
            v = (looksLocalHost(v) ? "ws://" : "wss://") + v;
        }
        v = v.replaceAll("/+$", "");
        if (!v.endsWith("/ws")) {
            v = v + "/ws";
        }
        return v;
    }

    /** localhost / 127.x / 10.x / 192.168.x / 172.16-31.x / *.local / [::1]. */
    static boolean looksLocalHost(String raw) {
        String host = raw == null ? "" : raw.trim();
        int scheme = host.indexOf("://");
        if (scheme >= 0) host = host.substring(scheme + 3);
        int slash = host.indexOf('/');
        if (slash >= 0) host = host.substring(0, slash);
        if (host.startsWith("[")) {
            int end = host.indexOf(']');
            if (end > 0) host = host.substring(1, end);
        } else {
            int colon = host.indexOf(':');
            if (colon >= 0) host = host.substring(0, colon);
        }
        host = host.toLowerCase(java.util.Locale.ROOT);
        if (host.equals("localhost") || host.equals("0.0.0.0") || host.equals("::1")
                || host.endsWith(".local")) {
            return true;
        }
        if (host.startsWith("127.") || host.startsWith("10.") || host.startsWith("192.168.")) {
            return true;
        }
        if (host.startsWith("172.")) {
            String[] parts = host.split("\\.");
            if (parts.length >= 2) {
                try {
                    int second = Integer.parseInt(parts[1]);
                    return second >= 16 && second <= 31;
                } catch (NumberFormatException ignored) {
                }
            }
        }
        return false;
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

    /** Tanı için sunucu adresi (hata mesajlarında görünür). */
    public static String hostOf(String serverUrl) {
        try {
            android.net.Uri u = android.net.Uri.parse(serverUrl.trim());
            String host = u.getHost();
            return host == null || host.isEmpty() ? serverUrl : host;
        } catch (Exception e) {
            return serverUrl;
        }
    }

    /** Kimlik doğrulama mesajı (test edilebilir saf fonksiyon). */
    public static JSONObject buildJoinMessage(String pcId, String password) throws JSONException {
        return buildJoinMessage(pcId, password, null, null, null);
    }

    /** Cihaz adı/OS sürümüyle birlikte join mesajı. */
    public static JSONObject buildJoinMessage(String pcId, String password,
                                              String deviceName, String deviceOs,
                                              String appVersion) throws JSONException {
        JSONObject join = new JSONObject();
        join.put("type", "join");
        join.put("pc_id", pcId);
        join.put("password", password);
        if (deviceName != null && !deviceName.trim().isEmpty()) {
            join.put("device_name", deviceName.trim());
        }
        if (deviceOs != null && !deviceOs.trim().isEmpty()) {
            join.put("device_os", deviceOs.trim());
        }
        if (appVersion != null && !appVersion.trim().isEmpty()) {
            join.put("app_version", appVersion.trim());
        }
        return join;
    }

    public void connect() {
        if (webSocket != null && connected) {
            return;
        }
        if (authFailed) {
            return;
        }
        manualClose = false;

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
                sendJoin();
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
                String reason = t.getMessage();
                if (reason == null || reason.trim().isEmpty()) {
                    // Render/Cloudflare bazen nedensiz kapatır; "null" yerine
                    // anlaşılır bir tür adı göster.
                    reason = t.getClass().getSimpleName();
                }
                String detail = "Sinyal (" + hostOf(serverUrl) + "): " + reason;
                Log.e(TAG, "WebSocket hatası: " + detail, t);
                if (listener != null) listener.onError(detail);
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

    /** Kimlik doğrulama isteğini (yeniden) gönderir. */
    public boolean sendJoin() {
        try {
            return sendMessage(buildJoinMessage(pcId, password, deviceName, deviceOs, appVersion));
        } catch (JSONException e) {
            Log.e(TAG, "join kurulamadı", e);
            return false;
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
            // Denemeler bitti, hâlâ kopuğuz: arayüz sonsuz beklemesin.
            if (!connected && !authFailed && !manualClose && listener != null) {
                listener.onError("Sinyal sunucusuna ulaşılamadı ("
                        + hostOf(serverUrl) + ", " + MAX_RECONNECT_ATTEMPTS + " deneme)");
            }
        });
        reconnectThread.start();
    }

    private void scheduleReconnect() {
        scheduleRepeatedReconnect();
    }
}
