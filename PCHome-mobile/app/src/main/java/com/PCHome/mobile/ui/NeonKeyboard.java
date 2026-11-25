package com.PCHome.mobile.ui;

import android.content.res.ColorStateList;
import android.graphics.Typeface;
import android.graphics.drawable.ColorDrawable;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.RippleDrawable;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.view.HapticFeedbackConstants;
import android.view.MotionEvent;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.PopupWindow;

import com.PCHome.mobile.control.KeyCodeMap;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/// Telefon QWERTY'sine benzeyen iki sayfalı uzak klavye.
/// 1) Harfler + Caps/Shift/Ctrl/Alt/Esc/Tab/Del/Enter/Space
/// 2) Üstte sayılar, altında semboller ve karakterler
///
/// Tuşlar Linux keycode'u olarak gönderilir; Shift gerektiren sembollerde
/// listener'a `withShift=true` geçilir, gerçek Shift'i DisplayActivity basar.
public class NeonKeyboard {

    public interface OnKeyListener {
        void onKey(int linuxCode, boolean down, boolean withShift);

        void onToggleModifier(int linuxCode);
    }

    private enum Kind {
        NORMAL,
        MODIFIER,
        CAPS,
        PAGE
    }

    private static class KeyDef {
        final String label;
        final int code;
        final float weight;
        final Kind kind;
        final boolean withShift;
        final int page; // yalnız Kind.PAGE için

        KeyDef(String label, int code, float weight, Kind kind, boolean withShift, int page) {
            this.label = label;
            this.code = code;
            this.weight = weight;
            this.kind = kind;
            this.withShift = withShift;
            this.page = page;
        }
    }

    private static class RowDef {
        final KeyDef[] keys;

        RowDef(KeyDef... keys) {
            this.keys = keys;
        }
    }

    private LinearLayout container;
    private OnKeyListener listener;
    private float density;
    private int page = 0;
    private boolean capsActive;
    private boolean shiftActive;
    private boolean ctrlActive;
    private boolean altActive;
    private final List<Button> shiftButtons = new ArrayList<>();
    private final List<Button> ctrlButtons = new ArrayList<>();
    private final List<Button> altButtons = new ArrayList<>();
    private Button capsButton;
    private PopupWindow altPopup;
    private final Handler altHandler = new Handler(Looper.getMainLooper());

    public void build(LinearLayout container, OnKeyListener listener) {
        this.container = container;
        this.listener = listener;
        this.density = container.getResources().getDisplayMetrics().density;
        buildPage();
    }

    /** Shift/Ctrl/Alt gibi yapışkan tuşların görünümünü günceller. */
    public void setModifierActive(int code, boolean active) {
        List<Button> buttons;
        if (code == KeyCodeMap.KEY_LEFTSHIFT) {
            buttons = shiftButtons;
            shiftActive = active;
        } else if (code == KeyCodeMap.KEY_LEFTCTRL) {
            buttons = ctrlButtons;
            ctrlActive = active;
        } else if (code == KeyCodeMap.KEY_LEFTALT) {
            buttons = altButtons;
            altActive = active;
        } else {
            return;
        }
        for (Button button : buttons) {
            applyActive(button, active);
        }
    }

    private void buildPage() {
        if (container == null) {
            return;
        }
        dismissAltPopup();
        altHandler.removeCallbacksAndMessages(null);
        container.removeAllViews();
        shiftButtons.clear();
        ctrlButtons.clear();
        altButtons.clear();
        capsButton = null;

        for (RowDef row : page == 0 ? letterRows() : symbolRows()) {
            addRow(row);
        }
        applyModifierStates();
    }

    private RowDef[] letterRows() {
        return new RowDef[]{
                new RowDef(
                        key("ESC", KeyCodeMap.KEY_ESC, 1f),
                        key("TAB", KeyCodeMap.KEY_TAB, 1f),
                        mod("CTRL", KeyCodeMap.KEY_LEFTCTRL, 1f),
                        mod("ALT", KeyCodeMap.KEY_LEFTALT, 1f),
                        key("DEL", KeyCodeMap.KEY_BACKSPACE, 1f)),
                new RowDef(
                        letter("Q"), letter("W"), letter("E"), letter("R"), letter("T"),
                        letter("Y"), letter("U"), letter("I"), letter("O"), letter("P")),
                new RowDef(
                        letter("A"), letter("S"), letter("D"), letter("F"), letter("G"),
                        letter("H"), letter("J"), letter("K"), letter("L")),
                new RowDef(
                        mod("SHIFT", KeyCodeMap.KEY_LEFTSHIFT, 1.35f),
                        letter("Z"), letter("X"), letter("C"), letter("V"),
                        letter("B"), letter("N"), letter("M"),
                        key("DEL", KeyCodeMap.KEY_BACKSPACE, 1.35f)),
                new RowDef(
                        pageKey("?123", 1, 1.2f),
                        capsKey(),
                        key("BOŞLUK", KeyCodeMap.KEY_SPACE, 3.4f),
                        key(".", KeyCodeMap.KEY_DOT, 1f),
                        key("ENTER", KeyCodeMap.KEY_ENTER, 1.6f)),
        };
    }

    private RowDef[] symbolRows() {
        return new RowDef[]{
                new RowDef(
                        sym("1"), sym("2"), sym("3"), sym("4"), sym("5"),
                        sym("6"), sym("7"), sym("8"), sym("9"), sym("0")),
                new RowDef(
                        sym("-"), sym("/"), sym(":"), sym(";"), sym("("), sym(")"),
                        sym("$"), sym("&"), sym("@"), sym("\"")),
                new RowDef(
                        sym("#"), sym("+"), sym("="), sym("%"), sym("!"), sym("?"),
                        sym("*"), sym("'"), sym("~")),
                new RowDef(
                        sym(","), sym("."), sym("_"), sym("\\"), sym("|"),
                        sym("<"), sym(">"), sym("^")),
                new RowDef(
                        pageKey("ABC", 0, 1.35f),
                        key("DEL", KeyCodeMap.KEY_BACKSPACE, 1.1f),
                        key("BOŞLUK", KeyCodeMap.KEY_SPACE, 3.4f),
                        key(".", KeyCodeMap.KEY_DOT, 1f),
                        key("ENTER", KeyCodeMap.KEY_ENTER, 1.6f)),
        };
    }

    private void addRow(RowDef row) {
        LinearLayout rowLayout = new LinearLayout(container.getContext());
        rowLayout.setOrientation(LinearLayout.HORIZONTAL);
        rowLayout.setGravity(Gravity.CENTER);
        int rowHeight = Math.round(38 * density);
        for (KeyDef key : row.keys) {
            Button btn = new Button(container.getContext());
            boolean special = key.kind != Kind.NORMAL || key.label.length() > 1;
            btn.setText(key.label);
            btn.setAllCaps(false);
            btn.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
            btn.setTextColor(special ? 0xFFA1A1AA : 0xFFE4E4E7);
            btn.setTextSize(key.label.length() > 1 ? 9.5f : 14f);
            btn.setGravity(Gravity.CENTER);
            btn.setMinWidth(0);
            btn.setMinimumWidth(0);
            btn.setMinHeight(0);
            btn.setMinimumHeight(0);
            btn.setPadding(0, 0, 0, 0);
            btn.setBackground(keyBackground(special));
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                    0, rowHeight, key.weight);
            int gap = Math.round(2.5f * density);
            lp.setMargins(gap, gap, gap, gap);
            btn.setLayoutParams(lp);

            if (key.kind == Kind.MODIFIER) {
                if (key.code == KeyCodeMap.KEY_LEFTSHIFT) {
                    shiftButtons.add(btn);
                } else if (key.code == KeyCodeMap.KEY_LEFTCTRL) {
                    ctrlButtons.add(btn);
                } else if (key.code == KeyCodeMap.KEY_LEFTALT) {
                    altButtons.add(btn);
                }
            } else if (key.kind == Kind.CAPS) {
                capsButton = btn;
                applyActive(btn, capsActive);
            }

            String[] alts = key.kind == Kind.NORMAL
                    ? turkishAlternatives(key.label) : null;
            if (alts != null && alts.length > 0) {
                final boolean[] longPressed = {false};
                final Runnable longPress = () -> {
                    longPressed[0] = true;
                    btn.performHapticFeedback(HapticFeedbackConstants.LONG_PRESS);
                    showAltPopup(btn, alts);
                };
                btn.setOnTouchListener((v, event) -> {
                    int action = event.getActionMasked();
                    if (action == MotionEvent.ACTION_DOWN) {
                        longPressed[0] = false;
                        altHandler.postDelayed(longPress, 380L);
                    } else if (action == MotionEvent.ACTION_UP
                            || action == MotionEvent.ACTION_CANCEL) {
                        altHandler.removeCallbacks(longPress);
                        if (action == MotionEvent.ACTION_UP
                                && !longPressed[0] && listener != null) {
                            listener.onKey(key.code, true, key.withShift);
                            listener.onKey(key.code, false, key.withShift);
                        }
                    }
                    return true;
                });
            } else {
                btn.setOnTouchListener((v, event) -> {
                    int action = event.getActionMasked();
                    if (action == MotionEvent.ACTION_DOWN) {
                        handleKeyDown(key);
                    } else if (action == MotionEvent.ACTION_UP
                            || action == MotionEvent.ACTION_CANCEL) {
                        handleKeyUp(key);
                    }
                    return true;
                });
            }
            rowLayout.addView(btn);
        }
        container.addView(rowLayout);
    }

    private void handleKeyDown(KeyDef key) {
        if (listener == null) {
            return;
        }
        switch (key.kind) {
            case PAGE:
                page = key.page;
                buildPage();
                break;
            case MODIFIER:
                listener.onToggleModifier(key.code);
                break;
            case CAPS:
                capsActive = !capsActive;
                applyActive(capsButton, capsActive);
                listener.onKey(KeyCodeMap.KEY_CAPSLOCK, true, false);
                listener.onKey(KeyCodeMap.KEY_CAPSLOCK, false, false);
                break;
            case NORMAL:
            default:
                listener.onKey(key.code, true, key.withShift);
                break;
        }
    }

    private void handleKeyUp(KeyDef key) {
        if (listener != null && key.kind == Kind.NORMAL) {
            listener.onKey(key.code, false, key.withShift);
        }
    }

    private String[] turkishAlternatives(String label) {
        if (label == null || label.length() != 1) {
            return null;
        }
        switch (label.toLowerCase(Locale.ROOT)) {
            case "c":
                return new String[]{"ç"};
            case "g":
                return new String[]{"ğ"};
            case "i":
                return new String[]{"ı"};
            case "o":
                return new String[]{"ö"};
            case "s":
                return new String[]{"ş"};
            case "u":
                return new String[]{"ü"};
            default:
                return null;
        }
    }

    /// Türkçe Q klavye fiziksel tuş karşılıkları.
    private int altCode(String alt) {
        switch (alt) {
            case "ç":
                return KeyCodeMap.KEY_DOT;
            case "ğ":
                return KeyCodeMap.KEY_LEFTBRACE;
            case "ı":
                return KeyCodeMap.KEY_I;
            case "ö":
                return KeyCodeMap.KEY_COMMA;
            case "ş":
                return KeyCodeMap.KEY_SEMICOLON;
            case "ü":
                return KeyCodeMap.KEY_RIGHTBRACE;
            default:
                return -1;
        }
    }

    private void showAltPopup(Button anchor, String[] alts) {
        dismissAltPopup();
        if (container == null) {
            return;
        }
        LinearLayout row = new LinearLayout(container.getContext());
        row.setOrientation(LinearLayout.HORIZONTAL);
        int pad = Math.round(6 * density);
        row.setPadding(pad, pad, pad, pad);
        GradientDrawable bg = new GradientDrawable();
        bg.setColor(0xFF1F1F23);
        bg.setCornerRadius(12 * density);
        row.setBackground(bg);

        for (String alt : alts) {
            Button choice = new Button(container.getContext());
            choice.setText(alt);
            choice.setTextSize(18f);
            choice.setTextColor(0xFFF4F4F5);
            choice.setAllCaps(false);
            choice.setGravity(Gravity.CENTER);
            choice.setMinWidth(0);
            choice.setMinimumWidth(0);
            choice.setMinHeight(0);
            choice.setMinimumHeight(0);
            choice.setPadding(0, 0, 0, 0);
            GradientDrawable choiceBg = new GradientDrawable();
            choiceBg.setColor(0xFF2A2A30);
            choiceBg.setCornerRadius(9 * density);
            choice.setBackground(choiceBg);
            int size = Math.round(46 * density);
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(size, size);
            int gap = Math.round(4 * density);
            lp.setMargins(gap, 0, gap, 0);
            choice.setLayoutParams(lp);
            choice.setOnClickListener(v -> {
                int code = altCode(alt);
                if (code > 0 && listener != null) {
                    listener.onKey(code, true, false);
                    listener.onKey(code, false, false);
                }
                dismissAltPopup();
            });
            row.addView(choice);
        }

        row.measure(
                View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED),
                View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED));
        int width = row.getMeasuredWidth();
        int height = row.getMeasuredHeight();
        int[] loc = new int[2];
        anchor.getLocationOnScreen(loc);
        int x = loc[0] + anchor.getWidth() / 2 - width / 2;
        int y = loc[1] - height - Math.round(8 * density);
        if (y < 0) {
            y = loc[1] + anchor.getHeight() + Math.round(8 * density);
        }
        altPopup = new PopupWindow(row, width, height, true);
        altPopup.setOutsideTouchable(true);
        altPopup.setBackgroundDrawable(new ColorDrawable(0x00000000));
        altPopup.showAtLocation(anchor, Gravity.NO_GRAVITY, Math.max(0, x), y);
    }

    private void dismissAltPopup() {
        if (altPopup != null) {
            altPopup.dismiss();
            altPopup = null;
        }
    }

    private void applyActive(Button button, boolean active) {
        if (button == null) {
            return;
        }
        GradientDrawable bg = new GradientDrawable();
        bg.setColor(active ? 0xFF2A2A30 : 0xFF1F1F23);
        bg.setCornerRadius(9 * density);
        button.setBackground(new RippleDrawable(
                ColorStateList.valueOf(0x1FFFFFFF), bg, null));
        button.setTextColor(active ? 0xFFF4F4F5 : 0xFFA1A1AA);
    }

    private void applyModifierStates() {
        for (Button button : shiftButtons) {
            applyActive(button, shiftActive);
        }
        for (Button button : ctrlButtons) {
            applyActive(button, ctrlActive);
        }
        for (Button button : altButtons) {
            applyActive(button, altActive);
        }
    }

    private KeyDef key(String label, int code, float weight) {
        return new KeyDef(label, code, weight, Kind.NORMAL, false, -1);
    }

    private KeyDef letter(String label) {
        return new KeyDef(label, KeyCodeMap.charToLinux(label.charAt(0)),
                1f, Kind.NORMAL, false, -1);
    }

    private KeyDef sym(String label) {
        return new KeyDef(label, KeyCodeMap.charToLinux(label.charAt(0)),
                1f, Kind.NORMAL, KeyCodeMap.needsShift(label.charAt(0)), -1);
    }

    private KeyDef mod(String label, int code, float weight) {
        return new KeyDef(label, code, weight, Kind.MODIFIER, false, -1);
    }

    private KeyDef capsKey() {
        return new KeyDef("CAPS", KeyCodeMap.KEY_CAPSLOCK, 1f, Kind.CAPS, false, -1);
    }

    private KeyDef pageKey(String label, int page, float weight) {
        return new KeyDef(label, -1, weight, Kind.PAGE, false, page);
    }

    /** Koyu cam tuş + yumuşak ripple. */
    private RippleDrawable keyBackground(boolean special) {
        GradientDrawable bg = new GradientDrawable();
        bg.setColor(special ? 0xFF1F1F23 : 0xFF18181B);
        bg.setCornerRadius(9 * density);
        return new RippleDrawable(
                ColorStateList.valueOf(0x1FFFFFFF), bg, null);
    }
}
