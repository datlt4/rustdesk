#!/usr/bin/env python3
"""Recolors the blue RustDesk logo red in every icon file (svg/png/ico/icns). Run once from the repo root."""
import colorsys
import glob
import re
import sys

import numpy as np
from PIL import Image

# Logo gradient: cyan #00bfe1 (~189°) .. blue #0071ff (~213°), shadow #004ba6.
# Cyan becomes a light orange-red, blue a deep red; white and grey stay.
BLUE_LO, BLUE_HI = 170.0, 250.0


def map_hue(deg):
    t = min(max((deg - 189.0) / (213.0 - 189.0), 0.0), 1.0)
    return (15.0 - 20.0 * t) % 360.0  # 15° (orange-red) .. 355° (red)


def recolor_rgba(arr):
    rgb = arr[..., :3].astype(np.float32) / 255.0
    mx, mn = rgb.max(-1), rgb.min(-1)
    delta = mx - mn
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    safe = np.where(delta == 0, 1, delta)
    h = np.where(mx == r, ((g - b) / safe) % 6,
                 np.where(mx == g, (b - r) / safe + 2, (r - g) / safe + 4)) * 60.0
    s = np.where(mx == 0, 0, delta / np.where(mx == 0, 1, mx))
    mask = (delta > 0.08) & (s > 0.25) & (h >= BLUE_LO) & (h <= BLUE_HI)
    t = np.clip((h - 189.0) / 24.0, 0, 1)
    nh = ((15.0 - 20.0 * t) % 360.0) / 60.0
    v = mx
    c = v * s
    x = c * (1 - np.abs(nh % 2 - 1))
    m = v - c
    i = np.floor(nh).astype(int) % 6
    z = np.zeros_like(c)
    table = [(c, x, z), (x, c, z), (z, c, x), (z, x, c), (x, z, c), (c, z, x)]
    out = rgb.copy()
    for k, (rr, gg, bb) in enumerate(table):
        sel = mask & (i == k)
        out[..., 0][sel] = (rr + m)[sel]
        out[..., 1][sel] = (gg + m)[sel]
        out[..., 2][sel] = (bb + m)[sel]
    res = arr.copy()
    res[..., :3] = np.clip(out * 255.0 + 0.5, 0, 255).astype(np.uint8)
    return res, int(mask.sum())


def recolor_image(im):
    mode = im.mode
    rgba = im.convert("RGBA")
    arr, n = recolor_rgba(np.array(rgba))
    out = Image.fromarray(arr, "RGBA")
    if mode in ("RGB", "L", "P"):
        out = out.convert("RGB" if mode != "P" else "RGBA")
    return out, n


def hex_recolor(m):
    h = m.group(0)
    r, g, b = (int(h[i:i + 2], 16) / 255.0 for i in (1, 3, 5))
    hh, ll, ss = colorsys.rgb_to_hls(r, g, b)
    deg = hh * 360.0
    if ss < 0.25 or not (BLUE_LO <= deg <= BLUE_HI) or max(r, g, b) - min(r, g, b) < 0.08:
        return h
    nr, ng, nb = colorsys.hls_to_rgb(map_hue(deg) / 360.0, ll, ss)
    return "#%02x%02x%02x" % (round(nr * 255), round(ng * 255), round(nb * 255))


def do_svg(path):
    s = open(path, encoding="utf-8").read()
    t = re.sub(r"#[0-9a-fA-F]{6}\b", hex_recolor, s)
    if t != s:
        open(path, "w", encoding="utf-8").write(t)
        return True
    return False


def do_png(path):
    im = Image.open(path)
    out, n = recolor_image(im)
    if n:
        out.save(path)
    return n > 0


def do_ico(path):
    im = Image.open(path)
    sizes = sorted(im.info["sizes"], key=lambda s: -s[0])
    frames = []
    for size in sizes:
        im.size = size
        im.load()
        frames.append(recolor_image(im.copy())[0].convert("RGBA"))
    frames[0].save(path, format="ICO", sizes=sizes, append_images=frames[1:])
    return True


def do_icns(path):
    im = Image.open(path)
    best = max(im.info["sizes"], key=lambda s: s[0] * s[2])
    im.size = best
    im.load()
    recolor_image(im)[0].convert("RGBA").save(path, format="ICNS")
    return True


FILES = [
    "res/*.svg", "flutter/assets/icon.svg",
    "res/icon.png", "res/mac-icon.png", "res/32x32.png", "res/64x64.png",
    "res/128x128.png", "res/128x128@2x.png",
    "flutter/android/app/src/main/res/mipmap-*/*.png",
    "flutter/ios/Runner/Assets.xcassets/AppIcon.appiconset/*.png",
    "flutter/ios/Runner/Assets.xcassets/LaunchImage.imageset/*.png",
    "fastlane/metadata/android/en-US/images/icon.png",
    "libs/portable/src/res/label.png",
    "res/icon.ico", "res/tray-icon.ico", "flutter/windows/runner/resources/app_icon.ico",
    "flutter/macos/Runner/AppIcon.icns",
]

if __name__ == "__main__":
    for pattern in FILES:
        for path in sorted(glob.glob(pattern)):
            ext = path.rsplit(".", 1)[-1].lower()
            changed = {"svg": do_svg, "png": do_png, "ico": do_ico, "icns": do_icns}[ext](path)
            print(("recolored " if changed else "unchanged ") + path)
    sys.exit(0)
