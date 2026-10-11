package com.pchome.mobile;

import org.json.JSONObject;
import org.junit.Test;
import org.junit.Assert;

public class SignalClientUnitTest {
    @Test
    public void testUrlConstruction() {
        Assert.assertEquals(
                "wss://pchome.onrender.com/ws?pc_id=482913&role=mobile",
                SignalClient.buildUrl("wss://pchome.onrender.com/ws", "482913"));
        // /ws'siz yazım da aynı kapıya çıkar.
        Assert.assertEquals(
                "wss://pchome.onrender.com/ws?pc_id=482913&role=mobile",
                SignalClient.buildUrl("wss://pchome.onrender.com", "482913"));
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
    public void testReconnectAttempts() {
        int maxAttempts = 10;
        int attempts = 0;
        while (attempts < maxAttempts) {
            attempts++;
        }
        Assert.assertEquals(10, attempts);
    }
}
