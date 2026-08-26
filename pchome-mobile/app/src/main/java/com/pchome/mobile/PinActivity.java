package com.pchome.mobile;

import android.content.Intent;
import android.os.Bundle;
import android.widget.Button;
import android.widget.EditText;
import android.widget.TextView;

import androidx.appcompat.app.AppCompatActivity;

public class PinActivity extends AppCompatActivity {
    private EditText serverEdit;
    private EditText pinEdit;
    private TextView statusText;
    private Button connectButton;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_pin);

        serverEdit = findViewById(R.id.server);
        pinEdit = findViewById(R.id.pin_edit);
        statusText = findViewById(R.id.status);
        connectButton = findViewById(R.id.connect_button);

        // Pre-fill the personal signal URL baked in from local.properties
        // (signalUrl=...) at build time; the user can still override it here.
        serverEdit.setText(BuildConfig.SIGNAL_URL);

        connectButton.setOnClickListener(v -> {
            String server = serverEdit.getText().toString().trim();
            String pin = pinEdit.getText().toString().replaceAll("[^0-9]", "");
            if (server.isEmpty() || pin.isEmpty()) {
                statusText.setText(R.string.error + ": enter server + PIN");
                return;
            }
            Intent intent = new Intent(this, DisplayActivity.class);
            intent.putExtra("signalUrl", server);
            intent.putExtra("pin", pin);
            startActivity(intent);
            finish();
        });
    }
}
