package com.PCHome.mobile.ui;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Rect;
import android.graphics.RectF;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.util.AttributeSet;
import android.view.MotionEvent;
import android.view.View;
import android.view.animation.DecelerateInterpolator;

import java.util.Collections;

/// Sağ kenarda beliren sonsuz kaydırma şeridi. Parmak yukarı/aşağı
/// sürüklendikçe yüksek çözünürlüklü tekerlek birimleri üretir; bırakınca
/// kısa bir momentum ile devam eder. Thumb konumu sınırlıdır ama kaydırma
/// miktarı sınırsızdır; bu yüzden "sonsuz" hissi verir.
public class ScrollRailView extends View {

    public interface Listener {
        void onStart();

        void onScroll(int hiResDy);

        void onFling(float unitsPerSecond);
    }

    /// 1 dp parmak hareketi ≈ 22 yüksek çözünürlüklü tekerlek birimi.
    private static final float UNITS_PER_DP = 22f;
    private static final float MAX_VELOCITY = 9000f;

    private final Paint trackPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint thumbPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint glowPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF trackRect = new RectF();
    private final RectF thumbRect = new RectF();
    private final float density;
    private final float thumbHeight;

    private Listener listener;
    private float thumbCenterY;
    private float lastTouchY;
    private float scrollAccum;
    private float velocity;
    private long lastMoveTime;
    private boolean dragging;
    private ValueAnimator settleAnimator;
    private int edgeDirection;
    private final Handler edgeHandler = new Handler(Looper.getMainLooper());
    private final Runnable edgeScroll = new Runnable() {
        @Override
        public void run() {
            if (!dragging || edgeDirection == 0) {
                return;
            }
            if (listener != null) {
                listener.onScroll(edgeDirection * 80);
            }
            edgeHandler.postDelayed(this, 50L);
        }
    };

    public ScrollRailView(Context context, AttributeSet attrs) {
        super(context, attrs);
        density = getResources().getDisplayMetrics().density;
        thumbHeight = 56f * density;

        trackPaint.setColor(0x33FFFFFF);
        trackPaint.setStyle(Paint.Style.FILL);
        thumbPaint.setColor(0xCCF4F4F5);
        thumbPaint.setStyle(Paint.Style.FILL);
        glowPaint.setColor(0x184F7BFF);
        glowPaint.setStyle(Paint.Style.FILL);
        setClickable(true);
        setFocusable(false);
    }

    public void setListener(Listener listener) {
        this.listener = listener;
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);
        float railWidth = 6f * density;
        float top = 8f * density;
        float bottom = h - 8f * density;
        if (bottom - top < thumbHeight * 1.5f) {
            bottom = top + thumbHeight * 1.5f;
        }
        trackRect.set(w / 2f - railWidth / 2f, top, w / 2f + railWidth / 2f, bottom);
        thumbCenterY = trackRect.centerY();
        updateThumbRect();
        excludeSystemGestures();
    }

    @Override
    protected void onVisibilityChanged(View changedView, int visibility) {
        super.onVisibilityChanged(changedView, visibility);
        if (visibility == VISIBLE) {
            excludeSystemGestures();
        } else {
            stopEdgeScroll();
        }
    }

    /** Sağ kenardaki geri jesti kaydırma şeridini çalmasın. */
    private void excludeSystemGestures() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q && getWidth() > 0) {
            setSystemGestureExclusionRects(Collections.singletonList(
                    new Rect(0, 0, getWidth(), getHeight())));
        }
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        float thumbWidth = 30f * density;
        trackRect.inset(0f, 0f);
        // Track.
        canvas.drawRoundRect(trackRect, trackRect.width() / 2f, trackRect.width() / 2f, trackPaint);
        // Thumb glow + thumb.
        RectF glow = new RectF(thumbRect);
        float g = 4f * density;
        glow.inset(-g, -g);
        canvas.drawRoundRect(glow, (thumbWidth + g) / 2f, (thumbWidth + g) / 2f, glowPaint);
        canvas.drawRoundRect(thumbRect, thumbWidth / 2f, thumbWidth / 2f, thumbPaint);
    }

    private void updateThumbRect() {
        float w = 30f * density;
        float cx = getWidth() / 2f;
        float top = clamp(thumbCenterY - thumbHeight / 2f,
                trackRect.top, trackRect.bottom - thumbHeight);
        thumbRect.set(cx - w / 2f, top, cx + w / 2f, top + thumbHeight);
        invalidate();
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                if (settleAnimator != null) {
                    settleAnimator.cancel();
                }
                if (listener != null) {
                    listener.onStart();
                }
                dragging = true;
                lastTouchY = event.getY();
                lastMoveTime = SystemClock.uptimeMillis();
                velocity = 0f;
                scrollAccum = 0f;
                stopEdgeScroll();
                thumbCenterY = clamp(event.getY(),
                        trackRect.top + thumbHeight / 2f,
                        trackRect.bottom - thumbHeight / 2f);
                updateThumbRect();
                if (getParent() != null) {
                    getParent().requestDisallowInterceptTouchEvent(true);
                }
                return true;

            case MotionEvent.ACTION_MOVE:
                if (!dragging) {
                    return true;
                }
                float y = event.getY();
                float dy = y - lastTouchY;
                lastTouchY = y;
                if (dy != 0f) {
                    long now = SystemClock.uptimeMillis();
                    long dtMs = now - lastMoveTime;
                    lastMoveTime = now;
                    float dtSec = dtMs / 1000f;
                    float speedDp = dtSec > 0f ? Math.abs(dy) / density / dtSec : 0f;
                    float boost = 1f + Math.min(speedDp / 1200f, 1.2f);
                    float units = -dy / density * UNITS_PER_DP * boost;
                    scrollAccum += units;
                    int send = (int) scrollAccum;
                    scrollAccum -= send;
                    if (send != 0 && listener != null) {
                        listener.onScroll(send);
                    }
                    if (dtMs > 0 && dtMs < 120) {
                        float instant = units / dtSec;
                        velocity = velocity * 0.55f + instant * 0.45f;
                        velocity = clamp(velocity, -MAX_VELOCITY, MAX_VELOCITY);
                    } else if (dtMs >= 120) {
                        velocity = 0f;
                    }
                }
                thumbCenterY = clamp(y,
                        trackRect.top + thumbHeight / 2f,
                        trackRect.bottom - thumbHeight / 2f);
                updateThumbRect();
                if (y <= trackRect.top + thumbHeight / 2f + 1f) {
                    startEdgeScroll(1);
                } else if (y >= trackRect.bottom - thumbHeight / 2f - 1f) {
                    startEdgeScroll(-1);
                } else {
                    stopEdgeScroll();
                }
                return true;

            case MotionEvent.ACTION_UP:
            case MotionEvent.ACTION_CANCEL:
                if (!dragging) {
                    return true;
                }
                dragging = false;
                stopEdgeScroll();
                if (event.getActionMasked() == MotionEvent.ACTION_UP
                        && listener != null && Math.abs(velocity) > 40f) {
                    listener.onFling(velocity);
                }
                settleThumbToCenter();
                return true;

            default:
                return super.onTouchEvent(event);
        }
    }

    private void startEdgeScroll(int direction) {
        if (edgeDirection == direction) {
            return;
        }
        edgeDirection = direction;
        edgeHandler.removeCallbacks(edgeScroll);
        if (direction != 0) {
            edgeHandler.post(edgeScroll);
        }
    }

    private void stopEdgeScroll() {
        edgeDirection = 0;
        edgeHandler.removeCallbacks(edgeScroll);
    }

    private void settleThumbToCenter() {
        if (settleAnimator != null) {
            settleAnimator.cancel();
        }
        settleAnimator = ValueAnimator.ofFloat(thumbCenterY, trackRect.centerY());
        settleAnimator.setDuration(220L);
        settleAnimator.setInterpolator(new DecelerateInterpolator());
        settleAnimator.addUpdateListener(a -> {
            thumbCenterY = (float) a.getAnimatedValue();
            updateThumbRect();
        });
        settleAnimator.start();
    }

    private static float clamp(float v, float min, float max) {
        return Math.max(min, Math.min(max, v));
    }
}
