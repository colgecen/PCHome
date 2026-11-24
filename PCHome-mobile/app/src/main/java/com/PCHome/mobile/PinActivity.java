package com.PCHome.mobile;

import android.content.Intent;
import android.os.Bundle;
import android.view.View;
import android.widget.ArrayAdapter;
import android.widget.AutoCompleteTextView;
import android.widget.ProgressBar;
import android.widget.ScrollView;
import android.widget.Toast;

import androidx.appcompat.app.AlertDialog;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.graphics.Insets;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import com.airbnb.lottie.LottieAnimationView;
import com.google.android.material.button.MaterialButton;
import com.google.android.material.materialswitch.MaterialSwitch;
import com.google.android.material.textfield.TextInputEditText;
import com.google.android.material.textview.MaterialTextView;

import java.util.List;

/**
 * Bağlantı ekranı: sunucu adresi + kalıcı PC ID + sabit şifre.
 * Altta "Son Girilenler" ızgarası: kare kutucukta PC son görüntüsü,
 * altında isim + ID/IP/MAC + son görülme. Karta dokununca kayıtlı
 * bilgilerle doğrudan bağlanılır (uzun basınca silinir).
 */
public class PinActivity extends AppCompatActivity {
    private AutoCompleteTextView serverEdit;
    private TextInputEditText pcIdEdit;
    private TextInputEditText passwordEdit;
    private MaterialSwitch rememberCheck;
    private MaterialTextView updateStateText;
    private RecyclerView recentGrid;
    private MaterialTextView recentEmpty;
    private LottieAnimationView heroPulse;
    private Prefs prefs;
    private RecentStore recentStore;
    private RecentAdapter recentAdapter;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        WindowCompat.setDecorFitsSystemWindows(getWindow(), false);
        Diag.init(this);
        setContentView(R.layout.activity_pin);
        showPendingCrash();
        applyWindowInsets();

        serverEdit = findViewById(R.id.server);
        pcIdEdit = findViewById(R.id.pc_id_input);
        passwordEdit = findViewById(R.id.password_input);
        rememberCheck = findViewById(R.id.remember_check);
        recentGrid = findViewById(R.id.recent_grid);
        recentEmpty = findViewById(R.id.recent_empty);
        updateStateText = findViewById(R.id.update_state_text);
        updateStateText.setText(getString(R.string.update_idle));
        heroPulse = findViewById(R.id.hero_pulse);
        MaterialTextView versionFooter = findViewById(R.id.version_footer);
        versionFooter.setText("PCHome v" + BuildConfig.VERSION_NAME
                + " • WebRTC • uçtan uca");
        MaterialTextView versionPill = findViewById(R.id.version_pill);
        versionPill.setText(getString(R.string.version_pill, BuildConfig.VERSION_NAME));
        prefs = new Prefs(this);
        recentStore = new RecentStore(this);
        playEntrance();

        // Sunucu adresi gömülü GELMEZ (anonim): kullanıcı eliyle yazar.
        // Daha önce yazılan adresler aşağıda öneri olarak çıkar.
        List<String> servers = prefs.getServers();

        ArrayAdapter<String> adapter = new ArrayAdapter<>(
                this, android.R.layout.simple_dropdown_item_1line, servers);
        serverEdit.setAdapter(adapter);
        if (!servers.isEmpty()) {
            // Prefs listesi en son kullanılan başta olacak şekilde sıralıdır.
            serverEdit.setText(servers.get(0), false);
        }

        // En alt güncelleme bölümü: modalı açar (masaüstüyle aynı mantık).
        findViewById(R.id.update_open_button).setOnClickListener(v -> openUpdateDialog());

        // Klavye akışı: sunucu → enter → ID → enter → şifre → enter → bağlan.
        serverEdit.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == android.view.inputmethod.EditorInfo.IME_ACTION_NEXT) {
                pcIdEdit.requestFocus();
                return true;
            }
            return false;
        });
        pcIdEdit.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == android.view.inputmethod.EditorInfo.IME_ACTION_NEXT) {
                passwordEdit.requestFocus();
                return true;
            }
            return false;
        });
        passwordEdit.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == android.view.inputmethod.EditorInfo.IME_ACTION_DONE) {
                attemptConnect();
                return true;
            }
            return false;
        });

        // Son bağlantılar: yatay, kaydırılabilir kart şeridi.
        recentGrid.setLayoutManager(new LinearLayoutManager(
                this, LinearLayoutManager.HORIZONTAL, false));
        recentGrid.setItemAnimator(null);
        recentAdapter = new RecentAdapter(new RecentAdapter.Listener() {
            @Override
            public void onOpen(RecentPc pc) {
                // Kart kendi sunucusunu hatırlıyorsa onu kullan; yoksa
                // ekrandaki güncel adrese düş.
                String server = pc.server == null ? "" : pc.server.trim();
                if (server.isEmpty()) {
                    server = serverEdit.getText().toString().trim();
                }
                if (server.isEmpty()) {
                    toast("Önce sinyal sunucusu adresini gir");
                    serverEdit.requestFocus();
                    return;
                }
                serverEdit.setText(server, false);
                if (pc.rememberPassword && pc.password.length() == 6) {
                    openConnection(server, pc.pcId, pc.password, true);
                } else {
                    // Şifre kayıtlı değilse forma doldur, kullanıcı yazsın.
                    pcIdEdit.setText(pc.pcId);
                    passwordEdit.setText("");
                    passwordEdit.requestFocus();
                    toast("Karttaki şifreyi yazıp bağlan");
                }
            }

            @Override
            public void onDelete(RecentPc pc) {
                recentStore.remove(pc.pcId);
                refreshRecent();
            }
        });
        recentGrid.setAdapter(recentAdapter);

        MaterialButton connectButton = findViewById(R.id.connect_button);
        connectButton.setOnClickListener(v -> {
            tick();
            attemptConnect();
        });
    }

    /** Edge-to-edge: durum çubuğu/klavye altında kalan içeriği kaydır. */
    private void applyWindowInsets() {
        final View scroll = findViewById(R.id.pin_scroll);
        ViewCompat.setOnApplyWindowInsetsListener(scroll, (v, insets) -> {
            Insets bars = insets.getInsets(
                    WindowInsetsCompat.Type.systemBars()
                            | WindowInsetsCompat.Type.displayCutout()
                            | WindowInsetsCompat.Type.ime());
            v.setPadding(0, bars.top, 0, bars.bottom);
            return insets;
        });
    }

    /** Açılışta içeriği yumuşakça yukarı kaydırarak göster. */
    private void playEntrance() {
        View content = findViewById(R.id.pin_content);
        float dy = 18f * getResources().getDisplayMetrics().density;
        content.setAlpha(0f);
        content.setTranslationY(dy);
        content.animate()
                .alpha(1f)
                .translationY(0f)
                .setDuration(420L)
                .setInterpolator(new android.view.animation.DecelerateInterpolator())
                .start();
    }

    /** 25 ms'lik dokunsal geri bildirim. */
    private void tick() {
        try {
            android.os.Vibrator vib =
                    (android.os.Vibrator) getSystemService(VIBRATOR_SERVICE);
            if (vib != null && vib.hasVibrator()) {
                if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
                    vib.vibrate(android.os.VibrationEffect.createOneShot(
                            25, android.os.VibrationEffect.DEFAULT_AMPLITUDE));
                } else {
                    vib.vibrate(25);
                }
            }
        } catch (Exception ignored) {
        }
    }

    /**
     * Önceki çalışta çökme olduysa raporu gösterir (kopyalanabilir tanı).
     * Çökme yoksa sessiz geçilir.
     */
    private void showPendingCrash() {
        String crash = Diag.readCrash(this);
        if (crash.isEmpty() || isFinishing()) {
            return;
        }
        String phases = Diag.readPhases(this);
        String body = crash;
        if (!phases.isEmpty()) {
            String tail = phases.length() > 2000
                    ? phases.substring(phases.length() - 2000) : phases;
            body += "\n--- son aşamalar ---\n" + tail;
        }
        android.widget.ScrollView scroll = new android.widget.ScrollView(this);
        android.widget.TextView tv = new android.widget.TextView(this);
        tv.setText(body);
        tv.setTextIsSelectable(true);
        tv.setPadding(24, 24, 24, 24);
        scroll.addView(tv);
        new com.google.android.material.dialog.MaterialAlertDialogBuilder(this)
                .setTitle("Önceki çalışta çökme oldu")
                .setMessage("Bu rapor bağlantı sorununun sebebini gösterir:")
                .setView(scroll)
                .setPositiveButton("Temizle", (d, w) -> Diag.clear(this))
                .setNegativeButton("Kapat", (d, w) -> {
                })
                .setOnDismissListener(d -> Diag.clear(this))
                .show();
    }

    /** Kısa hata balonu (eski durum çipinin yerini tutar). */
    private void toast(String msg) {
        Toast.makeText(this, msg, Toast.LENGTH_SHORT).show();
    }

    /** Alanları doğrula ve bağlan (düğme + klavye-enter ortak yolu). */
    private void attemptConnect() {
        String server = serverEdit.getText().toString().trim();
        String pcId = pcIdEdit.getText().toString().replaceAll("[^0-9]", "");
        String password = passwordEdit.getText().toString().replaceAll("[^0-9]", "");
        if (server.isEmpty()) {
            toast(getString(R.string.error) + ": sunucu adresi gir");
            serverEdit.requestFocus();
            return;
        }
        if (pcId.length() != 6) {
            toast(getString(R.string.error) + ": PC ID 6 hane olmalı");
            pcIdEdit.requestFocus();
            return;
        }
        if (password.length() != 6) {
            toast(getString(R.string.error) + ": şifre 6 hane olmalı");
            passwordEdit.requestFocus();
            return;
        }
        openConnection(server, pcId, password, rememberCheck.isChecked());
    }

    @Override
    protected void onPause() {
        if (heroPulse != null) {
            heroPulse.pauseAnimation();
        }
        super.onPause();
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (heroPulse != null) {
            heroPulse.playAnimation();
        }
        refreshRecent();
        // Güncelleme denetimi (Faz 5'te UpdateChecker bağlanır).
        checkForUpdate();
    }

    private void refreshRecent() {
        List<RecentPc> items = recentStore.list();
        recentAdapter.setItems(items);
        recentEmpty.setVisibility(items.isEmpty() ? View.VISIBLE : View.GONE);
        // Form boşsa en son cihazı doldur: tek dokunuşla "Bağlan" yetsin.
        if (!items.isEmpty() && pcIdEdit.getText().length() == 0) {
            RecentPc latest = items.get(0);
            pcIdEdit.setText(latest.pcId);
            if (latest.rememberPassword && latest.password.length() == 6) {
                passwordEdit.setText(latest.password);
                rememberCheck.setChecked(true);
            }
        }
    }

    private void openConnection(String server, String pcId, String password, boolean remember) {
        server = SignalClient.normalizeServer(server);
        prefs.addServer(server);

        // Kartı hemen başa taşı (cihaz bilgisi + görüntü kapanışta dolar).
        RecentPc entry = recentStore.find(pcId);
        if (entry == null) {
            entry = new RecentPc(pcId, "");
        }
        entry.password = remember ? password : "";
        entry.rememberPassword = remember;
        entry.server = server;
        entry.lastSeen = System.currentTimeMillis();
        recentStore.upsert(entry);

        Intent intent = new Intent(this, DisplayActivity.class);
        intent.putExtra("signalUrl", server);
        intent.putExtra("pcId", pcId);
        intent.putExtra("password", password);
        startActivity(intent);
    }

    /** Arka plan denetimi: günde en fazla 1 kez, sonucu alt karta yazar. */
    private void checkForUpdate() {
        long now = System.currentTimeMillis();
        if (now - prefs.getLastUpdateCheck() < 24L * 3600 * 1000) {
            return;
        }
        prefs.setLastUpdateCheck(now);
        android.os.Handler main = new android.os.Handler(getMainLooper());
        UpdateChecker.check(BuildConfig.VERSION_NAME, new UpdateChecker.Callback() {
            @Override
            public void onResult(UpdateChecker.UpdateInfo info) {
                if (isFinishing()) {
                    return;
                }
                if (info.available) {
                    updateStateText.setText(getString(R.string.update_available,
                            info.version, UpdateChecker.humanSize(info.sizeBytes)));
                    updateStateText.setTextColor(getColor(R.color.PCHome_success));
                } else {
                    updateStateText.setText(getString(R.string.update_uptodate,
                            BuildConfig.VERSION_NAME));
                }
            }

            @Override
            public void onFailure(String error) {
                if (!isFinishing()) {
                    updateStateText.setText(getString(R.string.update_failed, error));
                }
            }
        }, main);
    }

    /**
     * Güncelleme modalı (masaüstüyle aynı mantık): Kontrol Et → sonuç →
     * güncelle. Kurulum bitmeden modal kapanmaz; bitince sistem kurulum
     * ekranı açılır (Android uygulamayı kendi yeniden başlatır).
     */
    private void openUpdateDialog() {
        View body = getLayoutInflater().inflate(R.layout.dialog_update, null);
        MaterialTextView title = body.findViewById(R.id.update_dialog_title);
        MaterialTextView text = body.findViewById(R.id.update_dialog_text);
        MaterialTextView notes = body.findViewById(R.id.update_dialog_notes);
        ScrollView notesScroll = body.findViewById(R.id.update_dialog_notes_scroll);
        ProgressBar progress = body.findViewById(R.id.update_dialog_progress);
        MaterialButton action = body.findViewById(R.id.update_dialog_action);
        MaterialButton close = body.findViewById(R.id.update_dialog_close);

        title.setText(R.string.update_title);
        AlertDialog dialog = new AlertDialog.Builder(this)
                .setView(body)
                .setCancelable(true)
                .create();

        Runnable showIdle = () -> {
            text.setText(R.string.update_idle_dialog);
            notesScroll.setVisibility(View.GONE);
            progress.setVisibility(View.GONE);
            action.setVisibility(View.VISIBLE);
            action.setText(R.string.update_check_now);
            dialog.setCancelable(true);
            dialog.setCanceledOnTouchOutside(true);
        };
        showIdle.run();

        action.setOnClickListener(v -> {
            CharSequence label = action.getText();
            if (getString(R.string.update_retry).contentEquals(label)) {
                showIdle.run();
                return;
            }
            if (getString(R.string.update_check_now).contentEquals(label)) {
                progress.setVisibility(View.VISIBLE);
                text.setText(R.string.update_checking);
                action.setVisibility(View.GONE);
                android.os.Handler main = new android.os.Handler(getMainLooper());
                UpdateChecker.check(BuildConfig.VERSION_NAME, new UpdateChecker.Callback() {
                    @Override
                    public void onResult(UpdateChecker.UpdateInfo info) {
                        if (isFinishing()) {
                            return;
                        }
                        progress.setVisibility(View.GONE);
                        if (info.available) {
                            text.setText(getString(R.string.update_available,
                                    info.version, UpdateChecker.humanSize(info.sizeBytes)));
                            if (!info.notes.trim().isEmpty()) {
                                String cut = info.notes.length() > 800
                                        ? info.notes.substring(0, 800) + "…" : info.notes;
                                notes.setText(cut);
                                notesScroll.setVisibility(View.VISIBLE);
                            }
                            action.setVisibility(View.VISIBLE);
                            action.setText(R.string.update_now);
                            action.setOnClickListener(v2 -> {
                                // Kilit: bitmeden ekrandan çıkılamaz.
                                progress.setVisibility(View.VISIBLE);
                                text.setText(R.string.update_busy);
                                notesScroll.setVisibility(View.GONE);
                                action.setVisibility(View.GONE);
                                close.setVisibility(View.GONE);
                                dialog.setCancelable(false);
                                dialog.setCanceledOnTouchOutside(false);
                                updateStateText.setText(getString(R.string.update_busy));
                                UpdateChecker.downloadAndInstall(PinActivity.this, info,
                                        new UpdateChecker.DownloadCallback() {
                                            @Override
                                            public void onInstalled() {
                                                if (isFinishing()) {
                                                    return;
                                                }
                                                progress.setVisibility(View.GONE);
                                                text.setText("Kurulum ekranı açıldı ✓");
                                                close.setVisibility(View.VISIBLE);
                                                dialog.setCancelable(true);
                                                updateStateText.setText("Kurulum ekranı açıldı ✓");
                                            }

                                            @Override
                                            public void onFailed(String reason) {
                                                if (isFinishing()) {
                                                    return;
                                                }
                                                progress.setVisibility(View.GONE);
                                                text.setText(getString(R.string.update_failed, reason));
                                                action.setVisibility(View.VISIBLE);
                                                action.setText(R.string.update_retry);
                                                close.setVisibility(View.VISIBLE);
                                                dialog.setCancelable(true);
                                                dialog.setCanceledOnTouchOutside(true);
                                                updateStateText.setText(
                                                        getString(R.string.update_failed, reason));
                                            }
                                        });
                            });
                            updateStateText.setText(getString(R.string.update_available,
                                    info.version, UpdateChecker.humanSize(info.sizeBytes)));
                            updateStateText.setTextColor(getColor(R.color.PCHome_success));
                        } else {
                            text.setText(getString(R.string.update_uptodate,
                                    BuildConfig.VERSION_NAME));
                            action.setVisibility(View.GONE);
                            updateStateText.setText(getString(R.string.update_uptodate,
                                    BuildConfig.VERSION_NAME));
                        }
                    }

                    @Override
                    public void onFailure(String error) {
                        if (isFinishing()) {
                            return;
                        }
                        progress.setVisibility(View.GONE);
                        text.setText(getString(R.string.update_failed, error));
                        action.setVisibility(View.VISIBLE);
                        action.setText(R.string.update_retry);
                    }
                }, main);
            }
        });
        close.setOnClickListener(v -> dialog.dismiss());
        dialog.show();
    }
}
