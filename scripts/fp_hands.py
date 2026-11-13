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
# The walkie hand: the same left hand, 178 px left and 75 px down, in front of the handset.
left = Image.open(f'{out}/fp_left.webp')
moved = Image.new('RGBA', left.size, (0, 0, 0, 0)); moved.paste(left, (-178, 75), left)
save(moved, 'fp_lwalkie_back'); save(moved, 'fp_lwalkie_front')
