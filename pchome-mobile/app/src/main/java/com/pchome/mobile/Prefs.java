package com.pchome.mobile;

import android.content.Context;
import android.content.SharedPreferences;

import java.util.ArrayList;
import java.util.Collections;
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
    private static final String KEY_PINS = "pins";
    private static final String KEY_LAST_SERVER = "last_server";
    private static final int MAX_RECENT = 8;

    private final SharedPreferences sp;

    public Prefs(Context ctx) {
        this.sp = ctx.getSharedPreferences(NAME, Context.MODE_PRIVATE);
    }

    public List<String> getServers() {
        return new ArrayList<>(sp.getStringSet(KEY_SERVERS, new HashSet<>()));
    }

    public List<String> getPins() {
        return new ArrayList<>(sp.getStringSet(KEY_PINS, new HashSet<>()));
    }

    public int getLastServerIndex() {
        return sp.getInt(KEY_LAST_SERVER, 0);
    }

    /** Records a used server URL (most-recent-first) and marks it last-used. */
    public void addServer(String url) {
        if (url == null || url.trim().isEmpty()) return;
        List<String> servers = getServers();
        servers.remove(url);
        servers.add(0, url);
        if (servers.size() > MAX_RECENT) servers = servers.subList(0, MAX_RECENT);
        sp.edit().putStringSet(KEY_SERVERS, new HashSet<>(servers))
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
