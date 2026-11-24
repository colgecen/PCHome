package com.PCHome.mobile.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Path;
import android.util.AttributeSet;
import android.view.View;

import androidx.annotation.Nullable;

/// Neon çerçeve + uzaktan fare imleci. İmleç, son dokunulan PC
/// koordinatında kalır; böylece kullanıcı dokunmayı bıraktıktan sonra da
/// nereyi kontrol ettiğini görür.
public class ReticleView extends View {
    private final Paint borderPaint = new Paint();
    private final Paint cursorFill = new Paint();
    private final Paint cursorOutline = new Paint();
    private final Path cursorPath = new Path();
    private float x = 0f;
    private float y = 0f;
    private boolean visible = false;
    private float videoLeft = -1f;
    private float videoTop = -1f;
    private float videoRight = -1f;
    private float videoBottom = -1f;

    public ReticleView(Context context, @Nullable AttributeSet attrs) {
        super(context, attrs);
        borderPaint.setColor(0x5500F4FF);
        borderPaint.setStyle(Paint.Style.STROKE);
        borderPaint.setStrokeWidth(2f);
        borderPaint.setAntiAlias(true);
        cursorFill.setColor(0xFFFFFFFF);
        cursorFill.setStyle(Paint.Style.FILL);
        cursorFill.setAntiAlias(true);
        cursorOutline.setColor(0xFF090C10);
        cursorOutline.setStyle(Paint.Style.STROKE);
        cursorOutline.setStrokeWidth(2.5f);
        cursorOutline.setAntiAlias(true);
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

    public void setPosition(float x, float y) {
        this.x = x;
        this.y = y;
        visible = true;
        invalidate();
    }

    public void setVisible(boolean v) {
        visible = v;
        invalidate();
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        if (videoRight > videoLeft && videoBottom > videoTop) {
            canvas.drawRect(videoLeft, videoTop, videoRight, videoBottom, borderPaint);
        }
        if (!visible) {
            return;
        }
        // Standart beyaz fare oku; ucu (x,y) PC imleç konumudur.
        final float s = 26f;
        cursorPath.reset();
        cursorPath.moveTo(x, y);
        cursorPath.lineTo(x, y + s);
        cursorPath.lineTo(x + s * 0.30f, y + s * 0.72f);
        cursorPath.lineTo(x + s * 0.52f, y + s * 0.98f);
        cursorPath.lineTo(x + s * 0.68f, y + s * 0.90f);
        cursorPath.lineTo(x + s * 0.47f, y + s * 0.65f);
        cursorPath.lineTo(x + s * 0.82f, y + s * 0.63f);
        cursorPath.close();
        canvas.drawPath(cursorPath, cursorFill);
        canvas.drawPath(cursorPath, cursorOutline);
    }
}
