package com.pchome.mobile;

import android.graphics.Bitmap;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.view.PixelCopy;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowManager;
import android.view.inputmethod.InputMethodManager;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

import androidx.appcompat.app.AppCompatActivity;

import com.pchome.mobile.control.KeyCodeMap;
import com.pchome.mobile.ui.ReticleView;

import org.json.JSONException;
import org.json.JSONObject;
import org.webrtc.SurfaceViewRenderer;
import org.webrtc.VideoTrack;

public class DisplayActivity extends AppCompatActivity implements WebRtcClient.WebRtcListener {

    private static final int MODE_DIRECT = 0;
    private static final int MODE_TRACKPAD = 1;
    private static final float SENSITIVITY = 1.6f;
    private static final float MOVE_THRESHOLD = 12f;
    private static final long LONG_PRESS_MS = 250;
    private static final long DOUBLE_TAP_MS = 300;

    private SurfaceViewRenderer surfaceView;
    private ReticleView reticle;
    private TextView streamStatus;
    private TextView infoChip;
    private LinearLayout keyboardContainer;
    private WebRtcClient webRtcClient;
    private RecentStore recentStore;
    private String pcId = "";
    private String deviceName = "";
    private String deviceIp = "";
    private String deviceMac = "";
    private String deviceRes = "";
    private boolean recentSaved = false;

    private int pcWidth = 1920;
    private int pcHeight = 1080;
    private int mode = MODE_DIRECT;
    private float virtX = pcWidth / 2f;
    private float virtY = pcHeight / 2f;

    private HandlerThread touchThread;
    private Handler touchHandler;
    private Handler uiHandler;

    // Touch gesture state.
    private float downX, downY, lastX, lastY;
    private long downTime;
    private boolean moved;
    private boolean longPressFired;
    private long lastTapTime;
    private float lastCentroidX, lastCentroidY;
    private boolean ctrlSticky;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        setContentView(R.layout.activity_display);

        surfaceView = findViewById(R.id.surface_view);
        reticle = findViewById(R.id.reticle);
        streamStatus = findViewById(R.id.stream_status);
        infoChip = findViewById(R.id.info_chip);
        keyboardContainer = findViewById(R.id.keyboard_container);

        surfaceView.init(null, null);

        uiHandler = new Handler(getMainLooper());
        touchThread = new HandlerThread("touch-controller");
        touchThread.start();
        touchHandler = new Handler(touchThread.getLooper());

        String signalUrl = getIntent().getStringExtra("signalUrl");
        if (signalUrl == null || signalUrl.isEmpty()) {
            signalUrl = "ws://localhost:8080/ws";
        }
        pcId = getIntent().getStringExtra("pcId");
        String password = getIntent().getStringExtra("password");
        if (pcId == null || pcId.isEmpty()) {
            // Eski sürümden gelen "pin" ekine yedek uyumluluk.
            pcId = getIntent().getStringExtra("pin");
        }
        if (pcId == null) pcId = "";
        if (password == null) password = "";
        pcId = pcId.replaceAll("[^0-9]", "");
        password = password.replaceAll("[^0-9]", "");
        infoChip.setText("PC " + pcId);
        recentStore = new RecentStore(this);
        webRtcClient = new WebRtcClient(this, null, surfaceView);
        webRtcClient.setListener(this);
        webRtcClient.connect(signalUrl, pcId, password);

        setupHud();
        setupKeyboard();
        setupTouch();
    }

    private void setupHud() {
        Button btnMode = findViewById(R.id.btn_mode);
        Button btnLeft = findViewById(R.id.btn_left);
        Button btnRight = findViewById(R.id.btn_right);
        Button btnKeyboard = findViewById(R.id.btn_keyboard);
        Button btnRotate = findViewById(R.id.btn_rotate);
        Button btnEsc = findViewById(R.id.btn_esc);
        Button btnCtrl = findViewById(R.id.btn_ctrl);

        btnMode.setOnClickListener(v -> {
            mode = (mode == MODE_DIRECT) ? MODE_TRACKPAD : MODE_DIRECT;
            boolean direct = mode == MODE_DIRECT;
            btnMode.setText(direct ? "DOĞRUDAN" : "TOUCHPAD");
            infoChip.setText("PC " + pcId + " • " + (direct ? "DOĞRUDAN" : "TOUCHPAD"));
            tick();
        });
        btnLeft.setOnClickListener(v -> {
            sendClick("left", "click");
            tick();
        });
        btnRight.setOnClickListener(v -> {
            sendClick("right", "click");
            tick();
        });
        btnRotate.setOnClickListener(v -> rotateScreen());
        btnEsc.setOnClickListener(v -> {
            sendKey(KeyCodeMap.KEY_ESC, true);
            sendKey(KeyCodeMap.KEY_ESC, false);
        });
        btnCtrl.setOnClickListener(v -> {
            if (!ctrlSticky) {
                sendKey(KeyCodeMap.KEY_LEFTCTRL, true);
                ctrlSticky = true;
                btnCtrl.setBackgroundColor(0xFFFF2A55);
            } else {
                sendKey(KeyCodeMap.KEY_LEFTCTRL, false);
                ctrlSticky = false;
                btnCtrl.setBackgroundColor(0xFF00F4FF);
            }
        });

        findViewById(R.id.btn_disconnect).setOnClickListener(v -> {
            saveRecent();
            finish();
        });
    }

    private void setupKeyboard() {
        Button btnKeyboard = findViewById(R.id.btn_keyboard);
        // Use the system IME picker so a real on-screen keyboard can be used
        // for text entry on the remote desktop, instead of the custom grid.
        btnKeyboard.setOnClickListener(v -> {
            InputMethodManager imm =
                    (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
            if (imm != null) {
                imm.showInputMethodPicker();
            }
            int vis = keyboardContainer.getVisibility() == View.VISIBLE ? View.GONE : View.VISIBLE;
            keyboardContainer.setVisibility(vis);
        });
    }

    private void setupTouch() {
        surfaceView.setOnTouchListener((v, e) -> {
            MotionEvent copy = MotionEvent.obtain(e);
            touchHandler.post(() -> {
                processTouch(copy);
                copy.recycle();
            });
            return true;
        });
    }

    private void rotateScreen() {
        int[] orientations = {
                Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2
                        ? android.content.pm.ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE : 0,
                android.content.pm.ActivityInfo.SCREEN_ORIENTATION_REVERSE_LANDSCAPE,
                android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT,
                android.content.pm.ActivityInfo.SCREEN_ORIENTATION_REVERSE_PORTRAIT,
        };
        Integer current = getRequestedOrientation();
        int idx = 0;
        for (int i = 0; i < orientations.length; i++) {
            if (orientations[i] == current) {
                idx = i;
                break;
            }
        }
        setRequestedOrientation(orientations[(idx + 1) % orientations.length]);
    }

    private void processTouch(MotionEvent e) {
        int action = e.getActionMasked();
        float x = e.getX();
        float y = e.getY();
        int viewW = surfaceView.getWidth();
        int viewH = surfaceView.getHeight();
        if (viewW == 0 || viewH == 0) {
            return;
        }

        // Two-finger scroll.
        if (e.getPointerCount() >= 2 && action == MotionEvent.ACTION_MOVE) {
            float cx = (e.getX(0) + e.getX(1)) / 2f;
            float cy = (e.getY(0) + e.getY(1)) / 2f;
            if (lastCentroidX != 0 || lastCentroidY != 0) {
                int dx = (int) ((cx - lastCentroidX) * SENSITIVITY);
                int dy = (int) ((cy - lastCentroidY) * SENSITIVITY);
                sendScroll(-dx, -dy);
            }
            lastCentroidX = cx;
            lastCentroidY = cy;
            return;
        }
        lastCentroidX = 0;
        lastCentroidY = 0;

        if (mode == MODE_DIRECT) {
            processDirect(action, x, y, viewW, viewH);
        } else {
            processTrackpad(action, x, y, viewW, viewH);
        }
    }

    private void processDirect(int action, float x, float y, int viewW, int viewH) {
        float pcX = x / viewW * pcWidth;
        float pcY = y / viewH * pcHeight;
        updateReticle(x, y);
        switch (action) {
            case MotionEvent.ACTION_DOWN:
                downX = x;
                downY = y;
                downTime = System.currentTimeMillis();
                moved = false;
                longPressFired = false;
                sendMoveAbs((int) pcX, (int) pcY);
                sendClick("left", "down");
                touchHandler.postDelayed(() -> {
                    if (!moved && System.currentTimeMillis() - downTime >= LONG_PRESS_MS) {
                        longPressFired = true;
                        sendClick("right", "down");
                    }
                }, LONG_PRESS_MS);
                break;
            case MotionEvent.ACTION_MOVE:
                if (Math.abs(x - downX) > MOVE_THRESHOLD || Math.abs(y - downY) > MOVE_THRESHOLD) {
                    moved = true;
                }
                sendMoveAbs((int) pcX, (int) pcY);
                break;
            case MotionEvent.ACTION_UP:
                updateReticleHide();
                if (longPressFired) {
                    sendClick("right", "up");
                    sendClick("left", "up");
                } else {
                    sendClick("left", "up");
                    long now = System.currentTimeMillis();
                    if (!moved && now - lastTapTime < DOUBLE_TAP_MS) {
                        sendClick("left", "double");
                    }
                    lastTapTime = now;
                }
                break;
            default:
                break;
        }
    }

    private void processTrackpad(int action, float x, float y, int viewW, int viewH) {
        switch (action) {
            case MotionEvent.ACTION_DOWN:
                downX = x;
                downY = y;
                lastX = x;
                lastY = y;
                downTime = System.currentTimeMillis();
                moved = false;
                longPressFired = false;
                touchHandler.postDelayed(() -> {
                    if (!moved && System.currentTimeMillis() - downTime >= LONG_PRESS_MS) {
                        longPressFired = true;
                        sendClick("right", "down");
                    }
                }, LONG_PRESS_MS);
                break;
            case MotionEvent.ACTION_MOVE:
                int dx = (int) ((x - lastX) * SENSITIVITY);
                int dy = (int) ((y - lastY) * SENSITIVITY);
                lastX = x;
                lastY = y;
                if (Math.abs(x - downX) > MOVE_THRESHOLD || Math.abs(y - downY) > MOVE_THRESHOLD) {
                    moved = true;
                }
                virtX = clamp(virtX + dx, 0, pcWidth);
                virtY = clamp(virtY + dy, 0, pcHeight);
                sendMoveRel(dx, dy);
                updateReticle(virtX / pcWidth * viewW, virtY / pcHeight * viewH);
                break;
            case MotionEvent.ACTION_UP:
                if (longPressFired) {
                    sendClick("right", "up");
                } else if (!moved) {
                    sendClick("left", "click");
                }
                break;
            default:
                break;
        }
    }

    private float clamp(float v, float min, float max) {
        return Math.max(min, Math.min(max, v));
    }

    private void updateReticle(float x, float y) {
        uiHandler.post(() -> reticle.setPosition(x, y));
    }

    private void updateReticleHide() {
        uiHandler.post(() -> reticle.setVisible(false));
    }

    private void sendMoveAbs(int x, int y) {
        send(new ControlMessage().type("move_abs").put("x", x).put("y", y).toJson());
    }

    private void sendMoveRel(int dx, int dy) {
        send(new ControlMessage().type("move_rel").put("dx", dx).put("dy", dy).toJson());
    }

    private void sendClick(String button, String action) {
        send(new ControlMessage().type("click").put("button", button).put("action", action).toJson());
    }

    private void sendScroll(int dx, int dy) {
        send(new ControlMessage().type("scroll").put("dx", dx).put("dy", dy).toJson());
    }

    private void sendKey(int code, boolean down) {
        send(new ControlMessage().type("key").put("code", code).put("action", down ? "down" : "up").toJson());
    }

    private void send(String json) {
        if (webRtcClient != null) {
            webRtcClient.sendControl(json);
        }
    }

    @Override
    public boolean onKeyDown(int keyCode, android.view.KeyEvent event) {
        int linux = KeyCodeMap.androidToLinux(keyCode);
        if (linux != -1) {
            sendKey(linux, true);
            return true;
        }
        return super.onKeyDown(keyCode, event);
    }

    @Override
    public boolean onKeyUp(int keyCode, android.view.KeyEvent event) {
        int linux = KeyCodeMap.androidToLinux(keyCode);
        if (linux != -1) {
            sendKey(linux, false);
            return true;
        }
        return super.onKeyUp(keyCode, event);
    }

    @Override
    public void onStateChanged(WebRtcClient.ConnectionState state) {
        runOnUiThread(() -> {
            if (state == WebRtcClient.ConnectionState.CONNECTED) {
                streamStatus.setVisibility(View.GONE);
            } else if (state == WebRtcClient.ConnectionState.DISCONNECTED) {
                saveRecent();
                streamStatus.setText(R.string.disconnected);
                streamStatus.setVisibility(View.VISIBLE);
            } else if (state == WebRtcClient.ConnectionState.FAILED) {
                saveRecent();
                streamStatus.setText(R.string.error);
                streamStatus.setVisibility(View.VISIBLE);
            }
        });
    }

    @Override
    public void onRemoteTrack(org.webrtc.MediaStream stream) {
        runOnUiThread(() -> streamStatus.setVisibility(View.GONE));
    }

    @Override
    public void onDataChannelMessage(org.webrtc.DataChannel.Buffer buffer) {
        try {
            byte[] data = new byte[buffer.data.remaining()];
            buffer.data.get(data);
            String json = new String(data, java.nio.charset.StandardCharsets.UTF_8);
            JSONObject msg = new JSONObject(json);
            if ("display_info".equals(msg.getString("type"))) {
                pcWidth = msg.optInt("width", pcWidth);
                pcHeight = msg.optInt("height", pcHeight);
            }
        } catch (Exception e) {
            // ignore malformed control messages
        }
    }

    @Override
    public void onError(String error) {
        runOnUiThread(() -> {
            streamStatus.setText(getString(R.string.error) + ": " + error);
            streamStatus.setVisibility(View.VISIBLE);
            Toast.makeText(DisplayActivity.this,
                    getString(R.string.error) + ": " + error, Toast.LENGTH_LONG).show();
            // Şifre reddi gibi ölümcül hatalarda kartı kaydedip çık.
            if (error.contains("reddedildi") || error.contains("hatalı")) {
                saveRecent();
                finish();
            }
        });
    }

    @Override
    public void onDeviceInfo(org.json.JSONObject info) {
        deviceName = info.optString("pc_name", deviceName);
        deviceIp = info.optString("ip", deviceIp);
        deviceMac = info.optString("mac", deviceMac);
        deviceRes = info.optString("resolution", deviceRes);
        persistDeviceFields();
        runOnUiThread(this::refreshInfoChip);
    }

    @Override
    public void onDisplayInfo(int width, int height) {
        pcWidth = width;
        pcHeight = height;
        deviceRes = width + "x" + height;
        persistDeviceFields();
        runOnUiThread(this::refreshInfoChip);
    }

    /** Üst bilgi çipi: isim • çözünürlük • kip. */
    private void refreshInfoChip() {
        String name = deviceName.isEmpty() ? ("PC " + pcId) : deviceName;
        String res = deviceRes.isEmpty() ? "" : (" • " + deviceRes);
        String kip = mode == MODE_DIRECT ? "DOĞRUDAN" : "TOUCHPAD";
        infoChip.setText(name + res + " • " + kip);
    }

    /** Tuşlara hafif dokunsal geri bildirim (25ms). */
    private void tick() {
        try {
            android.os.Vibrator vib =
                    (android.os.Vibrator) getSystemService(VIBRATOR_SERVICE);
            if (vib == null || !vib.hasVibrator()) {
                return;
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                vib.vibrate(android.os.VibrationEffect.createOneShot(
                        25, android.os.VibrationEffect.DEFAULT_AMPLITUDE));
            } else {
                vib.vibrate(25);
            }
        } catch (Exception e) {
            // Titreşim yoksa sessiz geç.
        }
    }

    /** Cihaz kartı alanlarını beklemeden diske yazar (görüntü kapanışta eklenir). */
    private void persistDeviceFields() {
        if (pcId.isEmpty() || recentStore == null) {
            return;
        }
        RecentPc entry = recentStore.find(pcId);
        if (entry == null) {
            entry = new RecentPc(pcId, deviceName);
        }
        if (!deviceName.isEmpty()) entry.pcName = deviceName;
        if (!deviceIp.isEmpty()) entry.ip = deviceIp;
        if (!deviceMac.isEmpty()) entry.mac = deviceMac;
        if (!deviceRes.isEmpty()) entry.resolution = deviceRes;
        recentStore.upsert(entry);
    }

    /**
     * "Son Girilenler" kartını tamamlar: ekrandaki son karenin küçük resmini
     * JPEG olarak kaydedip cihaz bilgileriyle birlikte başa taşır.
     * Tek seferliktir (idempotent).
     */
    private void saveRecent() {
        if (recentSaved || pcId.isEmpty() || recentStore == null) {
            return;
        }
        recentSaved = true;
        persistDeviceFields();
        try {
            int w = surfaceView.getWidth();
            int h = surfaceView.getHeight();
            if (w <= 0 || h <= 0 || surfaceView.getHolder().getSurface() == null
                    || !surfaceView.getHolder().getSurface().isValid()) {
                return;
            }
            Bitmap full = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888);
            PixelCopy.request(surfaceView.getHolder().getSurface(), full,
                    copyResult -> {
                        if (copyResult == PixelCopy.SUCCESS) {
                            try {
                                int tw = 320;
                                int th = Math.max(1, (int) ((long) h * tw / Math.max(1, w)));
                                Bitmap small = Bitmap.createScaledBitmap(full, tw, th, true);
                                java.io.File dest = recentStore.thumbnailFile(pcId);
                                java.io.FileOutputStream out =
                                        new java.io.FileOutputStream(dest);
                                small.compress(Bitmap.CompressFormat.JPEG, 80, out);
                                out.close();
                                small.recycle();
                                RecentPc entry = recentStore.find(pcId);
                                if (entry == null) {
                                    entry = new RecentPc(pcId, deviceName);
                                }
                                entry.thumbnail = dest.getAbsolutePath();
                                entry.lastSeen = System.currentTimeMillis();
                                recentStore.upsert(entry);
                            } catch (Exception e) {
                                // Görüntü kaydedilemezse kart bilgisiz kalır.
                            }
                        }
                        full.recycle();
                    }, uiHandler);
        } catch (Exception e) {
            // PixelCopy yoksa sessiz geç (kart yine de durur).
        }
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        saveRecent();
        if (webRtcClient != null) {
            webRtcClient.disconnect();
        }
        if (touchThread != null) {
            touchThread.quitSafely();
        }
        if (surfaceView != null) {
            surfaceView.release();
        }
    }

    /// Small builder for control JSON payloads.
    private static class ControlMessage {
        private final JSONObject obj = new JSONObject();

        ControlMessage type(String t) {
            try {
                obj.put("type", t);
            } catch (JSONException ignore) {
            }
            return this;
        }

        ControlMessage put(String k, int v) {
            try {
                obj.put(k, v);
            } catch (JSONException ignore) {
            }
            return this;
        }

        ControlMessage put(String k, String v) {
            try {
                obj.put(k, v);
            } catch (JSONException ignore) {
            }
            return this;
        }

        String toJson() {
            return obj.toString();
        }
    }
}
