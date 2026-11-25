package com.PCHome.mobile.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RectF;
import android.util.AttributeSet;
import android.view.View;

import androidx.annotation.Nullable;

/// Dokunma katmanı ve ince video çerçevesi. Fare imleci artık videoda
/// masaüstünün kendi imlecidir; burada büyük bir yerel ok çizilmez.
public class ReticleView extends View {
    private final Paint borderPaint = new Paint();
    private final Paint glowPaint = new Paint();
    private final RectF videoRect = new RectF();
    private final float density;
    private float videoLeft = -1f;
    private float videoTop = -1f;
    private float videoRight = -1f;
    private float videoBottom = -1f;

    public ReticleView(Context context, @Nullable AttributeSet attrs) {
        super(context, attrs);
        density = getResources().getDisplayMetrics().density;
        borderPaint.setColor(0x24FFFFFF);
        borderPaint.setStyle(Paint.Style.STROKE);
        borderPaint.setStrokeWidth(1.1f * density);
        borderPaint.setAntiAlias(true);
        glowPaint.setColor(0x08FFFFFF);
        glowPaint.setStyle(Paint.Style.STROKE);
        glowPaint.setStrokeWidth(5f * density);
        glowPaint.setAntiAlias(true);
        setWillNotDraw(false);
    }

    /** Video görüntüsünün ekrandaki dikdörtgeni; ince çerçeve olarak çizilir. */
    public void setVideoRect(float left, float top, float right, float bottom) {
        videoLeft = left;
        videoTop = top;
        videoRight = right;
        videoBottom = bottom;
        invalidate();
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        if (videoRight > videoLeft && videoBottom > videoTop) {
            videoRect.set(videoLeft, videoTop, videoRight, videoBottom);
            // Video alanının köşeleri kırpılmasın: yuvarlatma yok.
            canvas.drawRect(videoRect, glowPaint);
            canvas.drawRect(videoRect, borderPaint);
        }
    }
}
