#!/usr/bin/env python3
"""The hands used on the tablet are the earlier first-person render (assets/source/fp_hands_ref.webp: body and both
arms in one image), preferred over a fresh render of the current mesh. This splits its arms into fp_left / fp_right
(against fp_body from scripts/first_person.py) and builds the walkie-talkie hand from the same left hand, moved to
grip the handset's lower left. Usage: python3 -I scripts/fp_hands.py web/img"""
import sys
import numpy as np
from PIL import Image

out = sys.argv[1]
o = np.asarray(Image.open('assets/source/fp_hands_ref.webp')).copy()
body = np.asarray(Image.open(f'{out}/fp_body.webp'))
H, W = o.shape[:2]
same = (np.abs(o[..., :3].astype(int) - body[..., :3].astype(int)).sum(2) < 40) & (body[..., 3] > 0)
arm = (o[..., 3] > 0) & ~same
xx = np.arange(W)[None, :].repeat(H, 0)
save = lambda im, name: im.save(f'{out}/{name}.webp', 'WEBP', quality=90, method=6)
for name, m in (('fp_left', arm & (xx < W // 2)), ('fp_right', arm & (xx >= W // 2))):
    a = o.copy(); a[..., 3] = np.where(m, o[..., 3], 0)
    save(Image.fromarray(a), name)
# The right hand's own thumb is taken out (painted over in the glove's shade, as if bent away behind the
# tablet): the separate blocky thumb (fp_thumb, first_person.py) does the pressing.
from PIL import ImageDraw
right = Image.open(f'{out}/fp_right.webp')
rgba = np.asarray(right).copy()
mask = Image.new('L', right.size, 0)
ImageDraw.Draw(mask).polygon([(1308, 492), (1376, 482), (1400, 580), (1312, 596), (1296, 590), (1277, 536)], fill=255)
m = (np.asarray(mask) > 0) & (rgba[..., 3] > 0)
rgba[m, :3] = (31, 33, 35)
save(Image.fromarray(rgba), 'fp_right')
# The left hand loses its thumb the same way (mirrored polygon): its blocky thumb is fp_thumb flipped.
left = Image.open(f'{out}/fp_left.webp')
rgba = np.asarray(left).copy()
mask = Image.new('L', left.size, 0)
ImageDraw.Draw(mask).polygon([(1920 - x, y) for x, y in [(1308, 492), (1376, 482), (1400, 580), (1312, 596), (1296, 590), (1277, 536)]], fill=255)
m = (np.asarray(mask) > 0) & (rgba[..., 3] > 0)
rgba[m, :3] = (31, 33, 35)
save(Image.fromarray(rgba), 'fp_left')
# The walkie hand: the same left hand, raised so the handset sits in its palm; the handset is drawn wholly in
# front of it (the front layer stays empty).
left = Image.open(f'{out}/fp_left.webp')
moved = Image.new('RGBA', left.size, (0, 0, 0, 0)); moved.paste(left, (-188, 5), left)
save(moved, 'fp_lwalkie_back')
save(Image.new('RGBA', left.size, (0, 0, 0, 0)), 'fp_lwalkie_front')
