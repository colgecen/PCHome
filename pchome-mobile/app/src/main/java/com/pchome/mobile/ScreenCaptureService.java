package com.pchome.mobile;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.hardware.display.DisplayManager;
import android.hardware.display.VirtualDisplay;
import android.media.Image;
import android.media.ImageReader;
import android.media.projection.MediaProjection;
import android.media.projection.MediaProjectionManager;
import android.os.Build;
import android.os.IBinder;
import android.util.Log;
import android.util.Size;
import android.view.Surface;

import androidx.annotation.Nullable;
import androidx.core.app.NotificationCompat;

import java.nio.ByteBuffer;

public class ScreenCaptureService extends Service {
    private static final String TAG = "ScreenCaptureService";
    private static final String CHANNEL_ID = "ScreenCaptureChannel";
    private static final int NOTIFICATION_ID = 1;

    private MediaProjection mediaProjection;
    private VirtualDisplay virtualDisplay;
    private ImageReader imageReader;
    private MediaCodecEncoder encoder;

    @Override
    public void onCreate() {
        super.onCreate();
        createNotificationChannel();
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        int resultCode = intent.getIntExtra("resultCode", 0);
        android.content.Intent data = (android.content.Intent) intent.getParcelableExtra("data");

        startForegroundWithType();

        MediaProjectionManager projectionManager = (MediaProjectionManager) getSystemService(MEDIA_PROJECTION_SERVICE);
        mediaProjection = projectionManager.getMediaProjection(resultCode, data);
        if (mediaProjection != null) {
            MediaProjectionHolder.set(data, resultCode);
            MediaProjectionHolder.setProjection(mediaProjection);
        }

        initCapture();
        return START_STICKY;
    }

    private void initCapture() {
        Size screenSize = new Size(1920, 1080);
        int density = getResources().getDisplayMetrics().densityDpi;

        imageReader = ImageReader.newInstance(screenSize.getWidth(), screenSize.getHeight(),
                android.graphics.ImageFormat.YUV_420_888, 2);
        virtualDisplay = mediaProjection.createVirtualDisplay(
                "PChomeCapture",
                screenSize.getWidth(),
                screenSize.getHeight(),
                density,
                DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                imageReader.getSurface(),
                null,
                null
        );

        imageReader.setOnImageAvailableListener(this::onImageAvailable, null);
        encoder = new MediaCodecEncoder(screenSize.getWidth(), screenSize.getHeight());
        encoder.start();
    }

    private void onImageAvailable(ImageReader reader) {
        try (android.media.Image image = reader.acquireLatestImage()) {
            if (image == null) return;
            encoder.encodeFrame(image);
        } catch (Exception e) {
            Log.e(TAG, "Error capturing frame", e);
        }
    }

    private Notification buildNotification() {
        return new NotificationCompat.Builder(this, CHANNEL_ID)
                .setContentTitle(getString(R.string.screen_capture_notification))
                .setContentText("Screen capture is active")
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setOngoing(true)
                .build();
    }

    private void startForegroundWithType() {
        Notification notification = buildNotification();
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(NOTIFICATION_ID, notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION);
        } else {
            startForeground(NOTIFICATION_ID, notification);
        }
    }

    private void createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel channel = new NotificationChannel(
                    CHANNEL_ID,
                    getString(R.string.screen_capture_notification_channel),
                    NotificationManager.IMPORTANCE_LOW
            );
            NotificationManager manager = getSystemService(NotificationManager.class);
            if (manager != null) {
                manager.createNotificationChannel(channel);
            }
        }
    }

    @Override
    public void onDestroy() {
        super.onDestroy();
        if (virtualDisplay != null) {
            virtualDisplay.release();
        }
        if (mediaProjection != null) {
            mediaProjection.stop();
        }
        if (encoder != null) {
            encoder.stop();
        }
    }

    @Nullable
    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
