package com.pchome.mobile.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.AttributeSet;
import android.view.View;

import androidx.annotation.Nullable;

/// Neon cyan crosshair/halo drawn at the current touch point to give direct
/// visual feedback of where the PC cursor is being moved.
public class ReticleView extends View {
    private final Paint paint = new Paint();
    private float x = 0f;
    private float y = 0f;
    private boolean visible = false;

    public ReticleView(Context context, @Nullable AttributeSet attrs) {
        super(context, attrs);
        paint.setColor(0xFF00F4FF); // #00F4FF
        paint.setStyle(Paint.Style.STROKE);
        paint.setStrokeWidth(3f);
        paint.setAntiAlias(true);
        setWillNotDraw(false);
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
        if (!visible) {
            return;
        }
        final float r = 22f;
        canvas.drawCircle(x, y, r, paint);
        canvas.drawCircle(x, y, r * 0.4f, paint);
        canvas.drawLine(x - r - 10, y, x - r + 6, y, paint);
        canvas.drawLine(x + r - 6, y, x + r + 10, y, paint);
        canvas.drawLine(x, y - r - 10, x, y - r + 6, paint);
        canvas.drawLine(x, y + r - 6, x, y + r + 10, paint);
    }
}
