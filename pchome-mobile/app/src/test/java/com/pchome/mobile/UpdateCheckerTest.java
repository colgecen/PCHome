package com.pchome.mobile;

import org.junit.Test;
import org.junit.Assert;

public class UpdateCheckerTest {
    @Test
    public void testIsNewer() {
        Assert.assertTrue(UpdateChecker.isNewer("v0.3.0", "0.2.0"));
        Assert.assertTrue(UpdateChecker.isNewer("0.3.1", "0.3.0"));
        Assert.assertFalse(UpdateChecker.isNewer("v0.3.0", "0.3.0"));
        Assert.assertFalse(UpdateChecker.isNewer("v0.2.9", "0.3.0"));
        Assert.assertTrue(UpdateChecker.isNewer("v0.10.0", "v0.9.9"));
    }

    @Test
    public void testParseBulur() throws Exception {
        String fixture = "{"
                + "\"tag_name\":\"v0.4.0\","
                + "\"body\":\"yenilikler\","
                + "\"assets\":["
                + "{\"name\":\"pchome-mobile-v0.4.0.apk\","
                + " \"browser_download_url\":\"https://x/m.apk\"}"
                + "]}";
        UpdateChecker.UpdateInfo info = UpdateChecker.parse(fixture, "0.3.0");
        Assert.assertTrue(info.available);
        Assert.assertEquals("v0.4.0", info.version);
        Assert.assertEquals("https://x/m.apk", info.apkUrl);
    }

    @Test
    public void testParseGuncelseYok() throws Exception {
        String fixture = "{"
                + "\"tag_name\":\"v0.3.0\","
                + "\"body\":\"\","
                + "\"assets\":["
                + "{\"name\":\"pchome-mobile-v0.3.0.apk\","
                + " \"browser_download_url\":\"https://x/m.apk\"}"
                + "]}";
        UpdateChecker.UpdateInfo info = UpdateChecker.parse(fixture, "0.3.0");
        Assert.assertFalse(info.available);
    }

    @Test
    public void testParseApkYoksaYok() throws Exception {
        String fixture = "{\"tag_name\":\"v9.9.9\",\"assets\":[]}";
        UpdateChecker.UpdateInfo info = UpdateChecker.parse(fixture, "0.3.0");
        Assert.assertFalse(info.available);
    }
}
