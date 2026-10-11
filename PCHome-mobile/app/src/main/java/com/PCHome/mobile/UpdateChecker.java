package com.PCHome.mobile;

import android.app.DownloadManager;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.os.Environment;
import android.util.Log;

import androidx.core.content.FileProvider;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;

/**
 * GitHub Releases tabanlı uygulama içi güncelleyici.
 * Yeni bağımlılık YOK: yalnız HttpURLConnection + DownloadManager.
 *
 * <p>Akış: bağlantı ekranı açıldığında (günde en fazla 1 kez) {@code
 * releases/latest} denetlenir. Yeni sürüm varsa güncelleme kartı görünür;
 * "Güncelle" APK'yı indirip sistem kurulum ekranını açar. Kurulum izni
 * ({@code REQUEST_INSTALL_PACKAGES}) ilk kurulumda sistem tarafından sorulur.
 */
public class UpdateChecker {
    private static final String TAG = "UpdateChecker";
    private static final String API_URL =
            "https://api.github.com/repos/colgecen/PCHome/releases/latest";

    public static class UpdateInfo {
        public String version = "";
        public String notes = "";
        public String apkUrl = "";
        public long sizeBytes = 0;
        public boolean available = false;
    }

    public interface Callback {
        void onResult(UpdateInfo info);
        void onFailure(String error);
    }

    /** Uzak etiket güncel sürümden yeniyse true ("v" öneki yoksayılır). */
    public static boolean isNewer(String remoteTag, String current) {
        int[] a = normalize(remoteTag);
        int[] b = normalize(current);
        int n = Math.max(a.length, b.length);
        for (int i = 0; i < n; i++) {
            int x = i < a.length ? a[i] : 0;
            int y = i < b.length ? b[i] : 0;
            if (x != y) {
                return x > y;
            }
        }
        return false;
    }

    private static int[] normalize(String v) {
        if (v == null) {
            return new int[0];
        }
        String clean = v.trim();
        if (clean.startsWith("v") || clean.startsWith("V")) {
            clean = clean.substring(1);
        }
        String[] parts = clean.split("[.\\-+]");
        int[] out = new int[parts.length];
        for (int i = 0; i < parts.length; i++) {
            String digits = parts[i].replaceAll("^([0-9]+).*$", "$1");
            try {
                out[i] = Integer.parseInt(digits);
            } catch (NumberFormatException e) {
                out[i] = 0;
            }
        }
        return out;
    }

    /** Sürüm yanıtını ayrıştırır (saf fonksiyon, test edilebilir). */
    public static UpdateInfo parse(String json, String currentVersion) throws Exception {
        JSONObject rel = new JSONObject(json);
        UpdateInfo info = new UpdateInfo();
        info.version = rel.optString("tag_name", "");
        info.notes = rel.optString("body", "");
        JSONArray assets = rel.optJSONArray("assets");
        if (assets != null) {
            for (int i = 0; i < assets.length(); i++) {
                JSONObject a = assets.optJSONObject(i);
                if (a == null) {
                    continue;
                }
                String name = a.optString("name", "").toLowerCase();
                if (info.apkUrl.isEmpty() && name.endsWith(".apk")) {
                    info.apkUrl = a.optString("browser_download_url", "");
                    info.sizeBytes = a.optLong("size", 0);
                }
            }
        }
        info.available = !info.version.isEmpty()
                && !info.apkUrl.isEmpty()
                && isNewer(info.version, currentVersion);
        return info;
    }

    /** İnsan dostu boyut ("54,1 MB"). */
    public static String humanSize(long bytes) {
        double b = bytes;
        double mb = 1024.0 * 1024.0;
        if (b >= mb) {
            return String.format(java.util.Locale.US, "%.1f MB", b / mb).replace('.', ',');
        } else if (b >= 1024.0) {
            return String.format(java.util.Locale.US, "%.0f KB", b / 1024.0);
        }
        return bytes + " B";
    }

    /** Arka plan iş parçacığında denetler, sonucu ana iş parçacığında verir. */
    public static void check(String currentVersion, Callback cb,
                             android.os.Handler mainHandler) {
        new Thread(() -> {
            try {
                HttpURLConnection con =
                        (HttpURLConnection) new URL(API_URL).openConnection();
                con.setRequestProperty("User-Agent", "pchome-mobile-updater");
                con.setRequestProperty("Accept", "application/vnd.github+json");
                con.setConnectTimeout(15000);
                con.setReadTimeout(15000);
                int code = con.getResponseCode();
                if (code != 200) {
                    throw new Exception("HTTP " + code);
                }
                BufferedReader br = new BufferedReader(
                        new InputStreamReader(con.getInputStream(), "UTF-8"));
                StringBuilder sb = new StringBuilder();
                String line;
                while ((line = br.readLine()) != null) {
                    sb.append(line);
                }
                br.close();
                UpdateInfo info = parse(sb.toString(), currentVersion);
                mainHandler.post(() -> cb.onResult(info));
            } catch (Exception e) {
                Log.w(TAG, "Güncelleme denetlenemedi: " + e.getMessage());
                mainHandler.post(() -> cb.onFailure(e.getMessage()));
            }
        }).start();
    }

    /** İndirme sonucu dinleyicisi (modal akışı buradan beslenir). */
    public interface DownloadCallback {
        void onInstalled();
        void onFailed(String reason);
    }

    /**
     * APK'yı indirip bitince kurulum ekranını açar.
     * Sonucu {@code cb} ile bildirir; hata da buraya gelir (sessiz yutulmaz).
     *
     * @return indirme kimliği, hata varsa -1 (cb.onFailed çağrılmış olur).
     */
    public static long downloadAndInstall(Context ctx, UpdateInfo info, DownloadCallback cb) {
        android.os.Handler main = new android.os.Handler(ctx.getMainLooper());
        try {
            String fileName = "PCHome-" + info.version + ".apk";
            DownloadManager.Request req = new DownloadManager.Request(Uri.parse(info.apkUrl));
            req.setTitle("PCHome " + info.version + " indiriliyor");
            req.setDescription("Güncelleme paketi");
            req.setNotificationVisibility(
                    DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED);
            // file:// hedef Android 7+ üstünde istisna fırlatır; API'nin
            // kendi dış dosya hedefi kullanılır.
            req.setDestinationInExternalFilesDir(ctx, Environment.DIRECTORY_DOWNLOADS,
                    "updates/" + fileName);
            DownloadManager dm = (DownloadManager) ctx.getSystemService(Context.DOWNLOAD_SERVICE);
            if (dm == null) {
                throw new Exception("DownloadManager yok");
            }
            long id = dm.enqueue(req);
            android.content.BroadcastReceiver done =
                    new android.content.BroadcastReceiver() {
                @Override
                public void onReceive(Context c, Intent intent) {
                    if (intent.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1) != id) {
                        return;
                    }
                    try {
                        c.unregisterReceiver(this);
                    } catch (Exception ignored) {
                    }
                    try {
                        android.database.Cursor cur = dm.query(
                                new DownloadManager.Query().setFilterById(id));
                        int status = -1;
                        String localUri = null;
                        if (cur != null) {
                            if (cur.moveToFirst()) {
                                status = cur.getInt(cur.getColumnIndexOrThrow(
                                        DownloadManager.COLUMN_STATUS));
                                localUri = cur.getString(cur.getColumnIndexOrThrow(
                                        DownloadManager.COLUMN_LOCAL_URI));
                            }
                            cur.close();
                        }
                        if (status == DownloadManager.STATUS_SUCCESSFUL && localUri != null) {
                            File apk = new File(android.net.Uri.parse(localUri).getPath());
                            installApk(ctx, apk);
                            main.post(cb::onInstalled);
                        } else {
                            final int code = status;
                            main.post(() -> cb.onFailed("indirme tamamlanamadı (durum=" + code + ")"));
                        }
                    } catch (Exception e) {
                        main.post(() -> cb.onFailed(String.valueOf(e.getMessage())));
                    }
                }
            };
            android.content.IntentFilter filter =
                    new android.content.IntentFilter(DownloadManager.ACTION_DOWNLOAD_COMPLETE);
            if (android.os.Build.VERSION.SDK_INT >= 33) {
                ctx.registerReceiver(done, filter, Context.RECEIVER_NOT_EXPORTED);
            } else {
                ctx.registerReceiver(done, filter);
            }
            return id;
        } catch (Exception e) {
            Log.e(TAG, "İndirme başlatılamadı", e);
            main.post(() -> cb.onFailed(String.valueOf(e.getMessage())));
            return -1;
        }
    }

    /** APK'yı indirip sistem kurulum ekranını açar (ateşle-unut sürümü). */
    public static void downloadAndInstall(Context ctx, UpdateInfo info) {
        downloadAndInstall(ctx, info, new DownloadCallback() {
            @Override
            public void onInstalled() {
            }

            @Override
            public void onFailed(String reason) {
                Log.e(TAG, "Otomatik kurulum başarısız: " + reason);
            }
        });
    }

    /** İndirilen APK için sistem kurulum ekranını açar. */
    public static void installApk(Context ctx, File apk) {
        try {
            Uri uri = FileProvider.getUriForFile(ctx,
                    ctx.getPackageName() + ".fileprovider", apk);
            Intent intent = new Intent(Intent.ACTION_INSTALL_PACKAGE);
            intent.setData(uri);
            intent.setFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                    | Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
        } catch (Exception e) {
            Log.e(TAG, "Kurulum ekranı açılamadı", e);
        }
    }
}
