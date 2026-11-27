#!/usr/bin/env python3
"""Frame the real PCHome screenshots used by the README.

The raw files must be captured from the running app:

* an Android screenshot in landscape with the phone connected to a desktop
* a desktop screenshot of the PCHome window while that phone is connected

This script only adds the device/window frames, shadows and rounded corners.
It never draws or invents app content. All visible data in the raw captures is
fictional demo data.

Run from the repository root:

    python3 scripts/make_screenshots.py \
        --phone /tmp/phone.png \
        --desktop /tmp/desktop.png

Outputs:
    docs/img/pixel-10-pro.png
    docs/img/desktop.png
"""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

OUT = Path("docs/img")
NOTO = "/usr/share/fonts/google-noto/NotoSans-Regular.ttf"


def rounded_mask(size: tuple[int, int], radius: int) -> Image.Image:
    mask = Image.new("L", size, 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, size[0] - 1, size[1] - 1), radius=radius, fill=255
    )
    return mask


def drop_shadow(
    canvas_size: tuple[int, int],
    box: tuple[int, int, int, int],
    radius: int,
    *,
    blur: int = 34,
    opacity: int = 95,
    offset: tuple[int, int] = (0, 26),
) -> Image.Image:
    shadow = Image.new("RGBA", canvas_size, (0, 0, 0, 0))
    x0, y0, x1, y1 = box
    ImageDraw.Draw(shadow).rounded_rectangle(
        (x0 + offset[0], y0 + offset[1], x1 + offset[0], y1 + offset[1]),
        radius=radius,
        fill=(0, 0, 0, opacity),
    )
    return shadow.filter(ImageFilter.GaussianBlur(blur))


def phone_frame(raw: Path) -> Image.Image:
    """Wrap the real 2400x1080 Android capture in a Pixel-style body."""
    screen = Image.open(raw).convert("RGB")
    if screen.size != (2400, 1080):
        screen = screen.resize((2400, 1080), Image.Resampling.LANCZOS)

    margin = 60
    body = (60, 60, 2580, 1260)
    canvas_size = (2640, 1320)
    canvas = Image.new("RGBA", canvas_size, (0, 0, 0, 0))
    canvas.alpha_composite(drop_shadow(canvas_size, body, 118, blur=38, opacity=105))

    draw = ImageDraw.Draw(canvas)
    # Phone body and a thin edge highlight.
    draw.rounded_rectangle(body, radius=118, fill=(19, 19, 22, 255))
    draw.rounded_rectangle(body, radius=118, outline=(58, 58, 64, 255), width=2)
    # Black bezel around the display.
    draw.rounded_rectangle((body[0] + 49, body[1] + 49, body[2] - 49, body[3] - 49),
                           radius=79, fill=(4, 4, 6, 255))

    sx, sy = 120, 120
    screen_rgba = screen.convert("RGBA")
    screen_rgba.putalpha(rounded_mask(screen.size, 62))
    canvas.alpha_composite(screen_rgba, (sx, sy))

    # Centred punch-hole camera on the phone's top edge (left in landscape).
    cx, cy = sx + 27, sy + screen.height // 2
    draw.ellipse((cx - 14, cy - 14, cx + 14, cy + 14), fill=(2, 2, 3, 255),
                 outline=(44, 44, 49, 255), width=2)
    draw.ellipse((cx - 7, cy - 7, cx + 7, cy + 7), fill=(8, 14, 22, 255))
    draw.ellipse((cx - 4, cy - 5, cx + 1, cy), fill=(70, 110, 150, 140))

    # Power button on the top edge.
    draw.rounded_rectangle((1510, 54, 1595, 65), radius=5, fill=(44, 44, 49, 255))
    return canvas


def desktop_frame(raw: Path) -> Image.Image:
    """Crop the KWin title bar and draw a macOS-style window around the app."""
    shot = Image.open(raw).convert("RGB")
    # The native title bar is 37 px tall in the captured window; the app draws
    # its own header immediately below it.
    if shot.height > 900:
        shot = shot.crop((0, 37, shot.width, shot.height))
    content_w, content_h = shot.size

    title_h = 38
    win_w, win_h = content_w, content_h + title_h
    win = Image.new("RGBA", (win_w, win_h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(win)
    draw.rounded_rectangle((0, 0, win_w - 1, win_h - 1), radius=14, fill=(15, 17, 21, 255))
    draw.rounded_rectangle((0, 0, win_w - 1, title_h - 1), radius=14, fill=(28, 29, 34, 255))
    draw.rectangle((0, title_h - 14, win_w - 1, title_h - 1), fill=(28, 29, 34, 255))
    draw.line((0, title_h - 1, win_w - 1, title_h - 1), fill=(43, 45, 52, 255))
    for i, color in enumerate(((255, 95, 87), (254, 188, 46), (40, 200, 64))):
        x = 22 + i * 24
        draw.ellipse((x - 7, title_h // 2 - 7, x + 7, title_h // 2 + 7), fill=color)
    font = ImageFont.truetype(NOTO, 14)
    title = "PCHome — Remote Desktop"
    tw = draw.textlength(title, font=font)
    draw.text(((win_w - tw) / 2, 11), title, font=font, fill=(161, 161, 170))

    win.alpha_composite(shot.convert("RGBA"), (0, title_h))
    win.putalpha(rounded_mask((win_w, win_h), 14))
    draw = ImageDraw.Draw(win)
    draw.rounded_rectangle((0, 0, win_w - 1, win_h - 1), radius=14,
                           outline=(58, 60, 68, 255), width=1)

    pad = 70
    canvas_size = (win_w + pad * 2, win_h + pad * 2)
    canvas = Image.new("RGBA", canvas_size, (0, 0, 0, 0))
    canvas.alpha_composite(drop_shadow(canvas_size, (pad, pad, pad + win_w, pad + win_h),
                                      14, blur=36, opacity=105))
    canvas.alpha_composite(win, (pad, pad))
    return canvas


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phone", type=Path, required=True,
                        help="raw Android screenshot (landscape)")
    parser.add_argument("--desktop", type=Path, required=True,
                        help="raw PCHome desktop window screenshot")
    args = parser.parse_args()

    OUT.mkdir(parents=True, exist_ok=True)
    phone = phone_frame(args.phone)
    desktop = desktop_frame(args.desktop)
    phone.save(OUT / "pixel-10-pro.png", optimize=True)
    desktop.save(OUT / "desktop.png", optimize=True)
    print(f"Wrote {OUT / 'pixel-10-pro.png'} {phone.size}")
    print(f"Wrote {OUT / 'desktop.png'} {desktop.size}")


if __name__ == "__main__":
    main()
