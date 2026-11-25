package com.PCHome.mobile.control;

import android.view.KeyEvent;

/// Maps Android key codes / characters to Linux key codes (input-event-codes.h)
/// so keystrokes captured on the phone can be injected into the PC kernel.
///
/// The Linux KEY_* values are NOT contiguous alphabetically. Mapping letters
/// with arithmetic (`KEY_A + offset`) silently turns W into comma, E into G,
/// etc. — use the explicit tables below.
public final class KeyCodeMap {

    private KeyCodeMap() {
    }

    // Selected Linux KEY_* codes.
    public static final int KEY_ESC = 1;
    public static final int KEY_1 = 2;
    public static final int KEY_0 = 11;
    public static final int KEY_MINUS = 12;
    public static final int KEY_EQUAL = 13;
    public static final int KEY_BACKSPACE = 14;
    public static final int KEY_TAB = 15;
    public static final int KEY_Q = 16;
    public static final int KEY_W = 17;
    public static final int KEY_E = 18;
    public static final int KEY_R = 19;
    public static final int KEY_T = 20;
    public static final int KEY_Y = 21;
    public static final int KEY_U = 22;
    public static final int KEY_I = 23;
    public static final int KEY_O = 24;
    public static final int KEY_P = 25;
    public static final int KEY_LEFTBRACE = 26;
    public static final int KEY_RIGHTBRACE = 27;
    public static final int KEY_ENTER = 28;
    public static final int KEY_LEFTCTRL = 29;
    public static final int KEY_A = 30;
    public static final int KEY_S = 31;
    public static final int KEY_D = 32;
    public static final int KEY_F = 33;
    public static final int KEY_G = 34;
    public static final int KEY_H = 35;
    public static final int KEY_J = 36;
    public static final int KEY_K = 37;
    public static final int KEY_L = 38;
    public static final int KEY_SEMICOLON = 39;
    public static final int KEY_APOSTROPHE = 40;
    public static final int KEY_GRAVE = 41;
    public static final int KEY_LEFTSHIFT = 42;
    public static final int KEY_BACKSLASH = 43;
    public static final int KEY_Z = 44;
    public static final int KEY_X = 45;
    public static final int KEY_C = 46;
    public static final int KEY_V = 47;
    public static final int KEY_B = 48;
    public static final int KEY_N = 49;
    public static final int KEY_M = 50;
    public static final int KEY_COMMA = 51;
    public static final int KEY_DOT = 52;
    public static final int KEY_SLASH = 53;
    public static final int KEY_LEFTALT = 56;
    public static final int KEY_SPACE = 57;
    public static final int KEY_CAPSLOCK = 58;
    public static final int KEY_F1 = 59;
    public static final int KEY_F10 = 68;
    public static final int KEY_F11 = 87;
    public static final int KEY_F12 = 88;
    public static final int KEY_HOME = 102;
    public static final int KEY_UP = 103;
    public static final int KEY_PAGEUP = 104;
    public static final int KEY_LEFT = 105;
    public static final int KEY_RIGHT = 106;
    public static final int KEY_END = 107;
    public static final int KEY_DOWN = 108;
    public static final int KEY_PAGEDOWN = 109;
    public static final int KEY_INSERT = 110;
    public static final int KEY_DELETE = 111;

    /// Linux key code for the lowercase/uppercase letter or printable ASCII
    /// character, or -1 if unsupported.
    private static final int[] LETTERS = {
            30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50,
            49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44
    };
    private static final int[] DIGITS = {11, 2, 3, 4, 5, 6, 7, 8, 9, 10};

    public static int charToLinux(char c) {
        if (c >= 'a' && c <= 'z') {
            return LETTERS[c - 'a'];
        }
        if (c >= 'A' && c <= 'Z') {
            return LETTERS[c - 'A'];
        }
        if (c >= '0' && c <= '9') {
            return DIGITS[c - '0'];
        }
        switch (c) {
            case ' ': return KEY_SPACE;
            case '\n':
            case '\r': return KEY_ENTER;
            case '\t': return KEY_TAB;
            case '-': return KEY_MINUS;
            case '_': return KEY_MINUS;
            case '=': return KEY_EQUAL;
            case '+': return KEY_EQUAL;
            case '[': return KEY_LEFTBRACE;
            case '{': return KEY_LEFTBRACE;
            case ']': return KEY_RIGHTBRACE;
            case '}': return KEY_RIGHTBRACE;
            case '\\': return KEY_BACKSLASH;
            case '|': return KEY_BACKSLASH;
            case ';': return KEY_SEMICOLON;
            case ':': return KEY_SEMICOLON;
            case '\'': return KEY_APOSTROPHE;
            case '"': return KEY_APOSTROPHE;
            case '`': return KEY_GRAVE;
            case '~': return KEY_GRAVE;
            case ',': return KEY_COMMA;
            case '<': return KEY_COMMA;
            case '.': return KEY_DOT;
            case '>': return KEY_DOT;
            case '/': return KEY_SLASH;
            case '?': return KEY_SLASH;
            case '!': return KEY_1;
            case '@': return 3; // KEY_2
            case '#': return 4; // KEY_3
            case '$': return 5; // KEY_4
            case '%': return 6; // KEY_5
            case '^': return 7; // KEY_6
            case '&': return 8; // KEY_7
            case '*': return 9; // KEY_8
            case '(': return 10; // KEY_9
            case ')': return KEY_0;
            default:
                return -1;
        }
    }

    /// True when the character needs Shift on a US layout.
    public static boolean needsShift(char c) {
        if (c >= 'A' && c <= 'Z') {
            return true;
        }
        return "~!@#$%^&*()_+{}|:\"<>?".indexOf(c) >= 0;
    }

    public static boolean isModifier(int linuxCode) {
        return linuxCode == KEY_LEFTSHIFT
                || linuxCode == KEY_LEFTCTRL
                || linuxCode == KEY_LEFTALT;
    }

    /// Map an Android `KeyEvent` key code to a Linux key code (or -1 if unknown).
    public static int androidToLinux(int androidKeyCode) {
        if (androidKeyCode >= KeyEvent.KEYCODE_A && androidKeyCode <= KeyEvent.KEYCODE_Z) {
            return LETTERS[androidKeyCode - KeyEvent.KEYCODE_A];
        }
        if (androidKeyCode >= KeyEvent.KEYCODE_0 && androidKeyCode <= KeyEvent.KEYCODE_9) {
            return DIGITS[androidKeyCode - KeyEvent.KEYCODE_0];
        }
        switch (androidKeyCode) {
            case KeyEvent.KEYCODE_NUMPAD_0: return KEY_0;
            case KeyEvent.KEYCODE_NUMPAD_1: return KEY_1;
            case KeyEvent.KEYCODE_NUMPAD_2: return 3;
            case KeyEvent.KEYCODE_NUMPAD_3: return 4;
            case KeyEvent.KEYCODE_NUMPAD_4: return 5;
            case KeyEvent.KEYCODE_NUMPAD_5: return 6;
            case KeyEvent.KEYCODE_NUMPAD_6: return 7;
            case KeyEvent.KEYCODE_NUMPAD_7: return 8;
            case KeyEvent.KEYCODE_NUMPAD_8: return 9;
            case KeyEvent.KEYCODE_NUMPAD_9: return 10;
            case KeyEvent.KEYCODE_ESCAPE: return KEY_ESC;
            case KeyEvent.KEYCODE_SPACE: return KEY_SPACE;
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER: return KEY_ENTER;
            case KeyEvent.KEYCODE_DEL:
            case KeyEvent.KEYCODE_FORWARD_DEL: return KEY_BACKSPACE;
            case KeyEvent.KEYCODE_TAB: return KEY_TAB;
            case KeyEvent.KEYCODE_SHIFT_LEFT:
            case KeyEvent.KEYCODE_SHIFT_RIGHT: return KEY_LEFTSHIFT;
            case KeyEvent.KEYCODE_CTRL_LEFT:
            case KeyEvent.KEYCODE_CTRL_RIGHT: return KEY_LEFTCTRL;
            case KeyEvent.KEYCODE_ALT_LEFT:
            case KeyEvent.KEYCODE_ALT_RIGHT: return KEY_LEFTALT;
            case KeyEvent.KEYCODE_DPAD_UP: return KEY_UP;
            case KeyEvent.KEYCODE_DPAD_DOWN: return KEY_DOWN;
            case KeyEvent.KEYCODE_DPAD_LEFT: return KEY_LEFT;
            case KeyEvent.KEYCODE_DPAD_RIGHT: return KEY_RIGHT;
            case KeyEvent.KEYCODE_MOVE_HOME: return KEY_HOME;
            case KeyEvent.KEYCODE_MOVE_END: return KEY_END;
            case KeyEvent.KEYCODE_PAGE_UP: return KEY_PAGEUP;
            case KeyEvent.KEYCODE_PAGE_DOWN: return KEY_PAGEDOWN;
            case KeyEvent.KEYCODE_INSERT: return KEY_INSERT;
            case KeyEvent.KEYCODE_GRAVE: return KEY_GRAVE;
            case KeyEvent.KEYCODE_MINUS: return KEY_MINUS;
            case KeyEvent.KEYCODE_EQUALS: return KEY_EQUAL;
            case KeyEvent.KEYCODE_COMMA: return KEY_COMMA;
            case KeyEvent.KEYCODE_PERIOD: return KEY_DOT;
            case KeyEvent.KEYCODE_SLASH: return KEY_SLASH;
            case KeyEvent.KEYCODE_SEMICOLON: return KEY_SEMICOLON;
            case KeyEvent.KEYCODE_APOSTROPHE: return KEY_APOSTROPHE;
            case KeyEvent.KEYCODE_LEFT_BRACKET: return KEY_LEFTBRACE;
            case KeyEvent.KEYCODE_RIGHT_BRACKET: return KEY_RIGHTBRACE;
            case KeyEvent.KEYCODE_BACKSLASH: return KEY_BACKSLASH;
            case KeyEvent.KEYCODE_F1: return KEY_F1;
            case KeyEvent.KEYCODE_F2: return KEY_F1 + 1;
            case KeyEvent.KEYCODE_F3: return KEY_F1 + 2;
            case KeyEvent.KEYCODE_F4: return KEY_F1 + 3;
            case KeyEvent.KEYCODE_F5: return KEY_F1 + 4;
            case KeyEvent.KEYCODE_F6: return KEY_F1 + 5;
            case KeyEvent.KEYCODE_F7: return KEY_F1 + 6;
            case KeyEvent.KEYCODE_F8: return KEY_F1 + 7;
            case KeyEvent.KEYCODE_F9: return KEY_F1 + 8;
            case KeyEvent.KEYCODE_F10: return KEY_F10;
            case KeyEvent.KEYCODE_F11: return KEY_F11;
            case KeyEvent.KEYCODE_F12: return KEY_F12;
            default:
                return -1;
        }
    }
}
