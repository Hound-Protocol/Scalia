#!/usr/bin/env python3
"""Bakes relief lighting into the PDA image (a bump map from its own shading): the frame's raised parts catch a
light from the top left and cast a soft edge shadow, so the device reads as moulded rather than printed.
The display area is left untouched. Usage: python3 -I scripts/pda_relief.py assets/source/pda_front_flat.webp web/img/pda_front.webp"""
import sys
import numpy as np
from PIL import Image, ImageFilter

src, out = sys.argv[1], sys.argv[2]
im = Image.open(src).convert('RGBA')
a = np.asarray(im).astype(float) / 255
rgb, alpha = a[..., :3], a[..., 3]
lum = rgb @ [0.3, 0.59, 0.11]
# Height: brighter is higher, smoothed so the texture noise doesn't sparkle; the silhouette slopes off.
h = np.asarray(Image.fromarray((lum * alpha * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(2.2))).astype(float) / 255
gy, gx = np.gradient(h)
k = 9.0
n = np.dstack([-gx * k, -gy * k, np.ones_like(h)]); n /= np.linalg.norm(n, axis=2, keepdims=True)
light = np.array([-0.55, -0.6, 0.58]); light /= np.linalg.norm(light)
shade = n @ light / light[2]           # 1 on a flat face
shade = 1 + (shade - 1) * 1.35
H, W = h.shape
yy, xx = np.mgrid[0:H, 0:W]
screen = (xx > 0.105 * W) & (xx < 0.898 * W) & (yy > 0.119 * H) & (yy < 0.877 * H)
shade[screen] = 1.0
lit = np.clip(rgb * shade[..., None], 0, 1)
Image.fromarray((np.dstack([lit, alpha]) * 255).astype(np.uint8), 'RGBA').save(out, 'WEBP', quality=92, method=6)
