package com.PCHome.mobile;

import android.content.Context;
import android.content.SharedPreferences;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.util.ArrayList;
import java.util.List;

/**
 * Son bağlanılan PC'lerin kalıcı deposu.
 * Üst veri SharedPreferences'te JSON dizisi, küçük resimler
 * {@code filesDir/recent/<pcId>.jpg} dosyalarında tutulur. En fazla 12 kayıt.
 */
public class RecentStore {
    private static final String NAME = "pchome_recent";
    private static final String KEY_LIST = "recent_list";
    private static final int MAX_RECENT = 12;

    private final SharedPreferences sp;
    private final File thumbDir;

    public RecentStore(Context ctx) {
        this.sp = ctx.getSharedPreferences(NAME, Context.MODE_PRIVATE);
        this.thumbDir = new File(ctx.getFilesDir(), "recent");
        if (!thumbDir.exists()) {
            thumbDir.mkdirs();
        }
    }

    public synchronized List<RecentPc> list() {
        List<RecentPc> out = new ArrayList<>();
        String raw = sp.getString(KEY_LIST, "[]");
        try {
            JSONArray arr = new JSONArray(raw);
            for (int i = 0; i < arr.length(); i++) {
                JSONObject o = arr.optJSONObject(i);
                if (o != null) {
                    RecentPc r = RecentPc.fromJson(o);
                    if (!r.pcId.isEmpty()) {
                        out.add(r);
                    }
                }
            }
        } catch (Exception e) {
            // Bozuk kayıt tek seferde temizlenir.
            sp.edit().remove(KEY_LIST).apply();
        }
        return out;
    }

    private synchronized void save(List<RecentPc> items) {
        try {
            JSONArray arr = new JSONArray();
            for (RecentPc r : items) {
                arr.put(r.toJson());
            }
            sp.edit().putString(KEY_LIST, arr.toString()).apply();
        } catch (Exception e) {
            // Yazım hatası sessiz geçilir; bağlantı etkilenmez.
        }
    }

    /** Bağlantı kurulurken/kapanırken kaydı başa taşı (bilinen alanları koru). */
    public synchronized void upsert(RecentPc entry) {
        if (entry == null || entry.pcId.isEmpty()) {
            return;
        }
        List<RecentPc> items = list();
        RecentPc keep = null;
        for (RecentPc r : items) {
            if (r.pcId.equals(entry.pcId)) {
                keep = r;
                break;
            }
        }
        if (keep != null) {
            if (entry.pcName.isEmpty()) entry.pcName = keep.pcName;
            if (entry.ip.isEmpty()) entry.ip = keep.ip;
            if (entry.mac.isEmpty()) entry.mac = keep.mac;
            if (entry.resolution.isEmpty()) entry.resolution = keep.resolution;
            if (entry.thumbnail.isEmpty()) entry.thumbnail = keep.thumbnail;
            if (entry.lastSeen == 0) entry.lastSeen = keep.lastSeen;
            if (!entry.rememberPassword && keep.rememberPassword) {
                entry.password = keep.password;
                entry.rememberPassword = true;
            }
            items.remove(keep);
        }
        if (entry.lastSeen == 0) {
            entry.lastSeen = System.currentTimeMillis();
        }
        items.add(0, entry);
        while (items.size() > MAX_RECENT) {
            RecentPc dropped = items.remove(items.size() - 1);
            deleteThumbnail(dropped.pcId);
        }
        save(items);
    }

    public synchronized void remove(String pcId) {
        List<RecentPc> items = list();
        for (int i = 0; i < items.size(); i++) {
            if (items.get(i).pcId.equals(pcId)) {
                items.remove(i);
                break;
            }
        }
        save(items);
        deleteThumbnail(pcId);
    }

    public synchronized RecentPc find(String pcId) {
        for (RecentPc r : list()) {
            if (r.pcId.equals(pcId)) {
                return r;
            }
        }
        return null;
    }

    public File thumbnailFile(String pcId) {
        String safe = pcId.replaceAll("[^A-Za-z0-9]", "_");
        return new File(thumbDir, safe + ".jpg");
    }

    private void deleteThumbnail(String pcId) {
        try {
            File f = thumbnailFile(pcId);
            if (f.exists()) {
                f.delete();
            }
        } catch (Exception e) {
            // Yoksay.
        }
    }
}
