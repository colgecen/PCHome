package com.PCHome.mobile;

import android.content.res.ColorStateList;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.graphics.Rect;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Process;
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

import androidx.annotation.RequiresApi;
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

import java.util.Collections;
import java.util.HashSet;
import java.util.Set;
import java.util.concurrent.atomic.AtomicLong;

public class DisplayActivity extends AppCompatActivity implements WebRtcClient.WebRtcListener {

    private static final int MODE_DIRECT = 0;
    private static final int MODE_TRACKPAD = 1;
    /** İki parmak ve sağ şerit için: 1 dp ≈ 20 yüksek çözünürlüklü birim. */
    private static final float SCROLL_UNITS_PER_DP = 20f;
    /** Trackpad'de imleç ile parmak arasındaki görsel boşluk (dp). */
    private static final float TRACKPAD_GAP_DP = 130f;
    /**
     * Dikeyde merkez sabit; uçlara gidildikçe imleç parmaktan bu oran
     * kadar daha ileri gider. 1.0 = parmakla birebir.
     */
    private static final float TRACKPAD_Y_GAIN = 1.25f;
    /** Kenarlara yaklaşırken yatay boşluğun sıfıra indiği oran. */
    private static final float TRACKPAD_EDGE_TAPER = 0.25f;
    private static final float MOVE_THRESHOLD = 12f;
    private static final long LONG_PRESS_MS = 500;
    private static final long DOUBLE_TAP_MS = 300;
    private static final long CONNECT_TIMEOUT_MS = 25000;
    /** İmleç paketleri en fazla bu sıklıkta gönderilir; ara konumlar atlanır. */
    private static final long POINTER_SEND_INTERVAL_MS = 8L;
    /** Tam ekran yan tık butonları: ekran genişliğinin %8'i. */
    private static final float SIDE_CLICK_WIDTH_RATIO = 0.08f;
    /** Dikeyde %8 çok ince kalıyor; en az 48dp genişlik kullan. */
    private static final float SIDE_CLICK_MIN_WIDTH_DP = 48f;
    /** Portrede butonlar PC görüntüsünün altında, yatayda ortalanmış. */
    private static final float SIDE_CLICK_PORTRAIT_WIDTH_RATIO = 0.34f;
    private static final float SIDE_CLICK_PORTRAIT_MAX_WIDTH_DP = 190f;
    private static final float SIDE_CLICK_PORTRAIT_TOP_GAP_DP = 12f;
    private static final float SIDE_CLICK_PORTRAIT_BOTTOM_MARGIN_DP = 10f;
    private static final float SIDE_CLICK_PORTRAIT_MAX_HEIGHT_DP = 300f;
    /** İki butonun toplam yüksekliği (yatayda) ekran yüksekliğinin yarısı. */
    private static final float SIDE_CLICK_HEIGHT_RATIO = 0.5f;
    private static final float SIDE_CLICK_GAP_DP = 6f;
    private static final float SIDE_CLICK_MARGIN_DP = 4f;

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
    private MaterialButton btnZoom;
    private TextView zoomHint;
    private LinearLayout sideClicks;
    private View sideBtnLeftClick;
    private View sideBtnRightClick;
    private ImageButton btnEyeFloat;
    private boolean menuHidden;
    private boolean zoomMode;
    private float zoomScale = 1f;
    private float zoomPanX;
    private float zoomPanY;
    private float zoomLastTouchX;
    private float zoomLastTouchY;
    private float zoomFocusLastX;
    private float zoomFocusLastY;
    private boolean zoomFocusValid;
    private ScaleGestureDetector scaleDetector;
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
                    getString(R.string.timeout_title),
                    getString(R.string.timeout_body));
        }
    };

    private int pcWidth = 1920;
    private int pcHeight = 1080;
    /** Klavye açıkken control_bar büyür; video alanı bu taban yüksekliği kullanır. */
    private int dockBaseHeight = 0;
    private int mode = MODE_TRACKPAD;
    private boolean leftHanded;
    private float virtX = pcWidth / 2f;
    private float virtY = pcHeight / 2f;

    private HandlerThread touchThread;
    private Handler touchHandler;
    private Handler uiHandler;
    private boolean surfaceInitialized;
    private volatile int surfaceLeft;
    private volatile int surfaceTop;

    // İmleç paketlerini birleştirme durumu.
    private long lastPointerSentAt;
    private int pendingPointerX;
    private int pendingPointerY;
    private boolean hasPendingPointer;
    /**
     * İmleç ve tık paketlerine ortak sıra numarası. İki ayrı veri kanalı
     * kullanıldığı için eski bir imleç paketi tıktan sonra varırsa masaüstü
     * onu sıra numarasından atar.
     */
    private final AtomicLong pointerSeq = new AtomicLong();
    private boolean pointerFlushScheduled;
    private final Runnable pointerFlushRunnable = this::flushPendingPointer;
    private final Runnable hideSystemBarsRunnable = this::hideSystemBars;

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
    /** Klavye kapanırken/bağlantı koparken takılı kalmasın diye izlenir. */
    private final Set<Integer> heldKeys = Collections.synchronizedSet(new HashSet<>());
    private final Set<String> heldMouseButtons =
            Collections.synchronizedSet(new HashSet<>());

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
        zoomHint = findViewById(R.id.zoom_hint);
        sideClicks = findViewById(R.id.side_clicks);
        sideBtnLeftClick = findViewById(R.id.side_btn_left_click);
        sideBtnRightClick = findViewById(R.id.side_btn_right_click);
        // Butonlar arasındaki boşluğa dokunmak uzak imleci oynatmasın.
        sideClicks.setClickable(true);
        sideClicks.setOnTouchListener((v, event) -> true);
        // Dock ve klavye alanı yalnızca arayüz içindir; buradaki boşluklara
        // dokunmak video/trackpad katmanına geçip imleci oynatmasın.
        controlBar.setClickable(true);
        controlBar.setOnTouchListener((v, event) -> true);
        keyboardContainer.setClickable(true);
        keyboardContainer.setOnTouchListener((v, event) -> true);
        btnEyeFloat = findViewById(R.id.btn_eye_float);

        // El tercihi (tam ekran yan butonları ve yüzen göz için) layout'tan
        // önce bilinmeli.
        leftHanded = getIntent().getBooleanExtra("leftHanded", false);

        applyHudInsets();

        // Video alanı; üst/alt HUD yükseklikleri değişince (döndürme,
        // klavye, inset) yeniden hesaplanmalı.
        View parent = (View) surfaceView.getParent();
        parent.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> {
            applyAspectLayout();
            updateGestureExclusions();
        });
        topOverlay.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> applyAspectLayout());
        controlBar.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> {
            applyAspectLayout();
            updateGestureExclusions();
        });
        keyboardContainer.addOnLayoutChangeListener(
                (v, l, t, r, b, ol, ot, or, ob) -> updateGestureExclusions());
        sideClicks.addOnLayoutChangeListener(
                (v, l, t, r, b, ol, ot, or, ob) -> updateGestureExclusions());
        surfaceView.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> {
            updateReticleRect();
            updateGestureExclusions();
        });


        uiHandler = new Handler(getMainLooper());
        // Dokunma/klavye paketleri UI çiziminin arkasında kalmasın: kısa
        // süren işler için yüksek öncelikli, ayrı bir iş parçacığı.
        touchThread = new HandlerThread("touch-controller",
                Process.THREAD_PRIORITY_URGENT_DISPLAY);
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
            showFatal(getString(R.string.start_failed),
                    getString(R.string.media_engine_failed,
                            String.valueOf(e.getMessage())));
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
        hideSystemBars();
        if (surfaceView != null) {
            // Yeni ölçüler oturduktan SONRA hesapla; aksi hâlde eski
            // portre boyutlarıyla hesaplanan video şeridi ekranda kalıyordu.
            surfaceView.post(this::applyAspectLayout);
            surfaceView.postDelayed(this::applyAspectLayout, 120L);
            surfaceView.postDelayed(this::updateGestureExclusions, 160L);
        }
    }

    /** Tam ekran, immersive; durum/navigasyon çubukları kaydırmayla gelir. */
    private void enterImmersive() {
        WindowCompat.setDecorFitsSystemWindows(getWindow(), false);
        getWindow().setStatusBarColor(Color.TRANSPARENT);
        getWindow().setNavigationBarColor(Color.TRANSPARENT);
        hideSystemBars();
    }

    /** Durum çubuğu, 3 tuşlu navigasyon ve gesture çubuğunu gizle. */
    private void hideSystemBars() {
        if (getWindow() == null) {
            return;
        }
        WindowInsetsControllerCompat controller =
                WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        controller.hide(WindowInsetsCompat.Type.systemBars());
        controller.setSystemBarsBehavior(
                WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            // Eski sürümlerde IMMERSIVE_STICKY, çubukların dokunuşla
            // kalıcı olarak geri gelmesini engeller.
            getWindow().getDecorView().setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                            | View.SYSTEM_UI_FLAG_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION);
        }
    }

    /** Ekran çubuklarını kısa bir gecikmeyle tekrar gizle (tuş/tık sonrası). */
    private void hideSystemBarsSoon() {
        if (uiHandler == null) {
            return;
        }
        uiHandler.removeCallbacks(hideSystemBarsRunnable);
        uiHandler.postDelayed(hideSystemBarsRunnable, 350L);
    }

    @Override
    public boolean dispatchTouchEvent(MotionEvent ev) {
        int action = ev.getActionMasked();
        if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL) {
            hideSystemBarsSoon();
        }
        return super.dispatchTouchEvent(ev);
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            hideSystemBars();
        } else {
            // Sistem diyaloğu/başka pencere açılırsa tuş basılı kalmasın.
            releaseAllHeldInput();
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        hideSystemBars();
    }

    @Override
    protected void onPause() {
        // Arka plana geçerken PC'de basılı tuş/fare butonu kalmasın.
        releaseAllHeldInput();
        super.onPause();
    }

    /** HUD'u çentik/durum çubuğu güvenli alanına yerleştir. */
    private void applyHudInsets() {
        View root = findViewById(android.R.id.content);
        ViewCompat.setOnApplyWindowInsetsListener(root, (v, insets) -> {
            Insets cut = insets.getInsets(WindowInsetsCompat.Type.displayCutout());
            Insets bars = insets.getInsets(WindowInsetsCompat.Type.systemBars());
            topOverlay.setPadding(16 + cut.left, 12 + bars.top + cut.top,
                    16 + cut.right, 16);
            // Alt menünün altında boşluk bırakma. Navigasyon/gesture çubuğu
            // immersive modda gizli; tuşların kenara yakınlığını gesture
            // hariç tutma alanları ve tekrar gizleme hallediyor.
            controlBar.setPadding(10 + cut.left, 9, 10 + cut.right, 9);
            if (btnEyeFloat != null) {
                FrameLayout.LayoutParams lp =
                        (FrameLayout.LayoutParams) btnEyeFloat.getLayoutParams();
                lp.gravity = leftHanded
                        ? (android.view.Gravity.TOP | android.view.Gravity.LEFT)
                        : (android.view.Gravity.TOP | android.view.Gravity.RIGHT);
                lp.topMargin = 12 + bars.top + cut.top;
                lp.leftMargin = 10 + cut.left;
                lp.rightMargin = 10 + cut.right;
                btnEyeFloat.setLayoutParams(lp);
            }
            // Çubuklar geçici olarak görünür olduysa kısa süre sonra tekrar gizle.
            boolean barsVisible = insets.isVisible(WindowInsetsCompat.Type.statusBars())
                    || insets.isVisible(WindowInsetsCompat.Type.navigationBars());
            if (barsVisible && (fatalDialog == null || !fatalDialog.isShowing())) {
                v.postDelayed(this::hideSystemBars, 900L);
            }
            v.post(this::applyAspectLayout);
            v.post(this::updateGestureExclusions);
            return insets;
        });
    }

    /** Sistem gesture alanlarını klavye/yan tık butonları için serbest bırak. */
    private void updateGestureExclusions() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) {
            return;
        }
        excludeGestureArea(controlBar, true);
        excludeGestureArea(keyboardContainer, false);
        excludeGestureArea(sideClicks, false);
    }

    @RequiresApi(Build.VERSION_CODES.Q)
    private void excludeGestureArea(View view, boolean bottomOnly) {
        if (view == null) {
            return;
        }
        if (view.getVisibility() != View.VISIBLE
                || view.getWidth() <= 0 || view.getHeight() <= 0) {
            view.setSystemGestureExclusionRects(Collections.emptyList());
            return;
        }
        int top = 0;
        if (bottomOnly) {
            // Android tek kenarda toplam ~200dp'lik hariç tutmaya izin verir.
            int maxHeight = Math.round(190f * getResources().getDisplayMetrics().density);
            top = Math.max(0, view.getHeight() - maxHeight);
        }
        view.setSystemGestureExclusionRects(Collections.singletonList(
                new Rect(0, top, view.getWidth(), view.getHeight())));
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
        View btnRotate = findViewById(R.id.btn_rotate);

        // Varsayılan kip trackpad.
        btnMode.setText(mode == MODE_DIRECT ? R.string.mode_direct : R.string.mode_trackpad);
        if (btnMode instanceof MaterialButton) {
            ((MaterialButton) btnMode).setIconResource(
                    mode == MODE_DIRECT ? R.drawable.ic_touch : R.drawable.ic_mouse);
        }
        refreshInfoChip();

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
        if (btnEyeFloat != null) {
            btnEyeFloat.setOnClickListener(v -> {
                toggleMenu();
                tick();
            });
        }
        setupSideClickButton(sideBtnLeftClick, "left");
        setupSideClickButton(sideBtnRightClick, "right");
        btnZoom.setOnClickListener(v -> {
            toggleZoomMode();
            tick();
        });
        btnZoom.setOnLongClickListener(v -> {
            resetZoom();
            return true;
        });
        scaleDetector = new ScaleGestureDetector(this,
                new ScaleGestureDetector.SimpleOnScaleGestureListener() {
                    @Override
                    public boolean onScale(ScaleGestureDetector detector) {
                        if (!zoomMode) {
                            return false;
                        }
                        zoomScale = clamp(zoomScale * detector.getScaleFactor(), 0.5f, 3f);
                        applyAspectLayout();
                        return true;
                    }
                });
        scaleDetector.setQuickScaleEnabled(false);
        scaleDetector.setStylusScaleEnabled(false);
        btnRotate.setOnClickListener(v -> rotateScreen());

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

    /** Tam ekranda yan tık: basılı tutunca mouse tuşu basılı kalır. */
    private void setupSideClickButton(View button, String mouseButton) {
        if (button == null) {
            return;
        }
        button.setOnTouchListener((v, event) -> {
            switch (event.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    v.setAlpha(0.68f);
                    sendClick(mouseButton, "down");
                    tick();
                    hideSystemBarsSoon();
                    return true;
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    v.setAlpha(1f);
                    sendClick(mouseButton, "up");
                    hideSystemBarsSoon();
                    return true;
                default:
                    return true;
            }
        });
    }

    /** Alt menüyü göster/gizle; video alanı boşalan yeri doldurur. */
    private void toggleMenu() {
        menuHidden = !menuHidden;
        if (menuHidden) {
            // Tam ekranda PC'nin tamamı görünsün: zoom/pan sıfırlanır.
            clearZoomForFullscreen();
            if (keyboardContainer.getVisibility() == View.VISIBLE) {
                releaseAllHeldInput();
                keyboardContainer.setVisibility(View.GONE);
                btnKeyboard.setText(R.string.keyboard);
            }
            if (scrollRail.getVisibility() == View.VISIBLE) {
                scrollRail.setVisibility(View.GONE);
                setWheelActive(false);
            }
            if (zoomHint != null) {
                zoomHint.setVisibility(View.GONE);
            }
        }
        controlBar.setVisibility(menuHidden ? View.GONE : View.VISIBLE);
        topOverlay.setVisibility(menuHidden ? View.GONE : View.VISIBLE);
        if (!menuHidden) {
            topOverlay.setAlpha(1f);
        }
        btnEyeFloat.setVisibility(menuHidden ? View.VISIBLE : View.GONE);
        sideClicks.setVisibility(menuHidden ? View.VISIBLE : View.GONE);
        btnHideMenu.setImageResource(menuHidden ? R.drawable.ic_eye : R.drawable.ic_eye_off);
        btnHideMenu.setContentDescription(getString(
                menuHidden ? R.string.show_menu : R.string.hide_menu));
        if (scrollRail != null) {
            FrameLayout.LayoutParams lp = (FrameLayout.LayoutParams) scrollRail.getLayoutParams();
            lp.bottomMargin = Math.round((menuHidden ? 24f : 130f)
                    * getResources().getDisplayMetrics().density);
            scrollRail.setLayoutParams(lp);
        }
        hideSystemBars();
        applyAspectLayout();
        surfaceView.postDelayed(this::applyAspectLayout, 120L);
        surfaceView.postDelayed(this::updateGestureExclusions, 160L);
    }

    /** Tam ekrana girerken zoom durumunu gösterişsizce sıfırla. */
    private void clearZoomForFullscreen() {
        zoomMode = false;
        zoomScale = 1f;
        zoomPanX = 0f;
        zoomPanY = 0f;
        zoomFocusValid = false;
        if (btnZoom != null) {
            btnZoom.setText(getString(R.string.zoom));
            btnZoom.setIconTint(ColorStateList.valueOf(0xFFA1A1AA));
            btnZoom.setTextColor(ColorStateList.valueOf(0xFFA1A1AA));
        }
        if (zoomHint != null) {
            zoomHint.setVisibility(View.GONE);
        }
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
        btnZoom.setText(zoomMode ? getString(R.string.zoom_close)
                : getString(R.string.zoom));
        btnZoom.setIconTint(ColorStateList.valueOf(
                zoomMode ? 0xFFF4F4F5 : 0xFFA1A1AA));
        btnZoom.setTextColor(zoomMode ? 0xFFF4F4F5 : 0xFFA1A1AA);
        if (zoomMode) {
            hideTopInfoNow();
            if (zoomHint != null) {
                zoomHint.setVisibility(View.VISIBLE);
            }
            if (scrollRail.getVisibility() == View.VISIBLE) {
                scrollRail.setVisibility(View.GONE);
                setWheelActive(false);
            }
            zoomFocusValid = false;
        } else {
            if (zoomHint != null) {
                zoomHint.setVisibility(View.GONE);
            }
            showTopInfoNow();
        }
        applyAspectLayout();
    }

    private void resetZoom() {
        zoomScale = 1f;
        zoomPanX = 0f;
        zoomPanY = 0f;
        zoomMode = false;
        btnZoom.setText(getString(R.string.zoom));
        btnZoom.setIconTint(ColorStateList.valueOf(0xFFA1A1AA));
        btnZoom.setTextColor(ColorStateList.valueOf(0xFFA1A1AA));
        if (zoomHint != null) {
            zoomHint.setVisibility(View.GONE);
        }
        showTopInfoNow();
        applyAspectLayout();
        Toast.makeText(this, R.string.zoom_reset, Toast.LENGTH_SHORT).show();
    }

    private void hideTopInfoNow() {
        if (topOverlay != null) {
            topOverlay.animate().cancel();
            topOverlay.setVisibility(View.INVISIBLE);
        }
    }

    private void showTopInfoNow() {
        if (topOverlay != null) {
            topOverlay.setVisibility(View.VISIBLE);
            topOverlay.animate().alpha(1f).setDuration(180L).start();
        }
    }

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
        // keycode'larını güvenilir/sıralı input DataChannel üzerinden gönderir.
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

            @Override
            public void onReleaseAll() {
                releaseAllHeldInput();
            }
        });
        btnKeyboard.setOnClickListener(v -> {
            boolean show = keyboardContainer.getVisibility() != View.VISIBLE;
            if (!show) {
                releaseAllHeldInput();
            }
            keyboardContainer.setVisibility(show ? View.VISIBLE : View.GONE);
            btnKeyboard.setText(show ? getString(R.string.keyboard_hide)
                    : getString(R.string.keyboard));
            if (show && scrollRail != null && scrollRail.getVisibility() == View.VISIBLE) {
                scrollRail.setVisibility(View.GONE);
                setWheelActive(false);
            }
            hideSystemBars();
            applyAspectLayout();
            surfaceView.postDelayed(this::applyAspectLayout, 120L);
            surfaceView.postDelayed(this::updateGestureExclusions, 160L);
        });
    }

    /** Ekran klavyesinden gelen tuş: değiştiriciler yapışkan, diğerleri anlık. */
    private void onSoftKey(int code, boolean down, boolean withShift) {
        if (code < 0) {
            return;
        }
        hideSystemBarsSoon();
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

    /**
     * Klavye kapanırken, sayfa değişirken, uygulama arka plana düşerken veya
     * bağlantı koparken tuşların PC'de basılı kalmasını engeller.
     */
    private void releaseAllHeldInput() {
        Integer[] keys;
        synchronized (heldKeys) {
            keys = heldKeys.toArray(new Integer[0]);
        }
        for (Integer code : keys) {
            sendKey(code, false);
        }
        String[] buttons;
        synchronized (heldMouseButtons) {
            buttons = heldMouseButtons.toArray(new String[0]);
        }
        for (String button : buttons) {
            sendClick(button, "up");
        }
        heldKeys.clear();
        heldMouseButtons.clear();
        shiftSticky = ctrlSticky = altSticky = false;
        if (softKeyboard != null) {
            softKeyboard.setModifierActive(KeyCodeMap.KEY_LEFTSHIFT, false);
            softKeyboard.setModifierActive(KeyCodeMap.KEY_LEFTCTRL, false);
            softKeyboard.setModifierActive(KeyCodeMap.KEY_LEFTALT, false);
        }
    }

    private void setupTouch() {
        // Tam ekran reticle dokunuşu alır; böylece parmak video/letterbox
        // kenarına çıksa da beyaz ok ekran kenarına kadar gider.
        reticle.setOnTouchListener((v, e) -> {
            // Alt HUD/klavye alanı yalnızca arayüz: buradaki dokunuş hiçbir
            // zaman trackpad hareketine dönüşmesin.
            if (controlBar != null && controlBar.getVisibility() == View.VISIBLE
                    && controlBar.getHeight() > 0 && e.getY() >= controlBar.getTop()) {
                return true;
            }
            if (zoomMode) {
                handleZoomTouch(e);
                return true;
            }
            MotionEvent copy = MotionEvent.obtain(e);
            copy.offsetLocation(-surfaceLeft, -surfaceTop);
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
        scaleDetector.onTouchEvent(event);

        int count = event.getPointerCount();
        if (count >= 2) {
            float fx = (event.getX(0) + event.getX(1)) / 2f;
            float fy = (event.getY(0) + event.getY(1)) / 2f;
            if (!zoomFocusValid
                    || action == MotionEvent.ACTION_POINTER_DOWN
                    || action == MotionEvent.ACTION_DOWN) {
                zoomFocusLastX = fx;
                zoomFocusLastY = fy;
                zoomFocusValid = true;
            } else if (action == MotionEvent.ACTION_MOVE) {
                zoomPanX += fx - zoomFocusLastX;
                zoomPanY += fy - zoomFocusLastY;
                zoomFocusLastX = fx;
                zoomFocusLastY = fy;
                applyAspectLayout();
            }
            return;
        }

        zoomFocusValid = false;
        float x = event.getX();
        float y = event.getY();
        switch (action) {
            case MotionEvent.ACTION_DOWN:
                zoomLastTouchX = x;
                zoomLastTouchY = y;
                break;
            case MotionEvent.ACTION_POINTER_UP:
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
                break;
            default:
                break;
        }
    }

    /** Yatay ↔ dikey geçişi; 180° ters çevirme yapmaz. */
    private void rotateScreen() {
        int current = getRequestedOrientation();
        boolean portrait = current == android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
                || current == android.content.pm.ActivityInfo.SCREEN_ORIENTATION_REVERSE_PORTRAIT
                || current == android.content.pm.ActivityInfo.SCREEN_ORIENTATION_SENSOR_PORTRAIT
                || getResources().getConfiguration().orientation
                        == android.content.res.Configuration.ORIENTATION_PORTRAIT;
        setRequestedOrientation(portrait
                ? android.content.pm.ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
                : android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);
        hideSystemBarsSoon();
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
        if (BuildConfig.DEBUG) {
            Log.d("PCHomeTouch", "touch action=" + action + " at " + x + "," + y
                    + " view=" + viewW + "x" + viewH);
        }

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
     *
     * Tam ekranda yan tık butonları PC görüntüsünün üstüne binmesin diye
     * gerektiğinde video, yanda buton genişliği kadar bir siyah kanal
     * bırakacak şekilde daraltılır.
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
        int availH = parentH - bottomInset;
        if (parentW <= 0 || availH <= 0) {
            return;
        }
        float videoAspect = (float) Math.max(1, pcWidth) / Math.max(1, pcHeight);
        boolean fullscreen = menuHidden;
        boolean portrait = parentH > parentW;
        int sideW = 0;
        if (fullscreen && !portrait) {
            // Yatayda butonlar PC görüntüsünün yanında; portrede videoyu
            // daraltmaya gerek yok çünkü butonlar videonun altına gidiyor.
            sideW = Math.max(1, Math.round(parentW * SIDE_CLICK_WIDTH_RATIO));
            sideW = Math.max(sideW, Math.round(SIDE_CLICK_MIN_WIDTH_DP
                    * getResources().getDisplayMetrics().density));
        }
        int sideMargin = Math.round(SIDE_CLICK_MARGIN_DP
                * getResources().getDisplayMetrics().density);
        int sideNeed = sideW > 0 ? sideW + sideMargin * 2 : 0;

        int fitAreaLeft = 0;
        int fitAvailW = parentW;
        float[] fit = fitVideoRect(fitAvailW, availH, videoAspect);
        float fitW = fit[0];
        float fitH = fit[1];
        float baseLeft = (fitAvailW - fitW) / 2f;
        float baseTop = (availH - fitH) / 2f;

        if (fullscreen && sideW > 0) {
            if (leftHanded) {
                // Butonlar sağda: video sağ kenara değmesin.
                if (baseLeft + fitW > parentW - sideNeed) {
                    fitAvailW = Math.max(1, parentW - sideNeed);
                    fit = fitVideoRect(fitAvailW, availH, videoAspect);
                    fitW = fit[0];
                    fitH = fit[1];
                    baseLeft = (fitAvailW - fitW) / 2f;
                    baseTop = (availH - fitH) / 2f;
                }
            } else {
                // Butonlar solda: video sol kenara değmesin.
                if (baseLeft < sideNeed) {
                    fitAvailW = Math.max(1, parentW - sideNeed);
                    fitAreaLeft = sideNeed;
                    fit = fitVideoRect(fitAvailW, availH, videoAspect);
                    fitW = fit[0];
                    fitH = fit[1];
                    baseLeft = fitAreaLeft + (fitAvailW - fitW) / 2f;
                    baseTop = (availH - fitH) / 2f;
                }
            }
        }

        float scaledW = fitW * zoomScale;
        float scaledH = fitH * zoomScale;
        float maxPanX = Math.max(0f, (scaledW - fitAvailW) / 2f);
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
        surfaceLeft = Math.round(left);
        surfaceTop = Math.round(top);
        reticle.setVideoRect(left, top, left + scaledW, top + scaledH);
        layoutSideClicks(parentW, parentH, sideW, left, top, scaledW, scaledH, fullscreen);
    }

    private float[] fitVideoRect(int availW, int availH, float videoAspect) {
        float fitW = availW;
        float fitH = fitW / videoAspect;
        if (fitH > availH) {
            fitH = availH;
            fitW = fitH * videoAspect;
        }
        return new float[]{fitW, fitH};
    }

    /**
     * Yan tık butonlarını yerleştirir.
     * - Yatayda: PC görüntüsünün yanında, toplam yükseklik ekranın yarısı.
     * - Portrede: PC görüntüsünün hemen altında, ortalanmış ve alt alta;
     *   genişlik ekranın ~%34'ü, aradaki boşluk küçük.
     */
    private void layoutSideClicks(int parentW, int parentH, int sideW,
                                  float videoLeft, float videoTop,
                                  float videoW, float videoH, boolean fullscreen) {
        if (sideClicks == null) {
            return;
        }
        if (!fullscreen) {
            sideClicks.setVisibility(View.GONE);
            return;
        }
        float density = getResources().getDisplayMetrics().density;
        int margin = Math.round(SIDE_CLICK_MARGIN_DP * density);
        int gap = Math.round(SIDE_CLICK_GAP_DP * density);
        boolean portrait = parentH > parentW;

        int pairW;
        int pairH;
        int x;
        int y;
        if (portrait) {
            pairW = Math.round(parentW * SIDE_CLICK_PORTRAIT_WIDTH_RATIO);
            pairW = Math.max(pairW, Math.round(SIDE_CLICK_MIN_WIDTH_DP * density));
            pairW = Math.min(pairW, Math.round(SIDE_CLICK_PORTRAIT_MAX_WIDTH_DP * density));
            pairW = Math.min(pairW, Math.max(1, parentW - margin * 2));

            int topGap = Math.round(SIDE_CLICK_PORTRAIT_TOP_GAP_DP * density);
            int bottomMargin = Math.round(SIDE_CLICK_PORTRAIT_BOTTOM_MARGIN_DP * density);
            int belowVideo = Math.round(videoTop + videoH) + topGap;
            int available = Math.max(1, parentH - belowVideo - bottomMargin);
            pairH = Math.min(available, Math.round(
                    SIDE_CLICK_PORTRAIT_MAX_HEIGHT_DP * density));
            pairH = Math.max(pairH, Math.min(available,
                    Math.round(84f * density) + gap));
            x = leftHanded ? parentW - pairW - margin : margin;
            y = belowVideo;
        } else {
            pairW = sideW;
            pairH = Math.round(parentH * SIDE_CLICK_HEIGHT_RATIO);
            pairH = Math.min(pairH, Math.round(videoH));
            x = leftHanded ? parentW - pairW - margin : margin;
            int centerY = Math.round(videoTop + videoH / 2f);
            y = Math.round(clamp(centerY - pairH / 2f, 0f,
                    Math.max(0, parentH - pairH)));
        }

        FrameLayout.LayoutParams lp = (FrameLayout.LayoutParams) sideClicks.getLayoutParams();
        lp.width = pairW;
        lp.height = pairH;
        lp.gravity = android.view.Gravity.TOP | android.view.Gravity.LEFT;
        lp.leftMargin = x;
        lp.topMargin = y;
        sideClicks.setLayoutParams(lp);

        if (sideBtnLeftClick != null) {
            LinearLayout.LayoutParams first =
                    (LinearLayout.LayoutParams) sideBtnLeftClick.getLayoutParams();
            first.width = LinearLayout.LayoutParams.MATCH_PARENT;
            first.height = 0;
            first.weight = 1f;
            first.topMargin = 0;
            first.bottomMargin = gap / 2;
            sideBtnLeftClick.setLayoutParams(first);
        }
        if (sideBtnRightClick != null) {
            LinearLayout.LayoutParams second =
                    (LinearLayout.LayoutParams) sideBtnRightClick.getLayoutParams();
            second.width = LinearLayout.LayoutParams.MATCH_PARENT;
            second.height = 0;
            second.weight = 1f;
            second.topMargin = gap - gap / 2;
            second.bottomMargin = 0;
            sideBtnRightClick.setLayoutParams(second);
        }
        sideClicks.setVisibility(View.VISIBLE);
        sideClicks.post(this::updateGestureExclusions);
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
            return;
        }
        if (BuildConfig.DEBUG) {
            Log.d("PCHomeTouch", "direct action=" + action + " touch=" + x + "," + y
                    + " view=" + viewW + "x" + viewH
                    + " pc=" + pcWidth + "x" + pcHeight
                    + " -> " + pcX + "," + pcY);
        }
        switch (action) {
            case MotionEvent.ACTION_DOWN:
                downX = x;
                downY = y;
                downTime = System.currentTimeMillis();
                moved = false;
                longPressFired = false;
                pressed = true;
                sendPointerMove(Math.round(pcX), Math.round(pcY), true);
                sendClickAt("left", "down", pcX, pcY);
                touchHandler.postDelayed(() -> {
                    if (pressed && !moved && !longPressFired) {
                        // Left button was pressed for dragging; release it
                        // before starting the right-button long press.
                        longPressFired = true;
                        sendClickAt("left", "up", virtX, virtY);
                        sendClickAt("right", "down", virtX, virtY);
                    }
                }, LONG_PRESS_MS);
                break;
            case MotionEvent.ACTION_MOVE:
                if (Math.abs(x - downX) > MOVE_THRESHOLD || Math.abs(y - downY) > MOVE_THRESHOLD) {
                    moved = true;
                }
                sendPointerMove(Math.round(pcX), Math.round(pcY), false);
                break;
            case MotionEvent.ACTION_UP:
                pressed = false;
                flushPendingPointer();
                if (longPressFired) {
                    sendClickAt("right", "up", virtX, virtY);
                } else {
                    sendClickAt("left", "up", virtX, virtY);
                    // İki dokunuş OS'nin çift tıklama aralığındaysa zaten
                    // çift tıklama olur; ayrıca "double" göndermiyoruz.
                    lastTapTime = System.currentTimeMillis();
                }
                break;
            case MotionEvent.ACTION_CANCEL:
                pressed = false;
                longPressFired = false;
                flushPendingPointer();
                sendClickAt("left", "up", virtX, virtY);
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
                placeTrackpadCursor(x, y, viewW, viewH, true);
                touchHandler.postDelayed(() -> {
                    if (pressed && !moved && !longPressFired) {
                        longPressFired = true;
                        sendClickAt("right", "down", virtX, virtY);
                    }
                }, LONG_PRESS_MS);
                break;
            case MotionEvent.ACTION_MOVE:
                lastX = x;
                lastY = y;
                if (Math.abs(x - downX) > MOVE_THRESHOLD || Math.abs(y - downY) > MOVE_THRESHOLD) {
                    moved = true;
                }
                placeTrackpadCursor(x, y, viewW, viewH, false);
                break;
            case MotionEvent.ACTION_UP:
                pressed = false;
                flushPendingPointer();
                if (longPressFired) {
                    sendClickAt("right", "up", virtX, virtY);
                } else if (!moved) {
                    sendClickAt("left", "click", virtX, virtY);
                }
                break;
            case MotionEvent.ACTION_CANCEL:
                pressed = false;
                if (longPressFired) {
                    sendClickAt("right", "up", virtX, virtY);
                }
                longPressFired = false;
                flushPendingPointer();
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
     * imleç parmağın solunda, sol el seçiliyse sağında kalır.
     *
     * Sağ elde imleç parmağın solunda 130dp boşlukla kalır. Sol kenara
     * yaslamak için parmağın en sola gitmesi gerekmez; parmak boşluk kadar
     * solda yeter. Yalnızca sağ kenara tam yaslanırken boşluk son bölgede
     * azalır. Sol elde bu durumun tam tersi geçerlidir.
     * Dikeyde merkez sabit, uçlara doğru 1.25x ileri gider.
     */
    private void placeTrackpadCursor(float x, float y, int viewW, int viewH,
                                     boolean announceMode) {
        float[] r = videoRect(viewW, viewH);
        float[] mapped = mapToPc(x, y, viewW, viewH);
        float density = getResources().getDisplayMetrics().density;
        float gapPc = (TRACKPAD_GAP_DP * density) / Math.max(1f, r[2]) * pcWidth;
        float normX = mapped[0] / Math.max(1f, pcWidth - 1f);
        float gapNorm = gapPc / Math.max(1f, pcWidth - 1f);
        float taperZone = clamp(Math.max(TRACKPAD_EDGE_TAPER, gapNorm * 1.12f),
                0.05f, 0.48f);
        float taper;
        if (leftHanded) {
            // İmleç parmağın sağında: sağ kenarda boşluk korunur; sol
            // kenara tam yaslanırken son bölgede boşluk azalır.
            taper = normX < taperZone ? normX / taperZone : 1f;
        } else {
            // İmleç parmağın solunda: sol kenarda boşluk korunur; sağ
            // kenara tam yaslanırken son bölgede boşluk azalır.
            taper = normX > 1f - taperZone ? (1f - normX) / taperZone : 1f;
        }
        float side = leftHanded ? 1f : -1f;
        float targetX = mapped[0] + side * gapPc * taper;

        float cy = pcHeight / 2f;
        float targetY = cy + (mapped[1] - cy) * TRACKPAD_Y_GAIN;
        targetX = clamp(targetX, 0, pcWidth - 1);
        targetY = clamp(targetY, 0, pcHeight - 1);
        virtX = targetX;
        virtY = targetY;
        sendPointerMove(Math.round(targetX), Math.round(targetY), announceMode);
        if (announceMode) {
            sendMode(); // Kip satırı trackpad kalsın.
        }
    }

    /**
     * İmleci en fazla POINTER_SEND_INTERVAL_MS aralıkla gönderir. Aradaki
     * hareketler atlanır; hep son konum gider. Böylece ağ tamponu şişmez ve
     * imleç parmağın arkasında birikmez.
     */
    private void sendPointerMove(int x, int y, boolean force) {
        pendingPointerX = x;
        pendingPointerY = y;
        hasPendingPointer = true;
        long now = SystemClock.uptimeMillis();
        long elapsed = now - lastPointerSentAt;
        if (force || elapsed >= POINTER_SEND_INTERVAL_MS) {
            flushPendingPointer();
        } else if (!pointerFlushScheduled && touchHandler != null) {
            pointerFlushScheduled = true;
            touchHandler.postDelayed(pointerFlushRunnable,
                    POINTER_SEND_INTERVAL_MS - elapsed);
        }
    }

    private void flushPendingPointer() {
        pointerFlushScheduled = false;
        if (!hasPendingPointer) {
            return;
        }
        hasPendingPointer = false;
        lastPointerSentAt = SystemClock.uptimeMillis();
        sendMoveAbs(pendingPointerX, pendingPointerY);
    }

    private void sendMode() {
        sendInput(new ControlMessage()
                .type("mode")
                .put("mode", mode == MODE_DIRECT ? "direct" : "trackpad")
                .toJson());
    }

    private void sendMoveAbs(int x, int y) {
        sendControl(new ControlMessage()
                .type("move_abs")
                .put("x", x)
                .put("y", y)
                .put("seq", pointerSeq.incrementAndGet())
                .toJson());
    }

    private void sendClick(String button, String action) {
        trackMouseButton(button, action);
        sendInput(new ControlMessage()
                .type("click")
                .put("button", button)
                .put("action", action)
                .put("seq", pointerSeq.incrementAndGet())
                .toJson());
    }

    private void sendClickAt(String button, String action, float pcX, float pcY) {
        trackMouseButton(button, action);
        sendInput(new ControlMessage()
                .type("click")
                .put("button", button)
                .put("action", action)
                .put("x", Math.round(pcX))
                .put("y", Math.round(pcY))
                .put("seq", pointerSeq.incrementAndGet())
                .toJson());
    }

    private void trackMouseButton(String button, String action) {
        if ("down".equals(action) || "hold".equals(action)) {
            heldMouseButtons.add(button);
        } else if ("up".equals(action)) {
            heldMouseButtons.remove(button);
        }
    }

    private void sendScrollHiRes(int dx, int dy) {
        sendControl(new ControlMessage()
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
        if (down) {
            heldKeys.add(code);
        } else {
            heldKeys.remove(code);
        }
        sendInput(new ControlMessage().type("key")
                .put("code", code)
                .put("action", down ? "down" : "up")
                .toJson());
    }

    private void sendCombo(int code, String mod) {
        sendInput(new ControlMessage()
                .type("combo")
                .put("code", code)
                .put("mod", mod)
                .toJson());
    }

    private void sendControl(String json) {
        if (webRtcClient != null) {
            webRtcClient.sendControl(json);
        }
    }

    private void sendInput(String json) {
        if (webRtcClient != null) {
            webRtcClient.sendInput(json);
        }
    }

    @Override
    public boolean onKeyDown(int keyCode, android.view.KeyEvent event) {
        int linux = KeyCodeMap.androidToLinux(keyCode);
        if (linux != -1) {
            sendKey(linux, true);
            hideSystemBarsSoon();
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
                releaseAllHeldInput();
                saveRecent();
                if (emptyGrid != null) emptyGrid.setVisibility(View.VISIBLE);
                streamStatus.setText(R.string.disconnected);
                streamStatus.setVisibility(View.VISIBLE);
            } else if (state == WebRtcClient.ConnectionState.FAILED) {
                releaseAllHeldInput();
                saveRecent();
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
            if (error.contains("reddedildi") || error.contains("hatalı")
                    || error.contains("rejected") || error.contains("invalid")) {
                saveRecent();
                showFatal(getString(R.string.connection_rejected), error);
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
        String res = deviceRes.isEmpty() ? getString(R.string.preparing) : deviceRes;
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
        releaseAllHeldInput();
        stopScrollFling();
        hasPendingPointer = false;
        pointerFlushScheduled = false;
        if (uiHandler != null) {
            uiHandler.removeCallbacks(timeoutCheck);
            uiHandler.removeCallbacks(hideSystemBarsRunnable);
        }
        if (touchHandler != null) {
            touchHandler.removeCallbacks(pointerFlushRunnable);
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

        ControlMessage put(String k, long v) {
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
