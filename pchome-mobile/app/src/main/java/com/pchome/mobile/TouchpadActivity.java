package com.pchome.mobile;

import android.os.Bundle;
import android.view.GestureDetector;
import android.view.MotionEvent;
import android.view.View;
import android.widget.ImageButton;

import androidx.appcompat.app.AppCompatActivity;
import androidx.core.view.GestureDetectorCompat;

public class TouchpadActivity extends AppCompatActivity {
    private View touchpadContainer;
    private ImageButton leftClickButton;
    private ImageButton rightClickButton;
    private GestureDetectorCompat gestureDetector;
    private AndroidControlService controlService;
    private WebRtcClient webRtcClient;

    private static final int SWIPE_THRESHOLD = 50;
    private static final int SWIPE_VELOCITY_THRESHOLD = 50;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_touchpad);

        touchpadContainer = findViewById(R.id.touchpad_container);
        leftClickButton = findViewById(R.id.btn_left_click);
        rightClickButton = findViewById(R.id.btn_right_click);

        controlService = new AndroidControlService();

        gestureDetector = new GestureDetectorCompat(this, new GestureDetector.SimpleOnGestureListener() {
            @Override
            public boolean onDown(MotionEvent e) {
                return true;
            }

            @Override
            public boolean onScroll(MotionEvent e1, MotionEvent e2, float distanceX, float distanceY) {
                float x = e2.getX() - e1.getX();
                float y = e2.getY() - e1.getY();
                controlService.scroll(e1.getX(), e1.getY(), e2.getX(), e2.getY(), 100);
                return true;
            }

            @Override
            public boolean onDoubleTap(MotionEvent e) {
                controlService.click(e.getX(), e.getY());
                controlService.click(e.getX(), e.getY());
                return true;
            }

            @Override
            public void onLongPress(MotionEvent e) {
                controlService.longClick(e.getX(), e.getY());
            }
        });

        touchpadContainer.setOnTouchListener((v, event) -> gestureDetector.onTouchEvent(event));

        leftClickButton.setOnClickListener(v -> {
            float x = touchpadContainer.getWidth() / 2f;
            float y = touchpadContainer.getHeight() / 2f;
            controlService.click(x, y);
        });

        rightClickButton.setOnClickListener(v -> {
            float x = touchpadContainer.getWidth() / 2f;
            float y = touchpadContainer.getHeight() / 2f;
            controlService.longClick(x, y);
        });
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
    }
}
