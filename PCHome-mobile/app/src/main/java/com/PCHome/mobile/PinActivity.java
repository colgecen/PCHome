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
import androidx.recyclerview.widget.GridLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import com.google.android.material.button.MaterialButton;
import com.google.android.material.checkbox.MaterialCheckBox;
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
    private MaterialCheckBox rememberCheck;
    private MaterialTextView updateStateText;
    private RecyclerView recentGrid;
    private MaterialTextView recentEmpty;
    private Prefs prefs;
    private RecentStore recentStore;
    private RecentAdapter recentAdapter;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_pin);

        serverEdit = findViewById(R.id.server);
        pcIdEdit = findViewById(R.id.pc_id_input);
        passwordEdit = findViewById(R.id.password_input);
        rememberCheck = findViewById(R.id.remember_check);
        recentGrid = findViewById(R.id.recent_grid);
        recentEmpty = findViewById(R.id.recent_empty);
        updateStateText = findViewById(R.id.update_state_text);
        updateStateText.setText(getString(R.string.update_idle));
        MaterialTextView versionFooter = findViewById(R.id.version_footer);
        versionFooter.setText("PCHome v" + BuildConfig.VERSION_NAME);
        prefs = new Prefs(this);
        recentStore = new RecentStore(this);

        // Sunucu adresi gömülü GELMEZ (anonim): kullanıcı eliyle yazar.
        // Daha önce yazılan adresler aşağıda öneri olarak çıkar.
        List<String> servers = prefs.getServers();

        ArrayAdapter<String> adapter = new ArrayAdapter<>(
                this, android.R.layout.simple_dropdown_item_1line, servers);
        serverEdit.setAdapter(adapter);
        if (!servers.isEmpty()) {
            int lastIdx = Math.min(prefs.getLastServerIndex(), servers.size() - 1);
            serverEdit.setText(servers.get(Math.max(0, lastIdx)), false);
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

        // Son Girilenler ızgarası (2 sütun kare kart).
        recentGrid.setLayoutManager(new GridLayoutManager(this, 2));
        recentAdapter = new RecentAdapter(new RecentAdapter.Listener() {
            @Override
            public void onOpen(RecentPc pc) {
                openConnection(serverEdit.getText().toString().trim(), pc.pcId,
                        pc.rememberPassword ? pc.password : "", pc.rememberPassword);
            }

            @Override
            public void onDelete(RecentPc pc) {
                recentStore.remove(pc.pcId);
                refreshRecent();
            }
        });
        recentGrid.setAdapter(recentAdapter);

        MaterialButton connectButton = findViewById(R.id.connect_button);
        connectButton.setOnClickListener(v -> attemptConnect());
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
    protected void onResume() {
        super.onResume();
        refreshRecent();
        // Güncelleme denetimi (Faz 5'te UpdateChecker bağlanır).
        checkForUpdate();
    }

    private void refreshRecent() {
        List<RecentPc> items = recentStore.list();
        recentAdapter.setItems(items);
        recentEmpty.setVisibility(items.isEmpty() ? View.VISIBLE : View.GONE);
    }

    private String normalizeServer(String server) {
        if (!server.startsWith("wss://") && !server.startsWith("ws://")) {
            server = "wss://" + server;
        } else if (server.startsWith("ws://")) {
            server = "wss://" + server.substring(5);
        }
        if (!server.endsWith("/ws")) {
            server = server.replaceAll("/+$", "") + "/ws";
        }
        return server;
    }

    private void openConnection(String server, String pcId, String password, boolean remember) {
        server = normalizeServer(server);
        prefs.addServer(server);

        // Kartı hemen başa taşı (cihaz bilgisi + görüntü kapanışta dolar).
        RecentPc entry = recentStore.find(pcId);
        if (entry == null) {
            entry = new RecentPc(pcId, "");
        }
        entry.password = remember ? password : "";
        entry.rememberPassword = remember;
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

        title.setText(R.string.update_open);
        AlertDialog dialog = new AlertDialog.Builder(this)
                .setView(body)
                .setCancelable(true)
                .create();

        Runnable showIdle = () -> {
            text.setText(R.string.update_idle);
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
                                UpdateChecker.downloadAndInstall(PinActivity.this, info);
                                toast("Kurulum ekranı açılacak — bekle");
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
