package com.pchome.mobile;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.GestureDescription;
import android.graphics.Path;
import android.os.Build;
import android.util.Log;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;

import androidx.annotation.RequiresApi;

public class AndroidControlService extends AccessibilityService {
    private static final String TAG = "AndroidControlService";

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        Log.d(TAG, "Event: " + event.getEventType());
    }

    @Override
    public void onInterrupt() {
        Log.d(TAG, "Service interrupted");
    }

    public boolean injectKeyEvent(int keyCode, boolean pressed) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            return performGlobalAction(keyCode);
        }
        return false;
    }

    public boolean click(float x, float y) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            Path path = new Path();
            path.moveTo(x, y);
            GestureDescription gesture = new GestureDescription.Builder()
                    .addStroke(new GestureDescription.StrokeDescription(path, 0, 1))
                    .build();
            return dispatchGesture(gesture, null, null);
        }
        return false;
    }

    public boolean longClick(float x, float y) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            Path path = new Path();
            path.moveTo(x, y);
            GestureDescription gesture = new GestureDescription.Builder()
                    .addStroke(new GestureDescription.StrokeDescription(path, 0, 100))
                    .build();
            return dispatchGesture(gesture, null, null);
        }
        return false;
    }

    public boolean scroll(float startX, float startY, float endX, float endY, long duration) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            Path path = new Path();
            path.moveTo(startX, startY);
            path.lineTo(endX, endY);
            GestureDescription gesture = new GestureDescription.Builder()
                    .addStroke(new GestureDescription.StrokeDescription(path, 0, duration))
                    .build();
            return dispatchGesture(gesture, null, null);
        }
        return false;
    }

    public AccessibilityNodeInfo getRootNode() {
        return getRootInActiveWindow();
    }
}
