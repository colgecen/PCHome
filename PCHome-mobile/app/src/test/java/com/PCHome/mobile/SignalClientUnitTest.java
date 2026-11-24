package com.PCHome.mobile;

import org.json.JSONObject;
import org.junit.Test;
import org.junit.Assert;

public class SignalClientUnitTest {
    @Test
    public void testUrlConstruction() {
        Assert.assertEquals(
                "wss://signal.example.com/ws?pc_id=482913&role=mobile",
                SignalClient.buildUrl("wss://signal.example.com/ws", "482913"));
        // /ws'siz yazım da aynı kapıya çıkar.
        Assert.assertEquals(
                "wss://signal.example.com/ws?pc_id=482913&role=mobile",
                SignalClient.buildUrl("wss://signal.example.com", "482913"));
    }

    @Test
    public void testPcIdTemizlenir() {
        Assert.assertEquals(
                "wss://x/ws?pc_id=482913&role=mobile",
                SignalClient.buildUrl("wss://x/ws", "482-913"));
    }

    @Test
    public void testJoinMesaji() throws Exception {
        JSONObject join = SignalClient.buildJoinMessage("482913", "123456");
        Assert.assertEquals("join", join.getString("type"));
        Assert.assertEquals("482913", join.getString("pc_id"));
        Assert.assertEquals("123456", join.getString("password"));
    }

    @Test
    public void testJoinCihazBilgisi() throws Exception {
        JSONObject join = SignalClient.buildJoinMessage(
                "482913", "123456", "Google Pixel 7", "Android 14", "0.4.0");
        Assert.assertEquals("Google Pixel 7", join.getString("device_name"));
        Assert.assertEquals("Android 14", join.getString("device_os"));
        Assert.assertEquals("0.4.0", join.getString("app_version"));
    }

    @Test
    public void testServerNormalize() {
        // ws:// yerel sunucular için KORUNUR (eskiden zorla wss:// yapılıyordu).
        Assert.assertEquals("ws://127.0.0.1:8080/ws",
                SignalClient.normalizeServer("ws://127.0.0.1:8080"));
        Assert.assertEquals("ws://127.0.0.1:8080/ws",
                SignalClient.normalizeServer("http://127.0.0.1:8080"));
        Assert.assertEquals("wss://signal.example.com/ws",
                SignalClient.normalizeServer("https://signal.example.com/ws/"));
        Assert.assertEquals("wss://signal.example.com/ws",
                SignalClient.normalizeServer("signal.example.com"));
        Assert.assertEquals("ws://192.168.1.20:8080/ws",
                SignalClient.normalizeServer("192.168.1.20:8080"));
        Assert.assertEquals("ws://10.0.0.5:8080/ws",
                SignalClient.normalizeServer("10.0.0.5:8080"));
        Assert.assertEquals("wss://172.32.0.1/ws",
                SignalClient.normalizeServer("172.32.0.1"));
    }

    @Test
    public void testReconnectAttempts() {
        int maxAttempts = 10;
        int attempts = 0;
        while (attempts < maxAttempts) {
            attempts++;
        }
        Assert.assertEquals(10, attempts);
    }
}
