package com.pchome.mobile;

import android.content.Context;
import android.os.Build;

import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;

import org.junit.Before;
import org.junit.Test;
import org.junit.runner.RunWith;

import java.util.ArrayList;
import java.util.List;

@RunWith(AndroidJUnit4.class)
public class MediaCodecPerformanceTest {
    private Context context;
    private MediaCodecEncoder encoder;

    @Before
    public void setUp() {
        context = InstrumentationRegistry.getInstrumentation().getTargetContext();
    }

    @Test
    public void testEncoderInitialization() {
        encoder = new MediaCodecEncoder(1920, 1080);
        encoder.start();
        encoder.stop();
    }

    @Test
    public void testEncoderFrameThroughput() {
        encoder = new MediaCodecEncoder(1920, 1080);
        encoder.start();

        byte[] frame = new byte[1920 * 1080 * 4];
        long startTime = System.currentTimeMillis();
        int frameCount = 100;

        for (int i = 0; i < frameCount; i++) {
            encoder.encodeFrame(frame);
        }

        long duration = System.currentTimeMillis() - startTime;
        double fps = frameCount / (duration / 1000.0);
        encoder.stop();

        assert fps > 0;
    }
}
