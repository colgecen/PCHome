package com.pchome.mobile;

import android.content.Intent;
import android.media.projection.MediaProjection;

/**
 * Holds the MediaProjection permission result so the WebRTC video capturer
 * (which is instantiated far from the Activity that obtained the permission)
 * can reuse the same projection Intent instead of being constructed with null
 * and crashing at runtime.
 */
public final class MediaProjectionHolder {
    private static Intent data;
    private static int resultCode = 0;
    private static MediaProjection projection;

    private MediaProjectionHolder() {
    }

    public static void set(Intent data, int resultCode) {
        MediaProjectionHolder.data = data;
        MediaProjectionHolder.resultCode = resultCode;
    }

    public static Intent getData() {
        return data;
    }

    public static int getResultCode() {
        return resultCode;
    }

    public static void setProjection(MediaProjection projection) {
        MediaProjectionHolder.projection = projection;
    }

    public static MediaProjection getProjection() {
        return projection;
    }

    public static boolean hasProjection() {
        return data != null;
    }

    public static void clear() {
        data = null;
        resultCode = 0;
        projection = null;
    }
}
