package com.PCHome.mobile;

import org.json.JSONException;
import org.json.JSONObject;

/**
 * "Son Girilenler" kartındaki tek PC kaydı.
 * Şifre yalnız kullanıcı "hatırla"yı işaretlediyse saklanır.
 */
public class RecentPc {
    public String pcId = "";
    public String pcName = "";
    public String password = "";
    public boolean rememberPassword = false;
    public String ip = "";
    public String mac = "";
    public String resolution = "";
    public long lastSeen = 0;
    public String thumbnail = "";

    public RecentPc() {
    }

    public RecentPc(String pcId, String pcName) {
        this.pcId = pcId;
        this.pcName = pcName;
    }

    public JSONObject toJson() throws JSONException {
        JSONObject o = new JSONObject();
        o.put("pcId", pcId);
        o.put("pcName", pcName);
        o.put("password", rememberPassword ? password : "");
        o.put("rememberPassword", rememberPassword);
        o.put("ip", ip);
        o.put("mac", mac);
        o.put("resolution", resolution);
        o.put("lastSeen", lastSeen);
        o.put("thumbnail", thumbnail);
        return o;
    }

    public static RecentPc fromJson(JSONObject o) {
        RecentPc r = new RecentPc();
        r.pcId = o.optString("pcId", "");
        r.pcName = o.optString("pcName", "");
        r.password = o.optString("password", "");
        r.rememberPassword = o.optBoolean("rememberPassword", false);
        r.ip = o.optString("ip", "");
        r.mac = o.optString("mac", "");
        r.resolution = o.optString("resolution", "");
        r.lastSeen = o.optLong("lastSeen", 0);
        r.thumbnail = o.optString("thumbnail", "");
        return r;
    }

    /** Kartın alt satırı: "ID • IP • MAC". */
    public String subtitle() {
        StringBuilder sb = new StringBuilder();
        sb.append("ID: ").append(pcId);
        if (!ip.isEmpty()) {
            sb.append("  •  ").append(ip);
        }
        if (!mac.isEmpty() && !"bilinmiyor".equalsIgnoreCase(mac)) {
            sb.append("  •  ").append(mac);
        }
        return sb.toString();
    }
}
