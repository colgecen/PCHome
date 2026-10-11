package com.pchome.mobile;

import org.json.JSONObject;
import org.junit.Test;
import org.junit.Assert;

public class RecentPcTest {
    @Test
    public void testJsonGidisDonus() throws Exception {
        RecentPc r = new RecentPc("482913", "Masaüstü");
        r.password = "123456";
        r.rememberPassword = true;
        r.ip = "192.168.1.10";
        r.mac = "AA:BB:CC:DD:EE:FF";
        r.resolution = "1920x1080";
        r.lastSeen = 123456789L;
        r.thumbnail = "/tmp/482913.jpg";

        JSONObject o = r.toJson();
        RecentPc back = RecentPc.fromJson(o);
        Assert.assertEquals("482913", back.pcId);
        Assert.assertEquals("Masaüstü", back.pcName);
        Assert.assertEquals("123456", back.password);
        Assert.assertTrue(back.rememberPassword);
        Assert.assertEquals("192.168.1.10", back.ip);
        Assert.assertEquals("AA:BB:CC:DD:EE:FF", back.mac);
        Assert.assertEquals(123456789L, back.lastSeen);
    }

    @Test
    public void testSifreHatirlanmazsaSaklanmaz() throws Exception {
        RecentPc r = new RecentPc("111111", "");
        r.password = "654321";
        r.rememberPassword = false;
        JSONObject o = r.toJson();
        Assert.assertEquals("", o.getString("password"));
    }

    @Test
    public void testAltBaslik() {
        RecentPc r = new RecentPc("482913", "");
        r.ip = "192.168.1.10";
        r.mac = "AA:BB:CC:DD:EE:FF";
        String sub = r.subtitle();
        Assert.assertTrue(sub.contains("482913"));
        Assert.assertTrue(sub.contains("192.168.1.10"));
        Assert.assertTrue(sub.contains("AA:BB:CC:DD:EE:FF"));
    }
}
