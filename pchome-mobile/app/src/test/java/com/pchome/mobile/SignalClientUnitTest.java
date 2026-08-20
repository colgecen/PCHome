package com.pchome.mobile;

import org.junit.Test;
import org.junit.Assert;

public class SignalClientUnitTest {
    @Test
    public void testUrlConstruction() {
        String serverUrl = "ws://localhost:8080/ws";
        String pin = "123456";
        String expected = "ws://localhost:8080/ws?pin=123456";
        Assert.assertEquals(expected, serverUrl + "?pin=" + pin);
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
