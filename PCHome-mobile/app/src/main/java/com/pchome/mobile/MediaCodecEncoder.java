package com.pchome.mobile;

import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaFormat;
import android.os.Build;
import android.util.Log;

import java.nio.ByteBuffer;

public class MediaCodecEncoder {
    private static final String TAG = "MediaCodecEncoder";
    private static final String MIME_TYPE = "video/avc";
    private static final int FRAME_RATE = 30;
    private static final int I_FRAME_INTERVAL = 1;

    private MediaCodec mediaCodec;
    private int width;
    private int height;
    private boolean isRunning;

    public MediaCodecEncoder(int width, int height) {
        this.width = width;
        this.height = height;
    }

    public void start() {
        try {
            mediaCodec = MediaCodec.createEncoderByType(MIME_TYPE);
            MediaFormat format = MediaFormat.createVideoFormat(MIME_TYPE, width, height);
            format.setInteger(MediaFormat.KEY_COLOR_FORMAT,
                    MediaCodecInfo.CodecCapabilities.COLOR_FormatYUV420Flexible);
            format.setInteger(MediaFormat.KEY_BIT_RATE, 4_000_000);
            format.setInteger(MediaFormat.KEY_FRAME_RATE, FRAME_RATE);
            format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, I_FRAME_INTERVAL);

            mediaCodec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE);
            mediaCodec.start();
            isRunning = true;

            Log.i(TAG, "Encoder started: " + width + "x" + height);
        } catch (Exception e) {
            Log.e(TAG, "Failed to start encoder", e);
        }
    }

    public void encodeFrame(android.media.Image image) {
        if (mediaCodec == null || !isRunning || image == null) return;

        try {
            int inputBufferIndex = mediaCodec.dequeueInputBuffer(10000);
            if (inputBufferIndex >= 0) {
                android.media.Image codecImage = mediaCodec.getInputImage(inputBufferIndex);
                if (codecImage != null) {
                    copyImage(image, codecImage);
                }
                int dataSize = imageSizeFromPlanes(image);
                mediaCodec.queueInputBuffer(inputBufferIndex, 0, dataSize,
                        System.nanoTime() / 1000, 0);
            }

            MediaCodec.BufferInfo bufferInfo = new MediaCodec.BufferInfo();
            int outputBufferIndex = mediaCodec.dequeueOutputBuffer(bufferInfo, 10000);

            while (outputBufferIndex >= 0) {
                ByteBuffer outputBuffer = mediaCodec.getOutputBuffer(outputBufferIndex);
                if (outputBuffer != null && (bufferInfo.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) == 0) {
                    byte[] encodedData = new byte[bufferInfo.size];
                    outputBuffer.get(encodedData);
                    Log.d(TAG, "Encoded frame: " + encodedData.length + " bytes");
                }
                mediaCodec.releaseOutputBuffer(outputBufferIndex, false);
                outputBufferIndex = mediaCodec.dequeueOutputBuffer(bufferInfo, 0);
            }
        } catch (Exception e) {
            Log.e(TAG, "Encode error", e);
        }
    }

    private void copyImage(android.media.Image src, android.media.Image dst) {
        for (int planeIndex = 0; planeIndex < src.getPlanes().length; planeIndex++) {
            android.media.Image.Plane srcPlane = src.getPlanes()[planeIndex];
            android.media.Image.Plane dstPlane = dst.getPlanes()[planeIndex];
            if (dstPlane == null || srcPlane == null) continue;

            ByteBuffer srcBuffer = srcPlane.getBuffer();
            ByteBuffer dstBuffer = dstPlane.getBuffer();
            int srcRowStride = srcPlane.getRowStride();
            int srcPixelStride = srcPlane.getPixelStride();
            int dstRowStride = dstPlane.getRowStride();
            int dstPixelStride = dstPlane.getPixelStride();

            int width = dstBuffer.remaining() / dstRowStride;
            if (width <= 0) width = srcBuffer.remaining() / Math.max(1, srcRowStride);

            for (int row = 0; row < srcPlane.getRowStride() / Math.max(1, srcPixelStride); row++) {
                if (row * dstRowStride >= dstBuffer.capacity()) break;
                for (int col = 0; col < width; col++) {
                    int srcPos = row * srcRowStride + col * srcPixelStride;
                    int dstPos = row * dstRowStride + col * dstPixelStride;
                    if (srcPos >= srcBuffer.capacity() || dstPos >= dstBuffer.capacity()) break;
                    dstBuffer.put(dstPos, srcBuffer.get(srcPos));
                }
            }
        }
    }

    private int imageSizeFromPlanes(android.media.Image image) {
        int size = 0;
        for (android.media.Image.Plane plane : image.getPlanes()) {
            size += plane.getBuffer().remaining();
        }
        return size;
    }

    public void stop() {
        isRunning = false;
        if (mediaCodec != null) {
            mediaCodec.stop();
            mediaCodec.release();
            mediaCodec = null;
        }
    }
}
