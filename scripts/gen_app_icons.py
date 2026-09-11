# -*- coding: utf-8 -*-
"""Faithful recreation of user icon: gradient rounded square + white ring + 3 bars."""
from PIL import Image, ImageDraw
import os

OUT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons"))

# User icon colors
BLUE_TOP = (90, 178, 255)
BLUE_BOT = (40, 140, 235)
RED_TOP = (255, 110, 100)
RED_BOT = (230, 55, 45)
ORANGE_TOP = (255, 180, 80)
ORANGE_BOT = (250, 130, 20)
WHITE = (255, 255, 255)


def lerp(a, b, t):
    return int(a + (b - a) * t)


def gradient_roundrect(size, c_top, c_bot, radius_ratio=0.16):
    """Vertical/diagonal gradient rounded square."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    # fill gradient via horizontal strips (diagonal-ish)
    for y in range(size):
        t = y / max(size - 1, 1)
        # slight diagonal: lighter at top-left
        for x in range(0, size, max(1, size // 64)):
            tx = (t + x / max(size - 1, 1)) / 2
            color = (
                lerp(c_top[0], c_bot[0], tx),
                lerp(c_top[1], c_bot[1], tx),
                lerp(c_top[2], c_bot[2], tx),
                255,
            )
            d.rectangle([x, y, x + max(1, size // 64) - 1, y], fill=color)
    # mask to rounded rect
    mask = Image.new("L", (size, size), 0)
    md = ImageDraw.Draw(mask)
    r = int(size * radius_ratio)
    md.rounded_rectangle([0, 0, size - 1, size - 1], radius=r, fill=255)
    img.putalpha(mask)
    return img


def draw_logo(img, scale=1.0):
    """White almost-full ring + 3 vertical bars + bottom hook. Matches user art."""
    w, h = img.size
    d = ImageDraw.Draw(img)
    # stroke scales: thicker relatively on small icons
    if w <= 32:
        stroke = max(2, int(w * 0.09))
    elif w <= 64:
        stroke = max(3, int(w * 0.075))
    else:
        stroke = max(4, int(w * 0.055))

    # outer ring circle
    pad = w * (0.12 if w > 48 else 0.10)
    bbox = [pad, pad, w - pad, h - pad]
    # leave gap at bottom-right like original (approx 300°–40°)
    d.arc(bbox, start=50, end=360, fill=WHITE, width=stroke)
    # bottom-right hook (short thick arc)
    hook_pad = w * 0.22
    d.arc(
        [hook_pad, hook_pad, w - hook_pad, h - hook_pad * 0.85],
        start=195,
        end=340,
        fill=WHITE,
        width=stroke,
    )

    # 3 vertical rounded bars — pattern from original: short | tall | short-ish middle-tall
    # looking at original: left medium-short, center tall, right medium
    bar_w = max(3, int(w * 0.08))
    gap = w * 0.06
    cx = w / 2
    # heights as fraction of width (original proportions)
    heights = [w * 0.28, w * 0.46, w * 0.30]
    # vertical center slightly above middle
    y_mid = h * 0.46
    xs = [cx - bar_w - gap, cx, cx + bar_w + gap]
    for x, bh in zip(xs, heights):
        top = y_mid - bh * 0.55  # slightly more above center
        bot = y_mid + bh * 0.45
        d.rounded_rectangle(
            [x - bar_w / 2, top, x + bar_w / 2, bot],
            radius=bar_w / 2,
            fill=WHITE,
        )


def make(size, top, bot, simplify_small=False):
    img = gradient_roundrect(size, top, bot)
    if simplify_small and size <= 24:
        # solid bg for tiny tray — gradient mushes
        img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        d = ImageDraw.Draw(img)
        r = max(2, int(size * 0.18))
        d.rounded_rectangle([0, 0, size - 1, size - 1], radius=r, fill=bot)
        # bolder mark
        stroke = max(2, int(size * 0.10))
        pad = size * 0.12
        d.arc([pad, pad, size - pad, size - pad], start=50, end=360, fill=WHITE, width=stroke)
        bw = max(2, int(size * 0.10))
        gap = size * 0.07
        cx = size // 2
        ym = size * 0.48
        for i, hh in enumerate([size * 0.26, size * 0.42, size * 0.28]):
            x = cx + (i - 1) * (bw + gap)
            d.rounded_rectangle([x - bw / 2, ym - hh * 0.5, x + bw / 2, ym + hh * 0.5], radius=bw // 2, fill=WHITE)
        return img
    draw_logo(img)
    return img


def save_png(size, path, **kw):
    make(size, **kw).save(path, format="PNG")
    print(f"{size:>4} {os.path.basename(path)} {os.path.getsize(path)}")


# ── Main blue ──
save_png(256, os.path.join(OUT, "icon.png"), top=BLUE_TOP, bot=BLUE_BOT)
save_png(128, os.path.join(OUT, "128x128.png"), top=BLUE_TOP, bot=BLUE_BOT)
save_png(128, os.path.join(OUT, "128x128@2x.png"), top=BLUE_TOP, bot=BLUE_BOT)
save_png(64, os.path.join(OUT, "icon-64.png"), top=BLUE_TOP, bot=BLUE_BOT)
save_png(48, os.path.join(OUT, "icon-48.png"), top=BLUE_TOP, bot=BLUE_BOT)
save_png(32, os.path.join(OUT, "32x32.png"), top=BLUE_TOP, bot=BLUE_BOT)
save_png(24, os.path.join(OUT, "icon-24.png"), top=BLUE_TOP, bot=BLUE_BOT, simplify_small=True)
save_png(16, os.path.join(OUT, "icon-16.png"), top=BLUE_TOP, bot=BLUE_BOT, simplify_small=True)

# ICO multi-res
ico_imgs = [make(s, top=BLUE_TOP, bot=BLUE_BOT, simplify_small=(s <= 24)) for s in (16, 24, 32, 48, 64, 128, 256)]
ico_path = os.path.join(OUT, "icon.ico")
ico_imgs[-1].save(ico_path, format="ICO", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)], append_images=ico_imgs[:-1])
print("ICO", os.path.getsize(ico_path))

# ── Tray ready = blue ──
save_png(128, os.path.join(OUT, "tray-icon.png"), top=BLUE_TOP, bot=BLUE_BOT)
save_png(32, os.path.join(OUT, "tray-icon-32.png"), top=BLUE_TOP, bot=BLUE_BOT, simplify_small=True)

# ── Tray init = orange ──
save_png(128, os.path.join(OUT, "tray-icon-init.png"), top=ORANGE_TOP, bot=ORANGE_BOT)
save_png(32, os.path.join(OUT, "tray-icon-init-32.png"), top=ORANGE_TOP, bot=ORANGE_BOT, simplify_small=True)
save_png(16, os.path.join(OUT, "tray-icon-init-16.png"), top=ORANGE_TOP, bot=ORANGE_BOT, simplify_small=True)

# ── Tray error = red ──
save_png(128, os.path.join(OUT, "tray-icon-error.png"), top=RED_TOP, bot=RED_BOT)
save_png(32, os.path.join(OUT, "tray-icon-error-32.png"), top=RED_TOP, bot=RED_BOT, simplify_small=True)
save_png(16, os.path.join(OUT, "tray-icon-error-16.png"), top=RED_TOP, bot=RED_BOT, simplify_small=True)

# alias
save_png(128, os.path.join(OUT, "icon-init.png"), top=ORANGE_TOP, bot=ORANGE_BOT)
print("done")
