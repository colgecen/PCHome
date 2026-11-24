package com.PCHome.mobile.ui;

import android.content.res.ColorStateList;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.RippleDrawable;
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
        float density = container.getResources().getDisplayMetrics().density;
        int rowHeight = (int) (40 * density);
        for (String[] row : ROWS) {
            LinearLayout rowLayout = new LinearLayout(container.getContext());
            rowLayout.setOrientation(LinearLayout.HORIZONTAL);
            rowLayout.setGravity(Gravity.CENTER);
            for (String label : row) {
                int code = linuxCodeForLabel(label);
                Button btn = new Button(container.getContext());
                boolean special = label.length() > 1 && !"SPACE".equals(label);
                btn.setText(label);
                btn.setAllCaps(true);
                btn.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
                btn.setTextColor(special ? 0xFF67E8F9 : 0xFFF1F5F9);
                btn.setTextSize(label.length() > 1 ? 10f : 14f);
                btn.setGravity(Gravity.CENTER);
                btn.setMinWidth(0);
                btn.setMinimumWidth(0);
                btn.setMinHeight(0);
                btn.setMinimumHeight(0);
                btn.setPadding(0, 0, 0, 0);
                btn.setBackground(keyBackground(density, special));
                LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, rowHeight, 1f);
                int gap = Math.round(2.5f * density);
                lp.setMargins(gap, gap, gap, gap);
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

    /** Koyu cam tuş + neon ripple; harfler beyaz, özel tuşlar camgöbeği. */
    private RippleDrawable keyBackground(float density, boolean special) {
        GradientDrawable bg = new GradientDrawable();
        bg.setColor(special ? 0xFF122033 : 0xFF0B1422);
        bg.setCornerRadius(9 * density);
        bg.setStroke(Math.round(1 * density), 0xFF1E2A3E);
        return new RippleDrawable(
                ColorStateList.valueOf(0x5522D3EE), bg, null);
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
