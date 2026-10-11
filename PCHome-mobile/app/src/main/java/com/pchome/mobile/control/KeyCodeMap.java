package com.pchome.mobile.control;

import android.view.KeyEvent;

/// Maps Android key codes / characters to Linux key codes (input-event-codes.h)
/// so keystrokes captured on the phone can be injected into the PC kernel.
public final class KeyCodeMap {

    private KeyCodeMap() {
    }

    // Selected Linux KEY_* codes.
    public static final int KEY_ESC = 1;
    public static final int KEY_1 = 2;
    public static final int KEY_0 = 11;
    public static final int KEY_BACKSPACE = 14;
    public static final int KEY_TAB = 15;
    public static final int KEY_Q = 16;
    public static final int KEY_LEFTCTRL = 29;
    public static final int KEY_A = 30;
    public static final int KEY_Z = 44;
    public static final int KEY_SPACE = 57;
    public static final int KEY_LEFTALT = 56;
    public static final int KEY_LEFTSHIFT = 42;
    public static final int KEY_ENTER = 28;
    public static final int KEY_UP = 103;
    public static final int KEY_LEFT = 105;
    public static final int KEY_RIGHT = 106;
    public static final int KEY_DOWN = 108;

    /// Map an Android `KeyEvent` key code to a Linux key code (or -1 if unknown).
    public static int androidToLinux(int androidKeyCode) {
        switch (androidKeyCode) {
            case KeyEvent.KEYCODE_ESCAPE:
                return KEY_ESC;
            case KeyEvent.KEYCODE_0:
            case KeyEvent.KEYCODE_NUMPAD_0:
                return KEY_0;
            case KeyEvent.KEYCODE_1:
            case KeyEvent.KEYCODE_NUMPAD_1:
                return KEY_1;
            case KeyEvent.KEYCODE_2:
            case KeyEvent.KEYCODE_NUMPAD_2:
                return KEY_1 + 1;
            case KeyEvent.KEYCODE_3:
            case KeyEvent.KEYCODE_NUMPAD_3:
                return KEY_1 + 2;
            case KeyEvent.KEYCODE_4:
            case KeyEvent.KEYCODE_NUMPAD_4:
                return KEY_1 + 3;
            case KeyEvent.KEYCODE_5:
            case KeyEvent.KEYCODE_NUMPAD_5:
                return KEY_1 + 4;
            case KeyEvent.KEYCODE_6:
            case KeyEvent.KEYCODE_NUMPAD_6:
                return KEY_1 + 5;
            case KeyEvent.KEYCODE_7:
            case KeyEvent.KEYCODE_NUMPAD_7:
                return KEY_1 + 6;
            case KeyEvent.KEYCODE_8:
            case KeyEvent.KEYCODE_NUMPAD_8:
                return KEY_1 + 7;
            case KeyEvent.KEYCODE_9:
            case KeyEvent.KEYCODE_NUMPAD_9:
                return KEY_1 + 8;
            case KeyEvent.KEYCODE_A:
                return KEY_A;
            case KeyEvent.KEYCODE_B:
                return KEY_A + 1;
            case KeyEvent.KEYCODE_C:
                return KEY_A + 2;
            case KeyEvent.KEYCODE_D:
                return KEY_A + 3;
            case KeyEvent.KEYCODE_E:
                return KEY_A + 4;
            case KeyEvent.KEYCODE_F:
                return KEY_A + 5;
            case KeyEvent.KEYCODE_G:
                return KEY_A + 6;
            case KeyEvent.KEYCODE_H:
                return KEY_A + 7;
            case KeyEvent.KEYCODE_I:
                return KEY_A + 8;
            case KeyEvent.KEYCODE_J:
                return KEY_A + 9;
            case KeyEvent.KEYCODE_K:
                return KEY_A + 10;
            case KeyEvent.KEYCODE_L:
                return KEY_A + 11;
            case KeyEvent.KEYCODE_M:
                return KEY_A + 12;
            case KeyEvent.KEYCODE_N:
                return KEY_A + 13;
            case KeyEvent.KEYCODE_O:
                return KEY_A + 14;
            case KeyEvent.KEYCODE_P:
                return KEY_A + 15;
            case KeyEvent.KEYCODE_Q:
                return KEY_Q;
            case KeyEvent.KEYCODE_R:
                return KEY_A + 17;
            case KeyEvent.KEYCODE_S:
                return KEY_A + 18;
            case KeyEvent.KEYCODE_T:
                return KEY_A + 19;
            case KeyEvent.KEYCODE_U:
                return KEY_A + 20;
            case KeyEvent.KEYCODE_V:
                return KEY_A + 21;
            case KeyEvent.KEYCODE_W:
                return KEY_A + 22;
            case KeyEvent.KEYCODE_X:
                return KEY_Z;
            case KeyEvent.KEYCODE_Y:
                return KEY_A + 24;
            case KeyEvent.KEYCODE_Z:
                return KEY_A + 25;
            case KeyEvent.KEYCODE_SPACE:
                return KEY_SPACE;
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER:
                return KEY_ENTER;
            case KeyEvent.KEYCODE_DEL:
            case KeyEvent.KEYCODE_FORWARD_DEL:
                return KEY_BACKSPACE;
            case KeyEvent.KEYCODE_TAB:
                return KEY_TAB;
            case KeyEvent.KEYCODE_SHIFT_LEFT:
            case KeyEvent.KEYCODE_SHIFT_RIGHT:
                return KEY_LEFTSHIFT;
            case KeyEvent.KEYCODE_CTRL_LEFT:
            case KeyEvent.KEYCODE_CTRL_RIGHT:
                return KEY_LEFTCTRL;
            case KeyEvent.KEYCODE_ALT_LEFT:
            case KeyEvent.KEYCODE_ALT_RIGHT:
                return KEY_LEFTALT;
            case KeyEvent.KEYCODE_DPAD_UP:
                return KEY_UP;
            case KeyEvent.KEYCODE_DPAD_DOWN:
                return KEY_DOWN;
            case KeyEvent.KEYCODE_DPAD_LEFT:
                return KEY_LEFT;
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                return KEY_RIGHT;
            case KeyEvent.KEYCODE_GRAVE:
                return 41; // KEY_GRAVE
            case KeyEvent.KEYCODE_MINUS:
                return 12; // KEY_MINUS
            case KeyEvent.KEYCODE_EQUALS:
                return 13; // KEY_EQUAL
            case KeyEvent.KEYCODE_COMMA:
                return 51; // KEY_COMMA
            case KeyEvent.KEYCODE_PERIOD:
                return 52; // KEY_DOT
            case KeyEvent.KEYCODE_SLASH:
                return 53; // KEY_SLASH
            case KeyEvent.KEYCODE_SEMICOLON:
                return 39; // KEY_SEMICOLON
            case KeyEvent.KEYCODE_APOSTROPHE:
                return 40; // KEY_APOSTROPHE
            case KeyEvent.KEYCODE_LEFT_BRACKET:
                return 26; // KEY_LEFTBRACE
            case KeyEvent.KEYCODE_RIGHT_BRACKET:
                return 27; // KEY_RIGHTBRACE
            case KeyEvent.KEYCODE_BACKSLASH:
                return 43; // KEY_BACKSLASH
            default:
                return -1;
        }
    }
}
