package com.pchome.mobile;

import android.content.Intent;
import android.os.Bundle;
import android.text.Editable;
import android.text.TextWatcher;
import android.view.View;
import android.widget.ArrayAdapter;
import android.widget.AutoCompleteTextView;

import androidx.appcompat.app.AppCompatActivity;
import androidx.recyclerview.widget.GridLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import com.google.android.material.button.MaterialButton;
import com.google.android.material.checkbox.MaterialCheckBox;
import com.google.android.material.chip.Chip;
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
    private Chip statusText;
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
        statusText = findViewById(R.id.status);
        recentGrid = findViewById(R.id.recent_grid);
        recentEmpty = findViewById(R.id.recent_empty);
        MaterialTextView versionFooter = findViewById(R.id.version_footer);
        versionFooter.setText("PChome v" + BuildConfig.VERSION_NAME);
        prefs = new Prefs(this);
        recentStore = new RecentStore(this);

        // Kayıtlı kişisel sunucu adresi (derleme anında local.properties'ten
        // gömülür); kullanıcı yine de değiştirebilir.
        String baked = BuildConfig.SIGNAL_URL;
        List<String> servers = prefs.getServers();
        if (!servers.contains(baked) && !baked.isEmpty()) {
            servers.add(0, baked);
        }
        if (servers.isEmpty()) {
            servers.add(0, "wss://pchome.onrender.com/ws");
        }

        ArrayAdapter<String> adapter = new ArrayAdapter<>(
                this, android.R.layout.simple_dropdown_item_1line, servers);
        serverEdit.setAdapter(adapter);
        int lastIdx = Math.min(prefs.getLastServerIndex(), servers.size() - 1);
        serverEdit.setText(servers.get(Math.max(0, lastIdx)), false);

        TextWatcher clearStatus = new TextWatcher() {
            @Override public void beforeTextChanged(CharSequence s, int a, int b, int c) {}
            @Override public void onTextChanged(CharSequence s, int a, int b, int c) {}
            @Override public void afterTextChanged(Editable s) { statusText.setText(R.string.idle); }
        };
        serverEdit.addTextChangedListener(clearStatus);
        pcIdEdit.addTextChangedListener(clearStatus);
        passwordEdit.addTextChangedListener(clearStatus);

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
        connectButton.setOnClickListener(v -> {
            String server = serverEdit.getText().toString().trim();
            String pcId = pcIdEdit.getText().toString().replaceAll("[^0-9]", "");
            String password = passwordEdit.getText().toString().replaceAll("[^0-9]", "");
            if (server.isEmpty()) {
                statusText.setText(getString(R.string.error) + ": sunucu adresi gir");
                return;
            }
            if (pcId.length() != 6) {
                statusText.setText(getString(R.string.error) + ": PC ID 6 hane olmalı");
                return;
            }
            if (password.length() != 6) {
                statusText.setText(getString(R.string.error) + ": şifre 6 hane olmalı");
                return;
            }
            openConnection(server, pcId, password, rememberCheck.isChecked());
        });
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

    /** Güncelleme denetimi: günde en fazla 1 kez, kart hazırsa gösterilir. */
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
                if (info.available && !isFinishing()) {
                    View card = findViewById(R.id.update_card);
                    MaterialTextView banner = findViewById(R.id.update_banner);
                    banner.setText(getString(R.string.update_available, info.version));
                    card.setVisibility(View.VISIBLE);
                    findViewById(R.id.update_button).setOnClickListener(v ->
                            UpdateChecker.downloadAndInstall(PinActivity.this, info));
                }
            }

            @Override
            public void onFailure(String error) {
                // Sessiz geç: bağlantı ekranı etkilenmez.
            }
        }, main);
    }
}
