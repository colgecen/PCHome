package com.PCHome.mobile.control;

import android.view.KeyEvent;

import org.junit.Assert;
import org.junit.Test;

public class KeyCodeMapTest {

    @Test
    public void harflerDogruLinuxKodlarinaGider() {
        // Linux KEY_* değerleri alfabetik olarak bitişik DEĞİLDİR.
        Assert.assertEquals(30, KeyCodeMap.charToLinux('a')); // KEY_A
        Assert.assertEquals(17, KeyCodeMap.charToLinux('w')); // KEY_W (eskiden virgül oluyordu)
        Assert.assertEquals(18, KeyCodeMap.charToLinux('e')); // KEY_E
        Assert.assertEquals(44, KeyCodeMap.charToLinux('z')); // KEY_Z
        Assert.assertEquals(45, KeyCodeMap.charToLinux('x')); // KEY_X
        Assert.assertEquals(50, KeyCodeMap.charToLinux('m')); // KEY_M
        Assert.assertEquals(2, KeyCodeMap.charToLinux('1'));
        Assert.assertEquals(11, KeyCodeMap.charToLinux('0'));
        Assert.assertEquals(57, KeyCodeMap.charToLinux(' '));
    }

    @Test
    public void androidTuslariDogruEslenir() {
        Assert.assertEquals(30, KeyCodeMap.androidToLinux(KeyEvent.KEYCODE_A));
        Assert.assertEquals(17, KeyCodeMap.androidToLinux(KeyEvent.KEYCODE_W));
        Assert.assertEquals(18, KeyCodeMap.androidToLinux(KeyEvent.KEYCODE_E));
        Assert.assertEquals(44, KeyCodeMap.androidToLinux(KeyEvent.KEYCODE_Z));
        Assert.assertEquals(45, KeyCodeMap.androidToLinux(KeyEvent.KEYCODE_X));
    }

    @Test
    public void degistiricilerTaninir() {
        Assert.assertTrue(KeyCodeMap.isModifier(KeyCodeMap.KEY_LEFTSHIFT));
        Assert.assertTrue(KeyCodeMap.isModifier(KeyCodeMap.KEY_LEFTCTRL));
        Assert.assertFalse(KeyCodeMap.isModifier(KeyCodeMap.KEY_A));
    }
}
