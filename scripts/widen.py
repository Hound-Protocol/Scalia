"""Makes the tablet ultrawide: stretches only its plain middle (between the corner bumpers), so the corners, LEDs
and side buttons keep their shape. Works on the face-on image and on the in-hand mesh alike.

Usage: python3 -I scripts/widen.py image in.png out.webp aspect
       python3 -I scripts/widen.py mesh in.fig out.fig aspect scale
"""
import struct, sys
import numpy as np
from PIL import Image

SLICE = (0.34, 0.66)  # the stretchable middle, as fractions of the image width


def main():
    mode = sys.argv[1]
    if mode == 'image':
        src, out, aspect = sys.argv[2], sys.argv[3], float(sys.argv[4])
        im = Image.open(src)
        w, h = im.size
        a, b = int(w * SLICE[0]), int(w * SLICE[1])
        nw = int(round(h * aspect))
        mid = nw - a - (w - b)
        wide = Image.new('RGBA', (nw, h))
        wide.paste(im.crop((0, 0, a, h)), (0, 0))
        wide.paste(im.crop((a, 0, b, h)).resize((mid, h), Image.LANCZOS), (a, 0))
        wide.paste(im.crop((b, 0, w, h)), (a + mid, 0))
        wide.save(out, 'WEBP', quality=90, method=6)
        # Where an x fraction of the original lands in the wide one.
        def map_x(f):
            px = f * w
            if px <= a: return px / nw
            if px >= b: return (a + mid + px - b) / nw
            return (a + (px - a) * mid / (b - a)) / nw
        print(f'size {nw}x{h} k {mid / (b - a):.4f}')
        for name, f in [('screen left', 0.1419), ('screen right', 0.8619), ('buttons left', 0.886), ('buttons right', 0.942)]:
            print(f'{name}: {map_x(f) * 100:.3f}%')
    else:
        src, out, aspect, scale = sys.argv[2], sys.argv[3], float(sys.argv[4]), float(sys.argv[5])
        d = np.frombuffer(open(src, 'rb').read(), np.float32).reshape(-1, 4).astype(float)
        lo, hi = d[:, :3].min(0), d[:, :3].max(0)
        w, h = hi[0] - lo[0], hi[1] - lo[1]
        # Same stretch as the image: the image is the model plus 2% margins of its width (glb_render_front).
        full = w / 0.96
        x0 = lo[0] - full * 0.02
        a, b = x0 + full * SLICE[0], x0 + full * SLICE[1]
        img_h = h + full * 0.04
        target_w = img_h * aspect
        k = (target_w - full * (1 - (SLICE[1] - SLICE[0]))) / (full * (SLICE[1] - SLICE[0]))
        x = d[:, 0]
        nx = np.where(x <= a, x - (b - a) * (k - 1) / 2, np.where(x >= b, x + (b - a) * (k - 1) / 2, (x - (a + b) / 2) * k))
        d[:, 0] = nx
        d[:, :3] *= scale
        with open(out, 'wb') as f:
            for row in d:
                f.write(struct.pack('<4f', *row))
        print('k', round(k, 4), 'size', (d[:, :3].max(0) - d[:, :3].min(0)).round(3))


main()
