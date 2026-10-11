package com.pchome.mobile;

import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.text.format.DateUtils;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ImageView;

import androidx.annotation.NonNull;
import androidx.recyclerview.widget.RecyclerView;

import com.google.android.material.card.MaterialCardView;
import com.google.android.material.textview.MaterialTextView;

import java.io.File;
import java.util.ArrayList;
import java.util.List;

/**
 * "Son Girilenler" kare kart ızgarası: üstte PC'nin son görüntüsü
 * (kare kutucuk), altta isim + ID/IP/MAC + son görülme.
 */
public class RecentAdapter extends RecyclerView.Adapter<RecentAdapter.Holder> {

    public interface Listener {
        void onOpen(RecentPc pc);
        void onDelete(RecentPc pc);
    }

    private final List<RecentPc> items = new ArrayList<>();
    private final Listener listener;

    public RecentAdapter(Listener listener) {
        this.listener = listener;
    }

    public void setItems(List<RecentPc> next) {
        items.clear();
        if (next != null) {
            items.addAll(next);
        }
        notifyDataSetChanged();
    }

    @NonNull
    @Override
    public Holder onCreateViewHolder(@NonNull ViewGroup parent, int viewType) {
        View v = LayoutInflater.from(parent.getContext())
                .inflate(R.layout.item_recent_pc, parent, false);
        return new Holder(v);
    }

    @Override
    public void onBindViewHolder(@NonNull Holder h, int position) {
        RecentPc pc = items.get(position);
        String title = pc.pcName.isEmpty() ? ("PC " + pc.pcId) : pc.pcName;
        h.title.setText(title);
        h.subtitle.setText(pc.subtitle());
        CharSequence ago = DateUtils.getRelativeTimeSpanString(
                pc.lastSeen, System.currentTimeMillis(), DateUtils.MINUTE_IN_MILLIS);
        h.lastSeen.setText(pc.resolution.isEmpty() ? ago : (pc.resolution + "  •  " + ago));

        File thumb = new File(pc.thumbnail);
        if (!pc.thumbnail.isEmpty() && thumb.exists()) {
            Bitmap bmp = BitmapFactory.decodeFile(thumb.getAbsolutePath());
            if (bmp != null) {
                h.thumb.setImageBitmap(bmp);
                h.thumb.setScaleType(ImageView.ScaleType.CENTER_CROP);
            } else {
                h.thumb.setImageResource(android.R.drawable.ic_menu_gallery);
            }
        } else {
            h.thumb.setImageResource(android.R.drawable.ic_menu_gallery);
        }

        h.card.setOnClickListener(v -> {
            if (listener != null) listener.onOpen(pc);
        });
        h.card.setOnLongClickListener(v -> {
            if (listener != null) listener.onDelete(pc);
            return true;
        });
    }

    @Override
    public int getItemCount() {
        return items.size();
    }

    static class Holder extends RecyclerView.ViewHolder {
        final MaterialCardView card;
        final ImageView thumb;
        final MaterialTextView title;
        final MaterialTextView subtitle;
        final MaterialTextView lastSeen;

        Holder(@NonNull View itemView) {
            super(itemView);
            card = (MaterialCardView) itemView;
            thumb = itemView.findViewById(R.id.recent_thumb);
            title = itemView.findViewById(R.id.recent_title);
            subtitle = itemView.findViewById(R.id.recent_subtitle);
            lastSeen = itemView.findViewById(R.id.recent_last_seen);
        }
    }
}
