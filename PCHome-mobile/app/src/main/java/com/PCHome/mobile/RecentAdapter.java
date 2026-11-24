package com.PCHome.mobile;

import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
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
        String host = pc.serverHost();
        h.subtitle.setText("ID " + pc.pcId + (host.isEmpty() ? "" : " • " + host));
        // Şifreyi kartta açık gösterme: omuz üstünden okunmasın.
        h.password.setText(pc.rememberPassword && !pc.password.isEmpty()
                ? h.itemView.getContext().getString(R.string.recent_password_saved)
                : h.itemView.getContext().getString(R.string.recent_password_missing));
        String ago = relativeTime(pc.lastSeen);
        h.lastSeen.setText(pc.resolution.isEmpty()
                ? ago : pc.resolution + "  •  " + ago);

        File thumb = new File(pc.thumbnail);
        if (!pc.thumbnail.isEmpty() && thumb.exists()) {
            Bitmap bmp = BitmapFactory.decodeFile(thumb.getAbsolutePath());
            if (bmp != null) {
                h.thumb.setImageBitmap(bmp);
                h.thumb.setScaleType(ImageView.ScaleType.CENTER_CROP);
                h.thumb.setColorFilter(null);
                h.thumb.setPadding(0, 0, 0, 0);
            } else {
                showPlaceholder(h.thumb);
            }
        } else {
            showPlaceholder(h.thumb);
        }

        h.card.setOnClickListener(v -> {
            if (listener != null) listener.onOpen(pc);
        });
        h.card.setOnLongClickListener(v -> {
            if (listener != null) listener.onDelete(pc);
            return true;
        });
        h.delete.setOnClickListener(v -> {
            if (listener != null) listener.onDelete(pc);
        });
    }

    /** Türkçe, kısa "son görülme" metni (sistem dili İngilizce olsa da). */
    private String relativeTime(long lastSeen) {
        long diff = Math.max(0L, System.currentTimeMillis() - lastSeen);
        if (diff < 60_000L) return "az önce";
        if (diff < 3_600_000L) return (diff / 60_000L) + " dk önce";
        if (diff < 86_400_000L) return (diff / 3_600_000L) + " sa önce";
        if (diff < 7L * 86_400_000L) return (diff / 86_400_000L) + " gün önce";
        return new java.text.SimpleDateFormat("d MMM", java.util.Locale.getDefault())
                .format(new java.util.Date(lastSeen));
    }

    /** Küçük görüntü yoksa jenerik galeri ikonu yerine monitör glifi. */
    private void showPlaceholder(ImageView view) {
        view.setImageResource(R.drawable.ic_monitor);
        view.setScaleType(ImageView.ScaleType.CENTER);
        int pad = Math.round(28 * view.getResources().getDisplayMetrics().density);
        view.setPadding(pad, pad, pad, pad);
        view.setColorFilter(view.getResources().getColor(R.color.PCHome_dim, null));
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
        final MaterialTextView password;
        final MaterialTextView lastSeen;
        final View delete;

        Holder(@NonNull View itemView) {
            super(itemView);
            card = (MaterialCardView) itemView;
            thumb = itemView.findViewById(R.id.recent_thumb);
            title = itemView.findViewById(R.id.recent_title);
            subtitle = itemView.findViewById(R.id.recent_subtitle);
            password = itemView.findViewById(R.id.recent_password);
            lastSeen = itemView.findViewById(R.id.recent_last_seen);
            delete = itemView.findViewById(R.id.recent_delete);
        }
    }
}
