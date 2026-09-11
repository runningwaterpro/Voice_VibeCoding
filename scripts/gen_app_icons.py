# -*- coding: utf-8 -*-
"""Generate Voice VibeCoding icons: blue main + red/orange tray states."""
from PIL import Image, ImageDraw
import os

OUT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons"))

# Colors matching user-provided icons
BLUE = (47, 149, 243)       # main / ready
RED = (240, 67, 58)         # error
ORANGE = (255, 152, 30)     # init / warning
WHITE = (255, 255, 255)


def draw_mark(img: Image.Image, color: tuple, mark_color=WHITE):
    """White ring + three rounded bars (mic/voice mark) on transparent square."""
    w, h = img.size
    d = ImageDraw.Draw(img)
    # full rounded-square background
    radius = int(w * 0.18)
    d.rounded_rectangle([0, 0, w - 1, h - 1], radius=radius, fill=color)

    # outer arc / ring (partial circle like logo)
    ring = w * 0.12
    pad = w * 0.14
    bbox = [pad, pad, w - pad, h - pad]
    stroke = max(2, int(w * 0.045))
    # open ring: start ~ -20deg to ~ 200deg via arc (PIL angles: 0=right, clockwise)
    d.arc(bbox, start=35, end=360, fill=mark_color, width=stroke)
    # bottom hook like user logo
    d.arc([pad, pad + h * 0.08, w - pad, h - pad], start=200, end=340, fill=mark_color, width=stroke)

    # three vertical rounded bars (equalizer / mic)
    bar_w = max(3, int(w * 0.07))
    gap = w * 0.055
    cx = w // 2
    heights = [w * 0.22, w * 0.38, w * 0.22]
    y_mid = h * 0.48
    xs = [cx - bar_w - gap, cx, cx + bar_w + gap]
    for x, bh in zip(xs, heights):
        top = y_mid - bh / 2
        bot = y_mid + bh / 2
        d.rounded_rectangle(
            [x - bar_w // 2, top, x + bar_w // 2, bot],
            radius=bar_w // 2,
            fill=mark_color,
        )


def make_icon(size: int, color: tuple) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw_mark(img, color)
    return img


def save_ico(sizes, color, path):
    imgs = [make_icon(s, color) for s in sizes]
    imgs[-1].save(path, format="ICO", sizes=[(s, s) for s in sizes], append_images=imgs[:-1])
    print("ico", path, os.path.getsize(path))


def save_png(size, color, path):
    make_icon(size, color).save(path, format="PNG")
    print("png", path, os.path.getsize(path))


# Main app icon (blue) — multi-res for Explorer/taskbar
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
save_ico(ICO_SIZES, BLUE, os.path.join(OUT, "icon.ico"))
save_png(256, BLUE, os.path.join(OUT, "icon.png"))
save_png(128, BLUE, os.path.join(OUT, "128x128.png"))
save_png(128, BLUE, os.path.join(OUT, "128x128@2x.png"))
save_png(32, BLUE, os.path.join(OUT, "32x32.png"))
# Tauri often wants square names too
save_png(256, BLUE, os.path.join(OUT, "128x128.png"))  # keep 128 path used by tray window icon

# Tray: blue=ready, orange=init, red=error
save_png(128, BLUE, os.path.join(OUT, "tray-icon.png"))
save_png(32, BLUE, os.path.join(OUT, "tray-icon-32.png"))
save_png(128, ORANGE, os.path.join(OUT, "tray-icon-init.png"))
save_png(32, ORANGE, os.path.join(OUT, "tray-icon-init-32.png"))
save_png(128, RED, os.path.join(OUT, "tray-icon-error.png"))
save_png(32, RED, os.path.join(OUT, "tray-icon-error-32.png"))
# alias used by some paths
save_png(128, ORANGE, os.path.join(OUT, "icon-init.png"))
print("done", OUT)
