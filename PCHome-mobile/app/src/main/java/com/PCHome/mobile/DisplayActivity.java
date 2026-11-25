package com.PCHome.mobile;

import android.content.res.ColorStateList;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;
import android.util.Log;
import android.view.PixelCopy;
import android.view.MotionEvent;
import android.view.ScaleGestureDetector;
import android.view.View;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.ImageButton;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

import androidx.appcompat.app.AlertDialog;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.graphics.Insets;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.core.view.WindowInsetsControllerCompat;

import com.PCHome.mobile.control.KeyCodeMap;
import com.PCHome.mobile.ui.NeonKeyboard;
import com.PCHome.mobile.ui.ReticleView;
import com.PCHome.mobile.ui.ScrollRailView;
import com.google.android.material.button.MaterialButton;

import org.json.JSONException;
import org.json.JSONObject;
import org.webrtc.SurfaceViewRenderer;
import org.webrtc.VideoTrack;

public class DisplayActivity extends AppCompatActivity implements WebRtcClient.WebRtcListener {

    private static final int MODE_DIRECT = 0;
    private static final int MODE_TRACKPAD = 1;
    /** İki parmak ve sağ şerit için: 1 dp ≈ 20 yüksek çözünürlüklü birim. */
    private static final float SCROLL_UNITS_PER_DP = 20f;
    /** Trackpad'de imleç ile parmak arasındaki görsel boşluk (dp). */
    private static final float TRACKPAD_GAP_DP = 56f;
    /** Dikey ön-gitme: parmak yukarıdayken yukarı, aşağıdayken aşağı. */
    private static final float TRACKPAD_LEAD_RATIO = 0.35f;
    private static final float TRACKPAD_LEAD_MAX_DP = 28f;
    private static final float MOVE_THRESHOLD = 12f;
    private static final long LONG_PRESS_MS = 500;
    private static final long DOUBLE_TAP_MS = 300;
    private static final long CONNECT_TIMEOUT_MS = 25000;

    private SurfaceViewRenderer surfaceView;
    private ReticleView reticle;
    private TextView streamStatus;
    private TextView infoChip;
    private TextView infoTitle;
    private View connDot;
    private View emptyGrid;
    private View topOverlay;
    private View controlBar;
    private LinearLayout keyboardContainer;
    private ScrollRailView scrollRail;
    private NeonKeyboard softKeyboard;
    private ImageButton btnHideMenu;
    private Button btnKeyboard;
    private MaterialButton btnWheel;
    private ImageButton btnZoom;
    private ImageButton btnHand;
    private boolean menuHidden;
    private boolean zoomMode;
    private float zoomScale = 1f;
    private float zoomPanX;
    private float zoomPanY;
    private float zoomLastTouchX;
    private float zoomLastTouchY;
    private ScaleGestureDetector scaleDetector;
    private boolean topInfoVisible = true;
    private float twoFingerScrollCarryX;
    private float twoFingerScrollCarryY;
    private float scrollFlingVelocity;
    private long scrollFlingLast;
    private WebRtcClient webRtcClient;
    private RecentStore recentStore;
    private String pcId = "";
    private String signalUrl = "";
    private String password = "";
    private String deviceName = "";
    private String deviceIp = "";
    private String deviceMac = "";
    private String deviceRes = "";
    private boolean recentSaved = false;
    private boolean connectedOnce = false;
    private boolean fatalShown = false;
    private boolean firstFrameRendered = false;
    private AlertDialog fatalDialog;
    private final Runnable timeoutCheck = () -> {
        if (!connectedOnce && !isFinishing()) {
            Diag.phase(DisplayActivity.this, "zaman-aşımı");
            showFatal(
                    "PC cevap vermedi",
                    "Sinyale ulaşıldı ama PC'den yanıt yok.\n\n"
                            + "• PC uygulaması açık mı?\n"
                            + "• İki tarafta da aynı sinyal adresi mi?\n"
                            + "• PC ID doğru mu?");
        }
    };

    private int pcWidth = 1920;
    private int pcHeight = 1080;
    /** Klavye açıkken control_bar büyür; video alanı bu taban yüksekliği kullanır. */
    private int dockBaseHeight = 0;
    private int mode = MODE_DIRECT;
    private boolean leftHanded;
    private float virtX = pcWidth / 2f;
    private float virtY = pcHeight / 2f;
    private float trackpadLeadScreen;

    private HandlerThread touchThread;
    private Handler touchHandler;
    private Handler uiHandler;
    private boolean surfaceInitialized;

    // Touch gesture state.
    private float downX, downY, lastX, lastY;
    private long downTime;
    private boolean moved;
    private boolean longPressFired;
    private volatile boolean pressed;
    private long lastTapTime;
    private float lastCentroidX, lastCentroidY;
    private boolean ctrlSticky;
    private boolean shiftSticky;
    private boolean altSticky;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        if (getSupportActionBar() != null) {
            getSupportActionBar().hide();
        }
        Diag.phase(this, "ekran-açıldı");
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        enterImmersive();
        setContentView(R.layout.activity_display);

        surfaceView = findViewById(R.id.surface_view);
        reticle = findViewById(R.id.reticle);
        streamStatus = findViewById(R.id.stream_status);
        infoChip = findViewById(R.id.info_chip);
        infoTitle = findViewById(R.id.info_title);
        connDot = findViewById(R.id.conn_dot);
        emptyGrid = findViewById(R.id.empty_grid);
        topOverlay = findViewById(R.id.top_overlay);
        controlBar = findViewById(R.id.control_bar);
        keyboardContainer = findViewById(R.id.keyboard_container);
        scrollRail = findViewById(R.id.scroll_rail);
        btnHideMenu = findViewById(R.id.btn_hide_menu);
        btnZoom = findViewById(R.id.btn_zoom);
        btnHand = findViewById(R.id.btn_hand);

        applyHudInsets();
        topOverlay.setOnTouchListener((v, event) -> {
            if (event.getActionMasked() == MotionEvent.ACTION_DOWN) {
                hideTopInfoOnTouch();
            }
            return false;
        });

        // Video alanı; üst/alt HUD yükseklikleri değişince (döndürme,
        // klavye, inset) yeniden hesaplanmalı.
        View parent = (View) surfaceView.getParent();
        parent.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> applyAspectLayout());
        topOverlay.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> applyAspectLayout());
        controlBar.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> applyAspectLayout());
        surfaceView.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> updateReticleRect());


        uiHandler = new Handler(getMainLooper());
        touchThread = new HandlerThread("touch-controller");
        touchThread.start();
        touchHandler = new Handler(touchThread.getLooper());

        String signalUrlExtra = getIntent().getStringExtra("signalUrl");
        if (signalUrlExtra == null || signalUrlExtra.isEmpty()) {
            signalUrlExtra = "ws://localhost:8080/ws";
        }
        signalUrl = signalUrlExtra;
        pcId = getIntent().getStringExtra("pcId");
        password = getIntent().getStringExtra("password");
        if (pcId == null || pcId.isEmpty()) {
            // Eski sürümden gelen "pin" ekine yedek uyumluluk.
            pcId = getIntent().getStringExtra("pin");
        }
        if (pcId == null) pcId = "";
        if (password == null) password = "";
        leftHanded = getIntent().getBooleanExtra("leftHanded", false);
        updateHandButton();
        pcId = pcId.replaceAll("[^0-9]", "");
        password = password.replaceAll("[^0-9]", "");
        infoChip.setText("PC " + pcId);
        streamStatus.setText(R.string.conn_phase_signal);
        streamStatus.setVisibility(View.VISIBLE);
        recentStore = new RecentStore(this);
        try {
            webRtcClient = new WebRtcClient(this, null, surfaceView);
            webRtcClient.setListener(this);
            // Renderer, decoder ile AYNI EGL bağlamını paylaşmalı; aksi hâlde
            // doku kimlikleri geçersiz olur ve ekran siyah kalır.
            surfaceView.init(webRtcClient.getEglBaseContext(),
                    new org.webrtc.RendererCommon.RendererEvents() {
                        @Override
                        public void onFirstFrameRendered() {
                            runOnUiThread(() -> {
                                firstFrameRendered = true;
                                markConnected();
                                streamStatus.setVisibility(View.GONE);
                                if (emptyGrid != null) emptyGrid.setVisibility(View.GONE);
                                Diag.phase(DisplayActivity.this, "ilk-kare");
                                surfaceView.post(() -> {
                                    if (surfaceView.getWidth() > 0 && surfaceView.getHeight() > 0) {
                                        reticle.setPosition(
                                                surfaceView.getLeft() + surfaceView.getWidth() / 2f,
                                                surfaceView.getTop() + surfaceView.getHeight() / 2f);
                                    }
                                });
                            });
                        }

                        @Override
                        public void onFrameResolutionChanged(int videoWidth, int videoHeight, int rotation) {
                            // Yayın boyutu ölçekli olabilir; dokunma eşlemesi
                            // display_info'daki native çözünürlüğü kullanır.
                        }
                    });
            surfaceView.setScalingType(org.webrtc.RendererCommon.ScalingType.SCALE_ASPECT_FIT);
            surfaceView.setEnableHardwareScaler(true);
            surfaceInitialized = true;
            surfaceView.post(this::applyAspectLayout);
            webRtcClient.connect(signalUrl, pcId, password);
            Diag.phase(this, "rtc-başlatıldı");
        } catch (Exception e) {
            Diag.phase(this, "rtc-hata: " + e.getMessage());
            showFatal("Başlatılamadı",
                    "Medya motoru kurulamadı:\n" + String.valueOf(e.getMessage()));
            return;
        }
        uiHandler.postDelayed(timeoutCheck, CONNECT_TIMEOUT_MS);

        setupHud();
        setupKeyboard();
        setupTouch();
    }

    @Override
    public void onConfigurationChanged(android.content.res.Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        if (surfaceView != null) {
            // Yeni ölçüler oturduktan SONRA hesapla; aksi hâlde eski
            // portre boyutlarıyla hesaplanan video şeridi ekranda kalıyordu.
            surfaceView.post(this::applyAspectLayout);
            surfaceView.postDelayed(this::applyAspectLayout, 120L);
        }
    }

    /** Tam ekran, immersive; durum/navigasyon çubukları kaydırmayla gelir. */
    private void enterImmersive() {
        WindowCompat.setDecorFitsSystemWindows(getWindow(), false);
        getWindow().setStatusBarColor(Color.TRANSPARENT);
        getWindow().setNavigationBarColor(Color.TRANSPARENT);
        WindowInsetsControllerCompat controller =
                WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        controller.hide(WindowInsetsCompat.Type.systemBars());
        controller.setSystemBarsBehavior(
                WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
    }

    /** HUD'u çentik/durum çubuğu güvenli alanına yerleştir. */
    private void applyHudInsets() {
        View root = findViewById(android.R.id.content);
        ViewCompat.setOnApplyWindowInsetsListener(root, (v, insets) -> {
            Insets cut = insets.getInsets(WindowInsetsCompat.Type.displayCutout());
            Insets bars = insets.getInsets(WindowInsetsCompat.Type.systemBars());
            topOverlay.setPadding(56 + cut.left, 12 + bars.top + cut.top,
                    16 + cut.right, 16);
            controlBar.setPadding(10 + cut.left, 9, 10 + cut.right, 9 + bars.bottom);
            if (btnZoom != null) {
                FrameLayout.LayoutParams lp = (FrameLayout.LayoutParams) btnZoom.getLayoutParams();
                lp.leftMargin = 8 + cut.left;
                lp.topMargin = 8 + bars.top + cut.top;
                btnZoom.setLayoutParams(lp);
            }
            v.post(this::applyAspectLayout);
            return insets;
        });
    }

    /** Gerçek video dikdörtgenini (letterbox dahil) reticle'a bildir. */
    private void updateReticleRect() {
        if (surfaceView == null || reticle == null) {
            return;
        }
        reticle.setVideoRect(
                surfaceView.getLeft(),
                surfaceView.getTop(),
                surfaceView.getLeft() + surfaceView.getWidth(),
                surfaceView.getTop() + surfaceView.getHeight());
    }

    private void setupHud() {
        Button btnMode = findViewById(R.id.btn_mode);
        Button btnLeft = findViewById(R.id.btn_left);
        Button btnRight = findViewById(R.id.btn_right);
        Button btnCopy = findViewById(R.id.btn_copy);
        Button btnPaste = findViewById(R.id.btn_paste);
        btnKeyboard = findViewById(R.id.btn_keyboard);
        btnWheel = findViewById(R.id.btn_wheel);
        Button btnRotate = findViewById(R.id.btn_rotate);
        Button btnRefresh = findViewById(R.id.btn_refresh);

        btnMode.setOnClickListener(v -> {
            mode = (mode == MODE_DIRECT) ? MODE_TRACKPAD : MODE_DIRECT;
            boolean direct = mode == MODE_DIRECT;
            btnMode.setText(direct ? R.string.mode_direct : R.string.mode_trackpad);
            if (btnMode instanceof MaterialButton) {
                ((MaterialButton) btnMode).setIconResource(
                        direct ? R.drawable.ic_touch : R.drawable.ic_mouse);
            }
            refreshInfoChip();
            // Masaüstü "Kip" satırı ilk hareketi beklemeden güncellensin.
            sendMode();
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
        btnCopy.setOnClickListener(v -> {
            sendCopyPaste(KeyCodeMap.KEY_C);
            tick();
        });
        btnPaste.setOnClickListener(v -> {
            sendCopyPaste(KeyCodeMap.KEY_V);
            tick();
        });
        btnWheel.setOnClickListener(v -> {
            toggleScrollRail();
            tick();
        });
        btnHideMenu.setOnClickListener(v -> {
            toggleMenu();
            tick();
        });
        btnZoom.setOnClickListener(v -> {
            toggleZoomMode();
            tick();
        });
        btnZoom.setOnLongClickListener(v -> {
            resetZoom();
            return true;
        });
        btnHand.setOnClickListener(v -> {
            toggleHandedness();
            tick();
        });
        scaleDetector = new ScaleGestureDetector(this,
                new ScaleGestureDetector.SimpleOnScaleGestureListener() {
                    @Override
                    public boolean onScale(ScaleGestureDetector detector) {
                        if (!zoomMode) {
                            return false;
                        }
                        zoomScale = clamp(zoomScale * detector.getScaleFactor(), 1f, 3f);
                        applyAspectLayout();
                        return true;
                    }
                });
        scaleDetector.setQuickScaleEnabled(false);
        scaleDetector.setStylusScaleEnabled(false);
        btnRotate.setOnClickListener(v -> rotateScreen());
        btnRefresh.setOnClickListener(v -> {
            if (webRtcClient != null) {
                webRtcClient.refreshStream();
            }
            streamStatus.setText(R.string.refreshing_stream);
            streamStatus.setVisibility(View.VISIBLE);
            Toast.makeText(this, R.string.refreshing_stream, Toast.LENGTH_SHORT).show();
            tick();
        });

        findViewById(R.id.btn_disconnect).setOnClickListener(v -> {
            saveRecent();
            finish();
        });

        scrollRail.setListener(new ScrollRailView.Listener() {
            @Override
            public void onStart() {
                stopScrollFling();
            }

            @Override
            public void onScroll(int hiResDy) {
                stopScrollFling();
                sendScrollHiRes(0, hiResDy);
            }

            @Override
            public void onFling(float unitsPerSecond) {
                startScrollFling(unitsPerSecond);
            }
        });
    }

    /** Alt menüyü göster/gizle; video alanı boşalan yeri doldurur. */
    private void toggleMenu() {
        menuHidden = !menuHidden;
        controlBar.setVisibility(menuHidden ? View.GONE : View.VISIBLE);
        if (menuHidden && keyboardContainer.getVisibility() == View.VISIBLE) {
            keyboardContainer.setVisibility(View.GONE);
            btnKeyboard.setText(R.string.keyboard);
            if (btnHand != null) {
                btnHand.setVisibility(View.VISIBLE);
            }
        }
        if (menuHidden && scrollRail.getVisibility() == View.VISIBLE) {
            scrollRail.setVisibility(View.GONE);
            setWheelActive(false);
        }
        btnHideMenu.setImageResource(menuHidden ? R.drawable.ic_eye : R.drawable.ic_eye_off);
        btnHideMenu.setContentDescription(getString(
                menuHidden ? R.string.show_menu : R.string.hide_menu));
        if (scrollRail != null) {
            FrameLayout.LayoutParams lp = (FrameLayout.LayoutParams) scrollRail.getLayoutParams();
            lp.bottomMargin = Math.round((menuHidden ? 24f : 130f)
                    * getResources().getDisplayMetrics().density);
            scrollRail.setLayoutParams(lp);
        }
        if (btnHand != null) {
            FrameLayout.LayoutParams lp = (FrameLayout.LayoutParams) btnHand.getLayoutParams();
            lp.bottomMargin = Math.round((menuHidden ? 24f : 120f)
                    * getResources().getDisplayMetrics().density);
            btnHand.setLayoutParams(lp);
        }
        applyAspectLayout();
        surfaceView.postDelayed(this::applyAspectLayout, 120L);
    }

    /** Sağdaki kaydırma şeridini aç/kapat. */
    private void toggleScrollRail() {
        boolean show = scrollRail.getVisibility() != View.VISIBLE;
        scrollRail.setVisibility(show ? View.VISIBLE : View.GONE);
        setWheelActive(show);
        if (!show) {
            stopScrollFling();
        }
    }

    private void setWheelActive(boolean active) {
        if (btnWheel == null) {
            return;
        }
        btnWheel.setBackgroundTintList(ColorStateList.valueOf(
                active ? 0xFF2A2A30 : 0xFF18181B));
        btnWheel.setTextColor(active ? 0xFFF4F4F5 : 0xFFA1A1AA);
        btnWheel.setIconTint(ColorStateList.valueOf(
                active ? 0xFFF4F4F5 : 0xFFA1A1AA));
    }

    private void toggleZoomMode() {
        zoomMode = !zoomMode;
        btnZoom.setImageTintList(ColorStateList.valueOf(
                zoomMode ? 0xFFF4F4F5 : 0xFFA1A1AA));
        if (zoomMode) {
            hideTopInfoNow();
            if (scrollRail.getVisibility() == View.VISIBLE) {
                scrollRail.setVisibility(View.GONE);
                setWheelActive(false);
            }
            reticle.setVisible(false);
        } else {
            reticle.setVisible(true);
            if (surfaceView != null && surfaceView.getWidth() > 0 && surfaceView.getHeight() > 0) {
                float[] r = videoRect(surfaceView.getWidth(), surfaceView.getHeight());
                updateReticle(r[0] + virtX / pcWidth * r[2],
                        r[1] + virtY / pcHeight * r[3]);
            }
            scheduleTopInfo();
        }
        applyAspectLayout();
    }

    private void resetZoom() {
        zoomScale = 1f;
        zoomPanX = 0f;
        zoomPanY = 0f;
        applyAspectLayout();
        Toast.makeText(this, R.string.zoom_reset, Toast.LENGTH_SHORT).show();
    }

    private void toggleHandedness() {
        leftHanded = !leftHanded;
        new Prefs(this).setLeftHanded(leftHanded);
        updateHandButton();
        Toast.makeText(this, leftHanded ? R.string.hand_left : R.string.hand_right,
                Toast.LENGTH_SHORT).show();
    }

    private void updateHandButton() {
        if (btnHand == null) {
            return;
        }
        btnHand.setScaleX(leftHanded ? -1f : 1f);
        btnHand.setContentDescription(getString(
                leftHanded ? R.string.hand_left : R.string.hand_right));
        btnHand.setImageTintList(ColorStateList.valueOf(
                leftHanded ? 0xFFF4F4F5 : 0xFFA1A1AA));
    }

    private void hideTopInfoNow() {
        topInfoVisible = false;
        if (uiHandler != null) {
            uiHandler.removeCallbacks(showTopInfoRunnable);
        }
        if (topOverlay != null) {
            topOverlay.animate().cancel();
            topOverlay.setVisibility(View.INVISIBLE);
        }
    }

    private void hideTopInfoOnTouch() {
        if (zoomMode) {
            hideTopInfoNow();
            return;
        }
        if (topInfoVisible) {
            topInfoVisible = false;
            topOverlay.animate().cancel();
            topOverlay.animate()
                    .alpha(0f)
                    .setDuration(140L)
                    .withEndAction(() -> {
                        if (!topInfoVisible) {
                            topOverlay.setVisibility(View.INVISIBLE);
                        }
                    })
                    .start();
        }
        if (uiHandler != null) {
            uiHandler.removeCallbacks(showTopInfoRunnable);
        }
    }

    private void scheduleTopInfo() {
        if (uiHandler == null) {
            return;
        }
        uiHandler.removeCallbacks(showTopInfoRunnable);
        uiHandler.postDelayed(showTopInfoRunnable, 1600L);
    }

    private final Runnable showTopInfoRunnable = () -> {
        if (zoomMode || topOverlay == null) {
            return;
        }
        topInfoVisible = true;
        topOverlay.setVisibility(View.VISIBLE);
        topOverlay.animate().alpha(1f).setDuration(180L).start();
    };

    private void sendCopyPaste(int keyCode) {
        if (ctrlSticky) {
            sendKey(keyCode, true);
            sendKey(keyCode, false);
        } else {
            sendCombo(keyCode, "ctrl");
        }
    }

    private void setupKeyboard() {
        // Telefon QWERTY'sine benzeyen iki sayfalı özel klavye; Linux
        // keycode'larını control DataChannel üzerinden gönderir.
        softKeyboard = new NeonKeyboard();
        softKeyboard.build(keyboardContainer, new NeonKeyboard.OnKeyListener() {
            @Override
            public void onKey(int code, boolean down, boolean withShift) {
                onSoftKey(code, down, withShift);
            }

            @Override
            public void onToggleModifier(int code) {
                toggleModifier(code);
            }
        });
        btnKeyboard.setOnClickListener(v -> {
            boolean show = keyboardContainer.getVisibility() != View.VISIBLE;
            keyboardContainer.setVisibility(show ? View.VISIBLE : View.GONE);
            btnKeyboard.setText(show ? "GİZLE" : getString(R.string.keyboard));
            if (btnHand != null) {
                btnHand.setVisibility(show ? View.GONE : View.VISIBLE);
            }
            if (show && scrollRail != null && scrollRail.getVisibility() == View.VISIBLE) {
                scrollRail.setVisibility(View.GONE);
                setWheelActive(false);
            }
            applyAspectLayout();
            surfaceView.postDelayed(this::applyAspectLayout, 120L);
        });
    }

    /** Ekran klavyesinden gelen tuş: değiştiriciler yapışkan, diğerleri anlık. */
    private void onSoftKey(int code, boolean down, boolean withShift) {
        if (code < 0) {
            return;
        }
        if (KeyCodeMap.isModifier(code)) {
            if (down) {
                toggleModifier(code);
            }
            return;
        }
        // Sembol sayfasındaki @, #, : gibi tuşlar Shift gerektirir.
        if (withShift && !shiftSticky) {
            if (down) {
                sendCombo(code, "shift");
            }
            return;
        }
        sendKey(code, down);
    }

    private void toggleModifier(int code) {
        boolean active;
        if (code == KeyCodeMap.KEY_LEFTSHIFT) {
            shiftSticky = !shiftSticky;
            active = shiftSticky;
        } else if (code == KeyCodeMap.KEY_LEFTCTRL) {
            ctrlSticky = !ctrlSticky;
            active = ctrlSticky;
        } else {
            altSticky = !altSticky;
            active = altSticky;
        }
        if (softKeyboard != null) {
            softKeyboard.setModifierActive(code, active);
        }
        sendKey(code, active);
        tick();
    }

    private void releaseModifiers() {
        if (shiftSticky) sendKey(KeyCodeMap.KEY_LEFTSHIFT, false);
        if (ctrlSticky) sendKey(KeyCodeMap.KEY_LEFTCTRL, false);
        if (altSticky) sendKey(KeyCodeMap.KEY_LEFTALT, false);
        shiftSticky = ctrlSticky = altSticky = false;
        if (softKeyboard != null) {
            softKeyboard.setModifierActive(KeyCodeMap.KEY_LEFTSHIFT, false);
            softKeyboard.setModifierActive(KeyCodeMap.KEY_LEFTCTRL, false);
            softKeyboard.setModifierActive(KeyCodeMap.KEY_LEFTALT, false);
        }
    }

    private void setupTouch() {
        reticle.setOnTouchListener((v, e) -> {
            if (!zoomMode) {
                return false;
            }
            if (e.getActionMasked() == MotionEvent.ACTION_DOWN) {
                hideTopInfoOnTouch();
            }
            handleZoomTouch(e);
            return true;
        });
        surfaceView.setOnTouchListener((v, e) -> {
            int touchAction = e.getActionMasked();
            if (touchAction == MotionEvent.ACTION_DOWN) {
                hideTopInfoOnTouch();
            }
            if (zoomMode) {
                return true;
            }
            if (touchAction == MotionEvent.ACTION_UP
                    || touchAction == MotionEvent.ACTION_CANCEL) {
                scheduleTopInfo();
            }
            MotionEvent copy = MotionEvent.obtain(e);
            touchHandler.post(() -> {
                processTouch(copy);
                copy.recycle();
            });
            return true;
        });
    }

    /** Zoom modunda tek parmak pan, iki parmak pinch. Normal kontroller kapalı. */
    private void handleZoomTouch(MotionEvent event) {
        if (scaleDetector == null) {
            return;
        }
        int action = event.getActionMasked();
        if (action == MotionEvent.ACTION_DOWN
                || action == MotionEvent.ACTION_POINTER_DOWN
                || action == MotionEvent.ACTION_POINTER_UP) {
            zoomLastTouchX = event.getRawX();
            zoomLastTouchY = event.getRawY();
        }
        scaleDetector.onTouchEvent(event);
        if (event.getPointerCount() == 1 && !scaleDetector.isInProgress()) {
            float x = event.getRawX();
            float y = event.getRawY();
            switch (action) {
                case MotionEvent.ACTION_DOWN:
                    zoomLastTouchX = x;
                    zoomLastTouchY = y;
                    break;
                case MotionEvent.ACTION_MOVE:
                    zoomPanX += x - zoomLastTouchX;
                    zoomPanY += y - zoomLastTouchY;
                    zoomLastTouchX = x;
                    zoomLastTouchY = y;
                    applyAspectLayout();
                    break;
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    scheduleTopInfo();
                    break;
                default:
                    break;
            }
        }
    }

    /** Yatay ↔ dikey geçişi; 180° ters çevirme yapmaz. */
    private void rotateScreen() {
        int current = getRequestedOrientation();
        boolean portrait = current == android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
                || current == android.content.pm.ActivityInfo.SCREEN_ORIENTATION_REVERSE_PORTRAIT
                || current == android.content.pm.ActivityInfo.SCREEN_ORIENTATION_SENSOR_PORTRAIT;
        setRequestedOrientation(portrait
                ? android.content.pm.ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
                : android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);
        tick();
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
        Log.d("PCHomeTouch", "touch action=" + action + " at " + x + "," + y
                + " view=" + viewW + "x" + viewH);

        // Two-finger scroll: yüksek çözünürlüklü tekerlek birimleriyle akıcı.
        if (e.getPointerCount() >= 2 && action == MotionEvent.ACTION_MOVE) {
            float cx = (e.getX(0) + e.getX(1)) / 2f;
            float cy = (e.getY(0) + e.getY(1)) / 2f;
            if (lastCentroidX != 0 || lastCentroidY != 0) {
                float density = getResources().getDisplayMetrics().density;
                float unitsPerPx = SCROLL_UNITS_PER_DP / Math.max(1f, density);
                twoFingerScrollCarryX += -(cx - lastCentroidX) * unitsPerPx;
                twoFingerScrollCarryY += -(cy - lastCentroidY) * unitsPerPx;
                int sx = (int) twoFingerScrollCarryX;
                int sy = (int) twoFingerScrollCarryY;
                twoFingerScrollCarryX -= sx;
                twoFingerScrollCarryY -= sy;
                if (sx != 0 || sy != 0) {
                    stopScrollFling();
                    sendScrollHiRes(sx, sy);
                }
            }
            lastCentroidX = cx;
            lastCentroidY = cy;
            return;
        }
        lastCentroidX = 0;
        lastCentroidY = 0;
        twoFingerScrollCarryX = 0f;
        twoFingerScrollCarryY = 0f;

        if (mode == MODE_DIRECT) {
            processDirect(action, x, y, viewW, viewH);
        } else {
            processTrackpad(action, x, y, viewW, viewH);
        }
    }

    /** Video letterbox/pillarbox alanını hesaba katarak PC koordinatı üretir. */
    private float[] videoRect(int viewW, int viewH) {
        float videoAspect = (float) pcWidth / Math.max(1, pcHeight);
        float viewAspect = (float) viewW / Math.max(1, viewH);
        float left = 0f, top = 0f, w = viewW, h = viewH;
        if (viewAspect > videoAspect) {
            w = viewH * videoAspect;
            left = (viewW - w) / 2f;
        } else {
            h = viewW / videoAspect;
            top = (viewH - h) / 2f;
        }
        return new float[]{left, top, w, h};
    }

    private float[] mapToPc(float x, float y, int viewW, int viewH) {
        float[] r = videoRect(viewW, viewH);
        float px = (x - r[0]) / Math.max(1f, r[2]) * pcWidth;
        float py = (y - r[1]) / Math.max(1f, r[3]) * pcHeight;
        px = Math.max(0, Math.min(pcWidth - 1, px));
        py = Math.max(0, Math.min(pcHeight - 1, py));
        return new float[]{px, py};
    }

    /**
     * VideoSurfaceView'ı üst/alt HUD arasında, doğru 16:9 oranında ortalar.
     * Her çağrıda güncel ebeveyn/HUD ölçüleriyle yeniden hesaplanır; böylece
     * döndürme ve klavye açılıp kapanması eski (klipsli) yerleşimi bırakmaz.
     */
    private void applyAspectLayout() {
        if (surfaceView == null || !(surfaceView.getParent() instanceof FrameLayout)) {
            return;
        }
        FrameLayout parent = (FrameLayout) surfaceView.getParent();
        int parentW = parent.getWidth();
        int parentH = parent.getHeight();
        int bottomInset = 0;
        if (controlBar != null && controlBar.getVisibility() == View.VISIBLE) {
            // Klavye açılınca dock yukarı büyür; videoyu sıfıra indirmek
            // yerine klavye videonun altını geçici olarak örtsün.
            if (keyboardContainer == null
                    || keyboardContainer.getVisibility() != View.VISIBLE
                    || dockBaseHeight == 0) {
                dockBaseHeight = controlBar.getHeight();
            }
            bottomInset = dockBaseHeight;
        }
        int availW = parentW;
        // Üst HUD artık videoyu aşağı itmez; üst alan da zoom/pan için kullanılır.
        int availH = parentH - bottomInset;
        if (availW <= 0 || availH <= 0) {
            return;
        }
        float videoAspect = (float) Math.max(1, pcWidth) / Math.max(1, pcHeight);
        float fitW = availW;
        float fitH = fitW / videoAspect;
        if (fitH > availH) {
            fitH = availH;
            fitW = fitH * videoAspect;
        }
        float baseLeft = (parentW - fitW) / 2f;
        float baseTop = (availH - fitH) / 2f;

        float scaledW = fitW * zoomScale;
        float scaledH = fitH * zoomScale;
        float maxPanX = Math.max(0f, (scaledW - availW) / 2f);
        float maxPanY = Math.max(0f, (scaledH - availH) / 2f);
        zoomPanX = clamp(zoomPanX, -maxPanX, maxPanX);
        zoomPanY = clamp(zoomPanY, -maxPanY, maxPanY);
        float left = baseLeft + (fitW - scaledW) / 2f + zoomPanX;
        float top = baseTop + (fitH - scaledH) / 2f + zoomPanY;

        FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(
                Math.round(scaledW), Math.round(scaledH));
        lp.gravity = android.view.Gravity.TOP | android.view.Gravity.LEFT;
        lp.leftMargin = Math.round(left);
        lp.topMargin = Math.round(top);
        surfaceView.setLayoutParams(lp);
        reticle.setVideoRect(left, top, left + scaledW, top + scaledH);
    }

    /** Dokunuş gerçekten video görüntüsünün üzerinde mi? */
    private boolean insideVideo(float x, float y, int viewW, int viewH) {
        float[] r = videoRect(viewW, viewH);
        return x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3];
    }

    private void processDirect(int action, float x, float y, int viewW, int viewH) {
        float[] mapped = mapToPc(x, y, viewW, viewH);
        float pcX = mapped[0];
        float pcY = mapped[1];
        // Trackpad'e geçildiğinde göreli sıçramanın doğru hesaplanması için
        // son mutlak imleç konumunu da aklımızda tut.
        virtX = pcX;
        virtY = pcY;
        // Siyah letterbox alanına dokunmak imleci ekranın kenarına
        // fırlatıyordu. Basışı yalnız gerçek görüntü dikdörtgeninde kabul et;
        // sürükleme görüntü dışına taşarsa mevcut basış devam eder.
        if (action == MotionEvent.ACTION_DOWN && !insideVideo(x, y, viewW, viewH)) {
            updateReticleHide();
            return;
        }
        Log.d("PCHomeTouch", "direct action=" + action + " touch=" + x + "," + y
                + " view=" + viewW + "x" + viewH
                + " pc=" + pcWidth + "x" + pcHeight
                + " -> " + pcX + "," + pcY);
        updateReticle(x, y);
        switch (action) {
            case MotionEvent.ACTION_DOWN:
                downX = x;
                downY = y;
                downTime = System.currentTimeMillis();
                moved = false;
                longPressFired = false;
                pressed = true;
                sendMoveAbs((int) pcX, (int) pcY);
                sendClick("left", "down");
                touchHandler.postDelayed(() -> {
                    if (pressed && !moved && !longPressFired) {
                        // Left button was pressed for dragging; release it
                        // before starting the right-button long press.
                        longPressFired = true;
                        sendClick("left", "up");
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
                pressed = false;
                if (longPressFired) {
                    sendClick("right", "up");
                } else {
                    sendClick("left", "up");
                    // İki dokunuş OS'nin çift tıklama aralığındaysa zaten
                    // çift tıklama olur; ayrıca "double" göndermiyoruz.
                    lastTapTime = System.currentTimeMillis();
                }
                // Reticle görünür kalsın: kullanıcı imlecin son konumunu
                // dokunmayı bıraktıktan sonra da görsün.
                break;
            case MotionEvent.ACTION_CANCEL:
                pressed = false;
                longPressFired = false;
                sendClick("left", "up");
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
                pressed = true;
                trackpadLeadScreen = 0f;
                placeTrackpadCursor(x, y, viewW, viewH, true);
                touchHandler.postDelayed(() -> {
                    if (pressed && !moved && !longPressFired) {
                        longPressFired = true;
                        sendClick("right", "down");
                    }
                }, LONG_PRESS_MS);
                break;
            case MotionEvent.ACTION_MOVE:
                float dy = y - lastY;
                if (dy != 0f) {
                    float density = getResources().getDisplayMetrics().density;
                    float maxLead = TRACKPAD_LEAD_MAX_DP * density;
                    trackpadLeadScreen = clamp(
                            trackpadLeadScreen + dy * TRACKPAD_LEAD_RATIO,
                            -maxLead, maxLead) * 0.82f;
                }
                lastX = x;
                lastY = y;
                if (Math.abs(x - downX) > MOVE_THRESHOLD || Math.abs(y - downY) > MOVE_THRESHOLD) {
                    moved = true;
                }
                placeTrackpadCursor(x, y, viewW, viewH, false);
                break;
            case MotionEvent.ACTION_UP:
                pressed = false;
                if (longPressFired) {
                    sendClick("right", "up");
                } else if (!moved) {
                    sendClick("left", "click");
                }
                break;
            case MotionEvent.ACTION_CANCEL:
                pressed = false;
                longPressFired = false;
                break;
            default:
                break;
        }
    }

    private float clamp(float v, float min, float max) {
        return Math.max(min, Math.min(max, v));
    }

    /**
     * Trackpad'de imleci parmağa göre mutlak konumlandırır. Sağ el seçiliyse
     * imleç parmağın solunda, sol el seçiliyse sağında kalır; göreli paket
     * kaybı offset'i bozamaz.
     */
    private void placeTrackpadCursor(float x, float y, int viewW, int viewH,
                                     boolean announceMode) {
        float[] r = videoRect(viewW, viewH);
        float[] mapped = mapToPc(x, y, viewW, viewH);
        float density = getResources().getDisplayMetrics().density;
        float gapPc = (TRACKPAD_GAP_DP * density) / Math.max(1f, r[2]) * pcWidth;
        float side = leftHanded ? 1f : -1f;
        float leadPc = (trackpadLeadScreen / Math.max(1f, r[3])) * pcHeight;
        float targetX = clamp(mapped[0] + side * gapPc, 0, pcWidth - 1);
        float targetY = clamp(mapped[1] + leadPc, 0, pcHeight - 1);
        virtX = targetX;
        virtY = targetY;
        sendMoveAbs(Math.round(targetX), Math.round(targetY));
        if (announceMode) {
            sendMode(); // Kip satırı trackpad kalsın.
        }
        updateReticle(r[0] + virtX / pcWidth * r[2], r[1] + virtY / pcHeight * r[3]);
    }

    private void updateReticle(float x, float y) {
        // x,y SurfaceView'a göre; ReticleView tüm ekranı kapladığı için
        // SurfaceView'ın üst/left kaymasını ekle.
        final float ox = surfaceView.getLeft();
        final float oy = surfaceView.getTop();
        uiHandler.post(() -> reticle.setPosition(ox + x, oy + y));
    }

    private void updateReticleHide() {
        uiHandler.post(() -> reticle.setVisible(false));
    }

    private void sendMode() {
        send(new ControlMessage()
                .type("mode")
                .put("mode", mode == MODE_DIRECT ? "direct" : "trackpad")
                .toJson());
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

    private void sendScrollHiRes(int dx, int dy) {
        send(new ControlMessage()
                .type("scroll_hi")
                .put("dx", dx)
                .put("dy", dy)
                .toJson());
    }

    private void startScrollFling(float unitsPerSecond) {
        if (uiHandler == null) {
            return;
        }
        scrollFlingVelocity = clamp(unitsPerSecond, -8000f, 8000f);
        if (Math.abs(scrollFlingVelocity) < 50f) {
            return;
        }
        scrollFlingLast = SystemClock.uptimeMillis();
        uiHandler.removeCallbacks(scrollFlingRunnable);
        uiHandler.post(scrollFlingRunnable);
    }

    private void stopScrollFling() {
        scrollFlingVelocity = 0f;
        if (uiHandler != null) {
            uiHandler.removeCallbacks(scrollFlingRunnable);
        }
    }

    private final Runnable scrollFlingRunnable = new Runnable() {
        @Override
        public void run() {
            long now = SystemClock.uptimeMillis();
            float dt = (now - scrollFlingLast) / 1000f;
            scrollFlingLast = now;
            if (dt <= 0f) {
                dt = 0.016f;
            }
            dt = Math.min(dt, 0.05f);
            int units = Math.round(scrollFlingVelocity * dt);
            if (units != 0) {
                sendScrollHiRes(0, units);
            }
            scrollFlingVelocity *= (float) Math.pow(0.12f, dt);
            if (Math.abs(scrollFlingVelocity) > 40f) {
                uiHandler.postDelayed(this, 16L);
            }
        }
    };

    private void sendKey(int code, boolean down) {
        send(new ControlMessage().type("key").put("code", code).put("action", down ? "down" : "up").toJson());
    }

    private void sendCombo(int code, String mod) {
        send(new ControlMessage()
                .type("combo")
                .put("code", code)
                .put("mod", mod)
                .toJson());
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
            if (connDot != null) {
                int color = state == WebRtcClient.ConnectionState.CONNECTED
                        ? 0xFF34D399
                        : state == WebRtcClient.ConnectionState.CONNECTING
                                ? 0xFFFBBF24 : 0xFFFB7185;
                ViewCompat.setBackgroundTintList(connDot,
                        ColorStateList.valueOf(color));
            }
            if (state == WebRtcClient.ConnectionState.CONNECTED) {
                markConnected();
                // Kanal açılır açılmaz seçili kipi masaüstüne bildir.
                sendMode();
                // İlk kare gelene kadar durumu görünür tut; siyah ekranda
                // kullanıcı ne olduğunu anlasın. İlk kareden sonra çipi
                // yeniden gösterme (aksi hâlde "ilk kare" yazısı ekranda kalıyor).
                if (!firstFrameRendered) {
                    streamStatus.setText(R.string.conn_phase_video);
                    streamStatus.setVisibility(View.VISIBLE);
                } else {
                    // Kısa bir kopma sırasında gösterilen hata metni,
                    // bağlantı geri gelince ekranda kalmasın.
                    streamStatus.setVisibility(View.GONE);
                }
            } else if (state == WebRtcClient.ConnectionState.DISCONNECTED) {
                saveRecent();
                if (reticle != null) reticle.setVisible(false);
                if (emptyGrid != null) emptyGrid.setVisibility(View.VISIBLE);
                streamStatus.setText(R.string.disconnected);
                streamStatus.setVisibility(View.VISIBLE);
            } else if (state == WebRtcClient.ConnectionState.FAILED) {
                saveRecent();
                if (reticle != null) reticle.setVisible(false);
                if (emptyGrid != null) emptyGrid.setVisibility(View.VISIBLE);
                streamStatus.setText(R.string.error);
                streamStatus.setVisibility(View.VISIBLE);
            }
        });
    }

    private void markConnected() {
        connectedOnce = true;
        Diag.phase(this, "bağlandı");
        if (uiHandler != null) {
            uiHandler.removeCallbacks(timeoutCheck);
        }
    }

    @Override
    public void onRemoteTrack(org.webrtc.MediaStream stream) {
        runOnUiThread(this::markConnected);
        // Durum çipi ilk gerçek karede (RendererEvents) gizlenir.
    }

    @Override
    public void onDataChannelMessage(org.webrtc.DataChannel.Buffer buffer) {
        try {
            byte[] data = new byte[buffer.data.remaining()];
            buffer.data.get(data);
            String json = new String(data, java.nio.charset.StandardCharsets.UTF_8);
            JSONObject msg = new JSONObject(json);
            if ("display_info".equals(msg.getString("type"))) {
                pcWidth = Math.max(1, msg.optInt("width", pcWidth));
                pcHeight = Math.max(1, msg.optInt("height", pcHeight));
                Log.i("PCHomeTouch", "display_info " + pcWidth + "x" + pcHeight);
                runOnUiThread(this::applyAspectLayout);
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
            // Ölümcül hatalarda sessizce başa atma: sebebi göster, seçim sun.
            if (error.contains("reddedildi") || error.contains("hatalı")) {
                saveRecent();
                showFatal("Bağlantı reddedildi", error);
            } else {
                Toast.makeText(DisplayActivity.this,
                        getString(R.string.error) + ": " + error, Toast.LENGTH_LONG).show();
            }
        });
    }

    /**
     * Ölümcül bağlantı hatası diyaloğu: ekranda kalır, [Tekrar Dene]
     * aynı bilgilerle yeniden bağlanır, [Geri Dön] kartı kaydedip çıkar.
     */
    private void showFatal(String title, String detail) {
        if (isFinishing() || fatalShown) {
            return;
        }
        fatalShown = true;
        Diag.phase(this, "ölümcül-hata: " + title);
        if (uiHandler != null) {
            uiHandler.removeCallbacks(timeoutCheck);
        }
        fatalDialog = new AlertDialog.Builder(this)
                .setTitle(title)
                .setMessage(detail)
                .setCancelable(false)
                .setPositiveButton(R.string.retry, (d, w) -> {
                    fatalShown = false;
                    retryConnection();
                })
                .setNegativeButton(R.string.go_back, (d, w) -> {
                    saveRecent();
                    finish();
                })
                .create();
        fatalDialog.show();
    }

    private void retryConnection() {
        recentSaved = false;
        connectedOnce = false;
        fatalShown = false;
        streamStatus.setText(R.string.conn_phase_signal);
        streamStatus.setVisibility(View.VISIBLE);
        if (emptyGrid != null) emptyGrid.setVisibility(View.VISIBLE);
        if (webRtcClient != null) {
            webRtcClient.disconnect();
            webRtcClient.connect(signalUrl, pcId, password);
        }
        if (uiHandler != null) {
            uiHandler.removeCallbacks(timeoutCheck);
            uiHandler.postDelayed(timeoutCheck, CONNECT_TIMEOUT_MS);
        }
    }

    @Override
    public void onDeviceInfo(org.json.JSONObject info) {
        deviceName = info.optString("pc_name", deviceName);
        deviceIp = info.optString("ip", deviceIp);
        deviceMac = info.optString("mac", deviceMac);
        deviceRes = info.optString("resolution", deviceRes);
        persistDeviceFields();
        runOnUiThread(() -> {
            refreshInfoChip();
            if (!connectedOnce) {
                streamStatus.setText(R.string.conn_phase_verified);
                streamStatus.setVisibility(View.VISIBLE);
            }
        });
    }

    @Override
    public void onDisplayInfo(int width, int height) {
        pcWidth = width;
        pcHeight = height;
        deviceRes = width + "x" + height;
        persistDeviceFields();
        runOnUiThread(() -> {
            refreshInfoChip();
            applyAspectLayout();
        });
    }

    /** Üst bilgi çipi: isim • çözünürlük • kip. */
    private void refreshInfoChip() {
        String name = deviceName.isEmpty() ? ("PC " + pcId) : deviceName;
        infoTitle.setText(name);
        String res = deviceRes.isEmpty() ? "Hazırlanıyor…" : deviceRes;
        String kip = getString(mode == MODE_DIRECT
                ? R.string.mode_direct : R.string.mode_trackpad);
        infoChip.setText(res + " • " + kip);
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
            if (surfaceView.getHolder() == null
                    || w <= 0 || h <= 0 || surfaceView.getHolder().getSurface() == null
                    || !surfaceView.getHolder().getSurface().isValid()) {
                return;
            }
            // Bellek dostu: PixelCopy doğrudan karta sığan boyuta ölçekler
            // (4K tam boy bitmap OOM patlatıyordu).
            int bw = Math.min(w, 480);
            int bh = Math.max(1, (int) ((long) h * bw / Math.max(1, w)));
            Bitmap thumb = Bitmap.createBitmap(bw, bh, Bitmap.Config.ARGB_8888);
            PixelCopy.request(surfaceView.getHolder().getSurface(), thumb,
                    copyResult -> {
                        try {
                            if (copyResult == PixelCopy.SUCCESS) {
                                java.io.File dest = recentStore.thumbnailFile(pcId);
                                java.io.FileOutputStream out =
                                        new java.io.FileOutputStream(dest);
                                thumb.compress(Bitmap.CompressFormat.JPEG, 80, out);
                                out.close();
                                RecentPc entry = recentStore.find(pcId);
                                if (entry == null) {
                                    entry = new RecentPc(pcId, deviceName);
                                }
                                entry.thumbnail = dest.getAbsolutePath();
                                entry.lastSeen = System.currentTimeMillis();
                                recentStore.upsert(entry);
                            }
                        } catch (Exception e) {
                            // Görüntü kaydedilemezse kart bilgisiz kalır.
                        } finally {
                            try {
                                thumb.recycle();
                            } catch (Exception e) {
                                // Yoksay.
                            }
                        }
                    }, uiHandler);
        } catch (Exception e) {
            // PixelCopy yoksa sessiz geç (kart yine de durur).
        } catch (OutOfMemoryError e) {
            // Düşük bellekte küçük resimden vazgeç, bağlantı etkilenmez.
        }
    }

    @Override
    protected void onDestroy() {
        releaseModifiers();
        stopScrollFling();
        if (uiHandler != null) {
            uiHandler.removeCallbacks(timeoutCheck);
        }
        if (fatalDialog != null && fatalDialog.isShowing()) {
            fatalDialog.dismiss();
        }
        saveRecent();
        if (webRtcClient != null) {
            webRtcClient.disconnect();
        }
        if (touchThread != null) {
            touchThread.quitSafely();
        }
        if (surfaceView != null && surfaceInitialized) {
            surfaceView.release();
            surfaceInitialized = false;
        }
        if (webRtcClient != null) {
            webRtcClient.releaseGlobals();
        }
        super.onDestroy();
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
