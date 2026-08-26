package com.pchome.mobile;

import android.content.Intent;
import android.os.Bundle;
import android.text.Editable;
import android.text.TextWatcher;
import android.view.View;
import android.widget.ArrayAdapter;
import android.widget.AutoCompleteTextView;

import androidx.appcompat.app.AppCompatActivity;

import com.google.android.material.chip.Chip;
import com.google.android.material.chip.ChipGroup;
import com.google.android.material.materialswitch.MaterialSwitch;
import com.google.android.material.textfield.TextInputEditText;
import com.google.android.material.textview.MaterialTextView;

import java.util.List;

public class PinActivity extends AppCompatActivity {
    private AutoCompleteTextView serverEdit;
    private TextInputEditText pinEdit;
    private MaterialTextView statusText;
    private ChipGroup recentServers;
    private Prefs prefs;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_pin);

        serverEdit = findViewById(R.id.server);
        pinEdit = findViewById(R.id.pin_edit);
        statusText = findViewById(R.id.status);
        recentServers = findViewById(R.id.recent_servers);
        prefs = new Prefs(this);

        // Pre-fill the personal signal URL baked in from local.properties
        // (signalUrl=...) at build time; the user can still override it here.
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

        // Recent server chips (tap to fill the field).
        if (servers.size() > 1) {
            recentServers.setVisibility(View.VISIBLE);
            for (String s : servers.subList(1, servers.size())) {
                Chip chip = new Chip(this);
                chip.setText(s.replace("wss://", "").replace("ws://", "").replace("/ws", ""));
                chip.setCheckable(false);
                chip.setOnClickListener(v -> serverEdit.setText(s, false));
                recentServers.addView(chip);
            }
        }

        serverEdit.addTextChangedListener(new TextWatcher() {
            @Override public void beforeTextChanged(CharSequence s, int a, int b, int c) {}
            @Override public void onTextChanged(CharSequence s, int a, int b, int c) {}
            @Override public void afterTextChanged(Editable s) { statusText.setText(R.string.idle); }
        });

        findViewById(R.id.connect_button).setOnClickListener(v -> {
            String server = serverEdit.getText().toString().trim();
            String pin = pinEdit.getText().toString().replaceAll("[^0-9]", "");
            if (server.isEmpty() || pin.isEmpty()) {
                statusText.setText(R.string.error + ": enter server + PIN");
                return;
            }
            prefs.addServer(server);
            prefs.addPin(pin);

            Intent intent = new Intent(this, DisplayActivity.class);
            intent.putExtra("signalUrl", server);
            intent.putExtra("pin", pin);
            startActivity(intent);
            finish();
        });
    }
}
