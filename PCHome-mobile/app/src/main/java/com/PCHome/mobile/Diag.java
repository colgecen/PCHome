package com.PCHome.mobile;

import android.content.Context;
import android.util.Log;

import java.io.File;
import java.io.FileOutputStream;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

/**
 * Tanı kaydı: çökme raporu + bağlantı aşama günlüğü.
 *
 * <p>Telefonda "2 sn sonra başa atma" gibi sessiz ölümlerin sebebini
 * bulmak için: yakalanmayan istisnalar {@code crash.log}'a, bağlantı
 * aşamaları {@code phases.log}'a yazılır. PinActivity açılışta taze
 * çökme varsa diyaloğla gösterir.
 */
public class Diag {
    private static final String TAG = "Diag";
    private static final long MAX_BYTES = 100 * 1024;
    private static boolean installed = false;

    /** Uygulama açılışında bir kez çağrılır. */
    public static synchronized void init(Context ctx) {
        final Context app = ctx.getApplicationContext();
        phase(app, "uygulama açıldı");
        if (installed) {
            return;
        }
        installed = true;
        final Thread.UncaughtExceptionHandler prev =
                Thread.getDefaultUncaughtExceptionHandler();
        Thread.setDefaultUncaughtExceptionHandler((thread, ex) -> {
            try {
                StringWriter sw = new StringWriter();
                ex.printStackTrace(new PrintWriter(sw));
                String report = stamp() + " thread=" + thread.getName()
                        + "\n" + sw + "\n";
                writeFile(app, "crash.log", report, false);
                Log.e(TAG, "çökme kaydedildi", ex);
            } catch (Exception e) {
                Log.e(TAG, "çökme yazılamadı", e);
            }
            if (prev != null) {
                prev.uncaughtException(thread, ex);
            } else {
                try {
                    android.os.Process.killProcess(android.os.Process.myPid());
                } catch (Exception ignored) {
                }
            }
        });
    }

    /** Bağlantı aşaması (ör. "rtc-hazır", "auth-ok", "offer", "video-kare"). */
    public static synchronized void phase(Context ctx, String s) {
        try {
            append(ctx, "phases.log", stamp() + " " + s + "\n");
        } catch (Exception e) {
            Log.w(TAG, "aşama yazılamadı", e);
        }
    }

    public static synchronized String readCrash(Context ctx) {
        return read(ctx, "crash.log");
    }

    public static synchronized String readPhases(Context ctx) {
        return read(ctx, "phases.log");
    }

    public static synchronized void clear(Context ctx) {
        try {
            new File(ctx.getFilesDir(), "crash.log").delete();
            new File(ctx.getFilesDir(), "phases.log").delete();
        } catch (Exception e) {
            Log.w(TAG, "temizlenemedi", e);
        }
    }

    private static String stamp() {
        return new SimpleDateFormat("HH:mm:ss", Locale.US).format(new Date());
    }

    private static void writeFile(Context ctx, String name, String data, boolean append) {
        try {
            File f = new File(ctx.getFilesDir(), name);
            if (!append && f.exists()) {
                f.delete();
            }
            FileOutputStream out = new FileOutputStream(f, append);
            out.write(data.getBytes(StandardCharsets.UTF_8));
            out.close();
        } catch (Exception e) {
            Log.w(TAG, "dosya yazılamadı: " + name, e);
        }
    }

    private static void append(Context ctx, String name, String line) {
        File f = new File(ctx.getFilesDir(), name);
        if (f.exists() && f.length() > MAX_BYTES) {
            f.delete();
        }
        writeFile(ctx, name, line, true);
    }

    private static String read(Context ctx, String name) {
        try {
            File f = new File(ctx.getFilesDir(), name);
            if (!f.exists()) {
                return "";
            }
            byte[] buf = new byte[(int) Math.min(f.length(), MAX_BYTES)];
            java.io.FileInputStream in = new java.io.FileInputStream(f);
            int n = in.read(buf);
            in.close();
            if (n <= 0) {
                return "";
            }
            return new String(buf, 0, n, StandardCharsets.UTF_8);
        } catch (Exception e) {
            return "";
        }
    }
}
