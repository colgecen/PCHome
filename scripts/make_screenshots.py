#!/usr/bin/env python3
"""Generate the two PCHome README screenshots.

The drawings use the app's real palette and UI structure. All names, IDs,
addresses, devices, files and events below are representative demo data only;
no data from a real machine or person is used.

Run from the repository root:

    python3 scripts/make_screenshots.py

Outputs:
    docs/img/pixel-10-pro.svg
    docs/img/desktop.svg
"""

from __future__ import annotations

import html
from pathlib import Path

OUT = Path("docs/img")

FONT = "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
MONO = "'JetBrains Mono', 'SF Mono', 'Cascadia Code', monospace"

# PCHome neutral palette (desktop and mobile share the same family).
BG = "#09090B"
PANEL = "#121214"
SURFACE = "#18181B"
SURFACE_HIGH = "#1F1F23"
SURFACE_HOVER = "#2A2A30"
TEXT = "#F4F4F5"
TEXT_2 = "#E4E4E7"
MUTED = "#A1A1AA"
DIM = "#71717A"
BORDER = "#27272A"
BORDER_BRIGHT = "#3F3F46"
ACCENT = "#818CF8"
BLUE = "#6366F1"
GREEN = "#4ADE80"
RED = "#F87171"
AMBER = "#FBBF24"
BLACK = "#050608"


def esc(text: str) -> str:
    return html.escape(text, quote=True)


def text(
    x: float,
    y: float,
    value: str,
    *,
    size: float = 14,
    fill: str = TEXT,
    weight: int = 400,
    family: str = FONT,
    anchor: str = "start",
    opacity: float = 1.0,
    letter_spacing: float = 0.0,
) -> str:
    return (
        f'<text x="{x}" y="{y}" font-family="{family}" font-size="{size}" '
        f'font-weight="{weight}" fill="{fill}" text-anchor="{anchor}" '
        f'opacity="{opacity}" letter-spacing="{letter_spacing}">{esc(value)}</text>'
    )


def rect(
    x: float,
    y: float,
    w: float,
    h: float,
    *,
    fill: str,
    rx: float = 0,
    stroke: str | None = None,
    stroke_width: float = 1,
    opacity: float = 1.0,
) -> str:
    extra = f' stroke="{stroke}" stroke-width="{stroke_width}"' if stroke else ""
    return (
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" '
        f'fill="{fill}" opacity="{opacity}"{extra}/>'
    )


def circle(cx: float, cy: float, r: float, fill: str, opacity: float = 1.0) -> str:
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}" opacity="{opacity}"/>'


def line(x1: float, y1: float, x2: float, y2: float, stroke: str, width: float = 1) -> str:
    return (
        f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" '
        f'stroke="{stroke}" stroke-width="{width}"/>'
    )


def group(content: str, *, clip: str | None = None, transform: str | None = None) -> str:
    attrs = ""
    if clip:
        attrs += f' clip-path="url(#{clip})"'
    if transform:
        attrs += f' transform="{transform}"'
    return f"<g{attrs}>{content}</g>"


def defs() -> str:
    return (
        "<defs>"
        '<clipPath id="phone-screen"><rect x="104" y="144" width="1392" height="612" rx="38"/></clipPath>'
        '<clipPath id="mac-window"><rect x="80" y="90" width="1440" height="820" rx="12"/></clipPath>'
        '<linearGradient id="phone-bevel" x1="0" y1="0" x2="0" y2="1">'
        '<stop offset="0" stop-color="#303036"/><stop offset="1" stop-color="#0B0B0E"/>'
        "</linearGradient>"
        '<linearGradient id="hud-scrim" x1="0" y1="0" x2="0" y2="1">'
        '<stop offset="0" stop-color="#09090B" stop-opacity="0.98"/>'
        '<stop offset="1" stop-color="#09090B" stop-opacity="0"/>'
        "</linearGradient>"
        '<linearGradient id="stage-glow" x1="0" y1="0" x2="1" y2="1">'
        '<stop offset="0" stop-color="#6E9BFF" stop-opacity="0.12"/>'
        '<stop offset="1" stop-color="#4ADE80" stop-opacity="0.05"/>'
        "</linearGradient>"
        "</defs>"
    )


def fake_desktop(x: float, y: float, w: float, h: float, scale: float = 1.0) -> str:
    """A neutral, fake editor desktop shown inside the remote stream."""
    s = scale
    out = [rect(x, y, w, h, fill="#0B0D12")]
    out.append(rect(x, y, w, 34 * s, fill="#141820"))
    out.append(circle(x + 16 * s, y + 17 * s, 4 * s, "#FF5F57"))
    out.append(circle(x + 30 * s, y + 17 * s, 4 * s, "#FEBC2E"))
    out.append(circle(x + 44 * s, y + 17 * s, 4 * s, "#28C840"))
    out.append(text(x + 66 * s, y + 22 * s, "studio-workspace  /  demo-app", size=10 * s,
                    fill=MUTED, family=MONO))
    out.append(text(x + w - 16 * s, y + 22 * s, "main.rs", size=10 * s, fill=DIM,
                    family=MONO, anchor="end"))

    sidebar_w = 176 * s
    out.append(rect(x, y + 34 * s, sidebar_w, h - 34 * s, fill="#10131A"))
    out.append(text(x + 18 * s, y + 60 * s, "EXPLORER", size=8.5 * s, fill=DIM,
                    weight=700, letter_spacing=1.2))
    out.append(text(x + 18 * s, y + 88 * s, "▾ demo-app", size=10 * s, fill=TEXT_2, family=MONO))
    for i, label in enumerate(["▾ src", "  main.rs", "  ui.rs", "  protocol.rs", "▾ docs", "  README.md"]):
        out.append(text(x + 30 * s, y + (108 + i * 20) * s, label, size=9.5 * s,
                        fill=MUTED if i else TEXT_2, family=MONO))

    code_x = x + sidebar_w + 28 * s
    code_y = y + 72 * s
    out.append(text(code_x, code_y, "1  use std::sync::Arc;", size=10 * s, fill="#7DD3FC", family=MONO))
    out.append(text(code_x, code_y + 20 * s, "2  use tokio::sync::mpsc;", size=10 * s, fill="#7DD3FC", family=MONO))
    out.append(text(code_x, code_y + 40 * s, "3", size=10 * s, fill=DIM, family=MONO))
    out.append(text(code_x, code_y + 60 * s, "4  pub struct Session {", size=10 * s, fill="#C4B5FD", family=MONO))
    out.append(text(code_x, code_y + 80 * s, "5      id: u64,", size=10 * s, fill=TEXT_2, family=MONO))
    out.append(text(code_x, code_y + 100 * s, "6      active: bool,", size=10 * s, fill=TEXT_2, family=MONO))
    out.append(text(code_x, code_y + 120 * s, "7  }", size=10 * s, fill="#C4B5FD", family=MONO))
    out.append(text(code_x, code_y + 140 * s, "8", size=10 * s, fill=DIM, family=MONO))
    out.append(text(code_x, code_y + 160 * s, "9  fn connect() -> Result<()> {", size=10 * s, fill="#C4B5FD", family=MONO))
    out.append(text(code_x, code_y + 180 * s, "10     let session = Session::new();", size=10 * s, fill=TEXT_2, family=MONO))
    out.append(text(code_x, code_y + 200 * s, "11     session.start()", size=10 * s, fill=TEXT_2, family=MONO))
    out.append(text(code_x, code_y + 220 * s, "12 }", size=10 * s, fill="#C4B5FD", family=MONO))

    # Fake cursor, clearly inside the remote desktop.
    cx, cy = x + w * 0.56, y + h * 0.55
    out.append(
        f'<path d="M{cx},{cy} L{cx},{cy + 18 * s} L{cx + 5 * s},{cy + 13 * s} '
        f'L{cx + 9 * s},{cy + 21 * s} L{cx + 12 * s},{cy + 19 * s} '
        f'L{cx + 8 * s},{cy + 11 * s} L{cx + 15 * s},{cy + 11 * s} Z" '
        f'fill="#FFFFFF" stroke="#05070E" stroke-width="{1.4 * s}"/>'
    )
    out.append(rect(x, y + h - 24 * s, w, 24 * s, fill="#11151D"))
    out.append(text(x + 14 * s, y + h - 8 * s, "Ln 42, Col 8", size=8.5 * s, fill=DIM, family=MONO))
    out.append(text(x + w - 14 * s, y + h - 8 * s, "Rust  •  UTF-8  •  LF", size=8.5 * s,
                    fill=DIM, family=MONO, anchor="end"))
    return "".join(out)


def phone_screenshot() -> str:
    out = [defs()]
    # Phone body, screen and camera.
    out.append(rect(70, 110, 1460, 680, fill="url(#phone-bevel)", rx="74"))
    out.append(rect(78, 118, 1444, 664, fill="#050507", rx="68"))
    out.append(rect(104, 144, 1392, 612, fill=BG, rx="38"))
    out.append(circle(800, 164, 7, "#050507"))
    out.append(circle(800, 164, 3.2, "#1F2937"))

    content = []
    # Top HUD.
    content.append(rect(104, 144, 1392, 82, fill="#0D0F14", opacity="0.97"))
    content.append(rect(104, 226, 1392, 1, fill="#23252C"))
    content.append(circle(132, 178, 5, GREEN))
    content.append(text(148, 174, "Studio-PC", size=15, weight=700, fill=TEXT))
    content.append(text(148, 195, "1920 × 1080  •  Trackpad", size=10.5, fill=MUTED))
    # Eye button.
    content.append(rect(1414, 160, 42, 42, fill="#1F1F23", rx="21"))
    content.append(
        '<path d="M1423 181c4-7 10-11 17-11s13 4 17 11c-4 7-10 11-17 11s-13-4-17-11z" '
        'fill="none" stroke="#D4D4D8" stroke-width="1.5"/>'
    )
    content.append(circle(1440, 181, 3.2, "#D4D4D8"))

    # Remote desktop stage.
    content.append(fake_desktop(104, 226, 1392, 374, 1.0))

    # Bottom dock.
    content.append(rect(104, 600, 1392, 156, fill="#101116", opacity="0.98"))
    content.append(rect(104, 600, 1392, 1, fill="#26272E"))

    # Dock buttons row.
    labels = ["Disconnect", "Zoom", "Trackpad", "Keyboard"]
    bx = 124
    bw = 210
    for i, label in enumerate(labels):
        x = bx + i * (bw + 10)
        bg = "#2A1518" if i == 0 else SURFACE_HIGH
        fg = RED if i == 0 else TEXT_2
        content.append(rect(x, 620, bw, 46, fill=bg, rx="10"))
        icon_x = x + 22
        if i == 0:
            content.append(circle(icon_x, 643, 7, "none"))
            content.append(line(icon_x - 5, 638, icon_x + 5, 648, fg, 1.8))
            content.append(line(icon_x + 5, 638, icon_x - 5, 648, fg, 1.8))
        elif i == 1:
            content.append(circle(icon_x, 643, 7, "none"))
            content.append(line(icon_x - 5, 643, icon_x + 5, 643, fg, 1.6))
            content.append(line(icon_x + 2, 639, icon_x + 5, 643, fg, 1.6))
            content.append(line(icon_x + 2, 647, icon_x + 5, 643, fg, 1.6))
        elif i == 2:
            content.append(
                f'<path d="M{icon_x - 6} 644 Q{icon_x - 6} 635 {icon_x} 635 '
                f'Q{icon_x + 6} 635 {icon_x + 6} 644" fill="none" stroke="{fg}" stroke-width="1.6"/>'
            )
            content.append(circle(icon_x, 644, 2, fg))
        else:
            for k in range(3):
                content.append(rect(icon_x - 7 + (k % 3) * 5, 641 + (k // 3) * 3, 3, 2.5, fill=fg, rx="0.5"))
        content.append(text(x + 40, 646, label, size=11, fill=fg, weight=600))

    # Rotate icon button (the app has no text label on this button).
    rx = 124 + 4 * (bw + 10)
    content.append(rect(rx, 620, 92, 46, fill=SURFACE_HIGH, rx="10"))
    cxr = rx + 46
    content.append(
        f'<path d="M{cxr - 10} 644 a11 11 0 0 1 18-8" fill="none" stroke="{TEXT_2}" stroke-width="2"/>'
    )
    content.append(f'<path d="M{cxr + 7} 631 l5 1 l-2 5z" fill="{TEXT_2}"/>')
    content.append(
        f'<rect x="{cxr - 8}" y="636" width="17" height="11" rx="2" '
        f'transform="rotate(45 {cxr} 642)" fill="none" stroke="{TEXT_2}" stroke-width="1.6"/>'
    )

    # Keyboard hotkey row.
    key_labels = ["CTRL+C", "Left", "Scroll", "Right", "CTRL+V"]
    kx = 124
    kw = 164
    for i, label in enumerate(key_labels):
        x = kx + i * (kw + 10)
        content.append(rect(x, 678, kw, 36, fill="#0D0E12", rx="8"))
        content.append(text(x + kw / 2, 700, label, size=9.5, fill=MUTED,
                            anchor="middle", weight=600))

    # Side click buttons, in the fullscreen style the latest app shows below
    # the video in portrait; in landscape they sit as a narrow stack at the
    # left edge. The screenshot shows the normal dock instead, so no overlay.
    phone = group("".join(content), clip="phone-screen")
    out.append(phone)
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="900" '
        'viewBox="0 0 1600 900" role="img" '
        'aria-label="PCHome Android app connected in landscape">'
        + "".join(out)
        + "</svg>"
    )


def mac_window() -> str:
    out = [defs()]
    # Soft outer shadow is deliberately avoided; the window itself is opaque.
    out.append(rect(70, 80, 1460, 840, fill="#050507", rx="14", opacity="0.65"))
    out.append(rect(80, 90, 1440, 820, fill=BG, rx="12"))
    out.append(group("", clip="mac-window"))

    content = []
    # macOS title bar.
    content.append(rect(80, 90, 1440, 48, fill="#1A1A1F"))
    content.append(rect(80, 138, 1440, 1, fill="#2A2A30"))
    content.append(circle(108, 114, 7, "#FF5F57"))
    content.append(circle(130, 114, 7, "#FEBC2E"))
    content.append(circle(152, 114, 7, "#28C840"))
    content.append(text(800, 119, "PCHome", size=12.5, fill=TEXT_2, weight=600, anchor="middle"))

    # Top app bar.
    content.append(rect(80, 138, 1440, 54, fill=BG))
    content.append(rect(80, 192, 1440, 1, fill="#202126"))
    content.append(text(104, 172, "PCHome", size=18, weight=700, fill=TEXT))
    content.append(circle(196, 166, 4.5, GREEN))
    content.append(text(208, 171, "Connected", size=10.5, fill=TEXT_2))
    # Right icons.
    for i, label in enumerate(["⟳", "⚙", "⏻"]):
        content.append(rect(1380 + i * 42, 150, 34, 30, fill="#1F1F23", rx="8"))
        content.append(text(1397 + i * 42, 170, label, size=14, fill=MUTED, anchor="middle"))


    # Sidebar.
    content.append(rect(80, 193, 340, 717, fill="#121214"))
    content.append(rect(420, 193, 1, 717, fill="#222328"))

    content.append(text(104, 230, "PC IDENTITY", size=9, fill=DIM, weight=700, letter_spacing=1.2))
    content.append(text(104, 278, "482 913", size=34, fill=TEXT, weight=700, family=MONO))
    content.append(rect(384, 246, 24, 24, fill="#1F1F23", rx="6"))
    content.append(text(396, 263, "⧉", size=12, fill=MUTED, anchor="middle"))
    content.append(text(104, 304, "Enter this in the PC ID field on your phone", size=9.5, fill=MUTED))

    content.append(text(104, 342, "PASSWORD", size=9, fill=DIM, weight=700, letter_spacing=1.2))
    content.append(text(104, 378, "••••••", size=20, fill=TEXT_2, family=MONO))
    content.append(text(104, 399, "6-digit fixed password", size=9, fill=MUTED))
    for i, label in enumerate(["↻", "⧉", "◉"]):
        content.append(rect(338 + i * 30, 350, 22, 22, fill="#1F1F23", rx="6"))
        content.append(text(349 + i * 30, 365, label, size=11, fill=MUTED, anchor="middle"))

    content.append(rect(104, 420, 292, 1, fill="#222328"))
    content.append(text(104, 452, "DEVICE", size=9, fill=DIM, weight=700, letter_spacing=1.2))
    rows = [
        ("Name", "Studio-PC"),
        ("Local IP", "192.0.2.10"),
        ("Resolution", "1920x1080"),
        ("Capture", "Wayland portal"),
        ("Server", "pchome.example.com"),
        ("Mode", "Trackpad"),
    ]
    y = 476
    for key, value in rows:
        content.append(text(104, y, key, size=9.5, fill=MUTED))
        content.append(text(396, y, value, size=9.5, fill=TEXT_2, anchor="end", family=MONO))
        y += 21

    content.append(rect(104, 618, 292, 1, fill="#222328"))
    content.append(text(104, 650, "CONNECTED DEVICE", size=9, fill=DIM, weight=700, letter_spacing=1.2))
    content.append(circle(112, 678, 4.5, GREEN))
    content.append(text(124, 682, "Connected", size=10.5, fill=TEXT_2))
    content.append(text(104, 708, "Pixel 10 Pro", size=15, fill=TEXT, weight=600))
    content.append(text(104, 727, "since 14:32  •  last auth 14:32", size=9.5, fill=MUTED))
    content.append(rect(104, 746, 292, 1, fill="#222328"))
    content.append(text(104, 778, "RECENT DEVICES", size=9, fill=DIM, weight=700, letter_spacing=1.2))
    content.append(text(104, 800, "Pixel 10 Pro", size=10, fill=TEXT_2))
    content.append(text(396, 800, "14:32", size=9, fill=MUTED, anchor="end", family=MONO))
    content.append(rect(104, 824, 292, 1, fill="#222328"))
    content.append(text(104, 858, "v0.9.3", size=9, fill=DIM, family=MONO))
    content.append(text(396, 858, "Update", size=10, fill=ACCENT, anchor="end"))

    # Main stage.
    content.append(text(456, 230, "●  LIVE PREVIEW", size=9.5, fill=MUTED, weight=700, letter_spacing=1.1))
    content.append(text(1490, 230, "1920x1080", size=9.5, fill=MUTED, anchor="end", family=MONO))
    content.append(rect(452, 246, 1042, 438, fill="#09090B", rx="10"))
    content.append(rect(452, 246, 1042, 438, fill="url(#stage-glow)", rx="10"))
    content.append(rect(452, 246, 1042, 438, fill="none", rx="10", stroke="#27272A", stroke_width="1"))
    content.append(fake_desktop(468, 262, 1010, 406, 0.94))
    content.append(rect(482, 278, 168, 28, fill="#09090B", rx="8", opacity="0.84"))
    content.append(circle(498, 292, 4, GREEN))
    content.append(text(510, 296, "Connected  •  14:32", size=9.5, fill=TEXT_2))

    # Metrics.
    content.append(text(456, 722, "LATENCY", size=8.5, fill=DIM, weight=700, letter_spacing=1.0))
    content.append(text(456, 744, "18 ms", size=15, fill=TEXT, weight=600, family=MONO))
    content.append(text(560, 722, "FPS", size=8.5, fill=DIM, weight=700, letter_spacing=1.0))
    content.append(text(560, 744, "30", size=15, fill=TEXT, weight=600, family=MONO))
    content.append(text(640, 722, "INPUT", size=8.5, fill=DIM, weight=700, letter_spacing=1.0))
    content.append(text(640, 744, "ACTIVE", size=15, fill=GREEN, weight=600, family=MONO))
    content.append(circle(700, 739, 4, GREEN))

    # Event drawer.
    content.append(rect(452, 782, 1042, 1, fill="#27272A"))
    content.append(text(456, 810, "EVENTS", size=9, fill=DIM, weight=700, letter_spacing=1.2))
    content.append(text(1490, 810, "6", size=9.5, fill=MUTED, anchor="end"))
    events = [
        "phone verified: Pixel 10 Pro",
        "control data channel opened (session 42, 1920x1080)",
        "device connected (data channel open)",
        "ID and password copied",
    ]
    y = 834
    for event in events:
        content.append(circle(460, y - 4, 3.2, GREEN))
        content.append(text(472, y, event, size=9.5, fill=TEXT_2, family=MONO))
        y += 19

    out.append(group("".join(content), clip="mac-window"))
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="1000" '
        'viewBox="0 0 1600 1000" role="img" '
        'aria-label="PCHome desktop app with a connected phone">'
        + "".join(out)
        + "</svg>"
    )


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "pixel-10-pro.svg").write_text(phone_screenshot(), encoding="utf-8")
    (OUT / "desktop.svg").write_text(mac_window(), encoding="utf-8")
    print(f"Wrote {OUT / 'pixel-10-pro.svg'}")
    print(f"Wrote {OUT / 'desktop.svg'}")


if __name__ == "__main__":
    main()
