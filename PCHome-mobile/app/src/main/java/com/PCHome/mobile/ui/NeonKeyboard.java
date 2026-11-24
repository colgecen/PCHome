package com.PCHome.mobile.ui;

import android.graphics.Color;
import android.graphics.Typeface;
import android.view.Gravity;
import android.widget.Button;
import android.widget.LinearLayout;

import com.PCHome.mobile.control.KeyCodeMap;

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
            {"Z", "X", "C", "V", "B", "N", "M", ",", ".", "/"},
            {"1", "2", "3", "4", "5", "6", "7", "8", "9", "0"},
            {"ESC", "TAB", "CTRL", "SHIFT", "ALT", "SPACE", "ENTER", "DEL"},
    };

    public void build(LinearLayout container, OnKeyListener listener) {
        container.removeAllViews();
        int rowHeight = (int) (48 * container.getResources().getDisplayMetrics().density);
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
                btn.setTextSize(label.length() > 1 ? 12f : 16f);
                btn.setGravity(Gravity.CENTER);
                btn.setMinWidth(0);
                btn.setMinimumWidth(0);
                btn.setMinHeight(0);
                btn.setMinimumHeight(0);
                btn.setPadding(0, 0, 0, 0);
                btn.setBackgroundColor(0xFF00E5FF);
                LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, rowHeight, 1f);
                lp.setMargins(2, 2, 2, 2);
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
            return KeyCodeMap.charToLinux(label.charAt(0));
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
