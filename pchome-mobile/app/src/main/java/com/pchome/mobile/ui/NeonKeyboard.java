package com.pchome.mobile.ui;

import android.graphics.Color;
import android.graphics.Typeface;
import android.view.Gravity;
import android.widget.Button;
import android.widget.LinearLayout;

import com.pchome.mobile.control.KeyCodeMap;

/// Builds a compact cyber-themed QWERTY soft keyboard on top of a vertical
/// container. Each key press forwards a Linux keycode (down on press, up on
/// release) through the supplied listener.
public class NeonKeyboard {

    public interface OnKeyListener {
        void onKey(int linuxCode, boolean down);
    }

    private final String[][] ROWS = {
            {"Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"},
            {"A", "S", "D", "F", "G", "H", "J", "K", "L"},
            {"Z", "X", "C", "V", "B", "N", "M"},
            {"1", "2", "3", "4", "5", "6", "7", "8", "9", "0"},
            {"ESC", "TAB", "CTRL", "SHIFT", "ALT", "SPACE", "ENTER", "DEL"},
    };

    public void build(LinearLayout container, OnKeyListener listener) {
        container.removeAllViews();
        for (String[] row : ROWS) {
            LinearLayout rowLayout = new LinearLayout(container.getContext());
            rowLayout.setOrientation(LinearLayout.HORIZONTAL);
            rowLayout.setGravity(Gravity.CENTER);
            for (String label : row) {
                int code = linuxCodeForLabel(label);
                Button btn = new Button(container.getContext());
                btn.setText(label);
                btn.setAllCaps(true);
                btn.setTypeface(Typeface.MONOSPACE);
                btn.setTextColor(Color.BLACK);
                btn.setBackgroundColor(0xFF00F4FF);
                LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT);
                lp.setMargins(4, 4, 4, 4);
                btn.setLayoutParams(lp);
                btn.setOnTouchListener((v, event) -> {
                    int action = event.getActionMasked();
                    if (action == android.view.MotionEvent.ACTION_DOWN) {
                        listener.onKey(code, true);
                    } else if (action == android.view.MotionEvent.ACTION_UP
                            || action == android.view.MotionEvent.ACTION_CANCEL) {
                        listener.onKey(code, false);
                    }
                    return true;
                });
                rowLayout.addView(btn);
            }
            container.addView(rowLayout);
        }
    }

    private int linuxCodeForLabel(String label) {
        if (label.length() == 1) {
            char c = label.charAt(0);
            if (c >= 'A' && c <= 'Z') {
                return KeyCodeMap.KEY_A + (c - 'A');
            }
            if (c >= 'a' && c <= 'z') {
                return KeyCodeMap.KEY_A + (c - 'a');
            }
            if (c >= '1' && c <= '9') {
                return KeyCodeMap.KEY_1 + (c - '1');
            }
            if (c == '0') {
                return KeyCodeMap.KEY_0;
            }
        }
        switch (label) {
            case "ESC":
                return KeyCodeMap.KEY_ESC;
            case "TAB":
                return KeyCodeMap.KEY_TAB;
            case "CTRL":
                return KeyCodeMap.KEY_LEFTCTRL;
            case "SHIFT":
                return KeyCodeMap.KEY_LEFTSHIFT;
            case "ALT":
                return KeyCodeMap.KEY_LEFTALT;
            case "SPACE":
                return KeyCodeMap.KEY_SPACE;
            case "ENTER":
                return KeyCodeMap.KEY_ENTER;
            case "DEL":
                return KeyCodeMap.KEY_BACKSPACE;
            default:
                return -1;
        }
    }
}
