package com.PCHome.mobile;

import android.content.Context;
import android.content.SharedPreferences;

import org.json.JSONArray;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * SharedPreferences-backed persistence for the mobile app:
 *  - recently used signal server URLs (most-recent-first)
 *  - recently used PINs (most-recent-first)
 *  - the index of the last used server (so the PIN screen can pre-select it)
 */
public class Prefs {
    private static final String NAME = "pchome_prefs";
    private static final String KEY_SERVERS = "servers";
    private static final String KEY_SERVERS_JSON = "servers_json";
    private static final String KEY_PINS = "pins";
    private static final String KEY_LAST_SERVER = "last_server";
    private static final String KEY_LAST_UPDATE_CHECK = "last_update_check";
    private static final int MAX_RECENT = 8;

    private final SharedPreferences sp;

    public Prefs(Context ctx) {
        this.sp = ctx.getSharedPreferences(NAME, Context.MODE_PRIVATE);
    }

    /**
     * Sunucu listesini en son kullanılan başta olacak şekilde döndürür.
     * Eskiden {@code StringSet} kullanıldığı için sıra kayboluyordu; yeni
     * kayıtlar JSON dizisinde sıralı tutulur, eski kayıtlar bir kez taşınır.
     */
    public List<String> getServers() {
        String raw = sp.getString(KEY_SERVERS_JSON, null);
        if (raw != null) {
            try {
                JSONArray arr = new JSONArray(raw);
                List<String> out = new ArrayList<>();
                for (int i = 0; i < arr.length(); i++) {
                    String url = arr.optString(i, "").trim();
                    if (!url.isEmpty() && !out.contains(url)) {
                        out.add(url);
                    }
                }
                return out;
            } catch (Exception e) {
                // Bozuk JSON: aşağıdaki eski StringSet yolunu dene.
            }
        }
        // Eski sürüm StringSet kullanıyordu; bir kez buradan okunur.
        return new ArrayList<>(sp.getStringSet(KEY_SERVERS, new HashSet<>()));
    }

    public List<String> getPins() {
        return new ArrayList<>(sp.getStringSet(KEY_PINS, new HashSet<>()));
    }

    public int getLastServerIndex() {
        return sp.getInt(KEY_LAST_SERVER, 0);
    }

    public long getLastUpdateCheck() {
        return sp.getLong(KEY_LAST_UPDATE_CHECK, 0);
    }

    public void setLastUpdateCheck(long now) {
        sp.edit().putLong(KEY_LAST_UPDATE_CHECK, now).apply();
    }

    /** Records a used server URL (most-recent-first) and marks it last-used. */
    public void addServer(String url) {
        if (url == null || url.trim().isEmpty()) return;
        String clean = url.trim();
        List<String> servers = getServers();
        servers.remove(clean);
        servers.add(0, clean);
        if (servers.size() > MAX_RECENT) servers = new ArrayList<>(servers.subList(0, MAX_RECENT));
        JSONArray arr = new JSONArray();
        for (String s : servers) {
            arr.put(s);
        }
        sp.edit().putString(KEY_SERVERS_JSON, arr.toString())
                .remove(KEY_SERVERS)
                .putInt(KEY_LAST_SERVER, 0).apply();
    }

    /** Records a used PIN (most-recent-first). */
    public void addPin(String pin) {
        if (pin == null || pin.trim().isEmpty()) return;
        List<String> pins = getPins();
        pins.remove(pin);
        pins.add(0, pin);
        if (pins.size() > MAX_RECENT) pins = pins.subList(0, MAX_RECENT);
        sp.edit().putStringSet(KEY_PINS, new HashSet<>(pins)).apply();
    }
}
