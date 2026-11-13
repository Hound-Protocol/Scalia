"""(The arm layers shipped are the earlier hand render, rebuilt by scripts/fp_hands.py after this script; only
fp_body and fp_thumb are taken from here.)

Renders what the commander sees looking down at his tablet: his own body, arms and gloved hands, from his eyes,
in two layers split by the tablet's plane. The back layer goes behind the high-resolution tablet close-up and the
front layer (fingers and thumbs over its edges) on top. Prints where the tablet's image falls, as CSS.

Input: the soldier in the tablet pose (examples/pose_dump 10) and the tablet mesh (assets/pda.fig, face-on frame).
Usage: cargo run --release -p scalia-render --example fp_pose_dump -- /tmp/fp_tab.fig
       cargo run --release -p scalia-render --example fp_pose_dump -- /tmp/fp_walkie.fig walkie -0.32 0.36 1.14 > /tmp/fp_walkie.json
       python3 -I scripts/first_person.py /tmp/fp_tab.fig crates/scalia-render/assets/pda.fig out_dir /tmp/fp_walkie.fig /tmp/fp_walkie.json
"""
import sys
import numpy as np
from PIL import Image

W, H = 1920, 1080
HFOV = np.radians(100.0)
EYE = np.array([0.0, 0.16, 1.57])
TABLET_AT = np.array([0.0, 0.375, 1.19])
TILT = 0.85
# The ultrawide tablet image (scripts/widen.py): width and margin in pixels, the display's span across it.
IMG_W_PX, IMG_MARGIN_PX = 2164, 32
SCREEN_X = (0.10492, 0.89789)  # (reference only)
# Palette (gpu.rs PALETTE), with the commander's recolouring (viewer.rs commander_figure) and his red armband.
PAL = [[0.10, 0.12, 0.15], [0.42, 0.46, 0.52], [0.55, 0.58, 0.60], [0.30, 0.33, 0.38], [0.72, 0.58, 0.20],
       [0.80, 0.35, 0.15], [0.25, 0.50, 0.70], [0.30, 0.62, 0.40], [0.90, 0.88, 0.80], [0.60, 0.20, 0.20],
       [0.25, 0.40, 0.20], [0.32, 0.48, 0.24], [0.40, 0.52, 0.28], [0.48, 0.44, 0.30], [0.95, 0.80, 0.35],
       [0.85, 0.85, 0.90], [0.62, 0.60, 0.44], [0.49, 0.47, 0.34], [0.17, 0.18, 0.19], [0.58, 0.60, 0.62],
       [0.40, 0.38, 0.28], [0.25, 0.26, 0.27], [0.12, 0.12, 0.13], [0.13, 0.14, 0.15], [0.20, 0.21, 0.22],
       [0.10, 0.11, 0.12], [0.63, 0.61, 0.45], [0.86, 0.70, 0.55], [0.08, 0.08, 0.09], [0.17, 0.18, 0.19],
       [0.72, 0.62, 0.42], [0.60, 0.20, 0.20]]
RECOLOUR = {16: 23, 17: 28, 20: 28, 26: 28, 19: 29, 22: 29}


def camera():
    f = TABLET_AT - EYE; f /= np.linalg.norm(f)
    r = np.cross(f, [0, 0, 1.0]); r /= np.linalg.norm(r)
    u = np.cross(r, f)
    return r, u, f


def project(p, cam):
    r, u, f = cam
    d = p - EYE
    x, y, z = d @ r, d @ u, d @ f
    F = (W / 2) / np.tan(HFOV / 2)
    return np.stack([W / 2 + x / z * F, H / 2 - y / z * F, z], -1)


def raster(tris, codes, cam, light, rgb=None, alpha=None, depth=None):
    """Flat-shaded, z-buffered triangles (perspective-correct depth). Returns rgb, alpha, depth."""
    if rgb is None:
        rgb = np.zeros((H, W, 3)); alpha = np.zeros((H, W)); depth = np.full((H, W), np.inf)
    for t, code in zip(tris, codes):
        P = project(t, cam)
        if (P[:, 2] < 0.04).any():
            continue
        x, y, z = P[:, 0], P[:, 1], P[:, 2]
        x0, x1 = int(max(0, np.floor(x.min()))), int(min(W - 1, np.ceil(x.max())))
        y0, y1 = int(max(0, np.floor(y.min()))), int(min(H - 1, np.ceil(y.max())))
        if x1 < x0 or y1 < y0:
            continue
        sx, sy = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
        den = (y[1] - y[2]) * (x[0] - x[2]) + (x[2] - x[1]) * (y[0] - y[2])
        if abs(den) < 1e-9:
            continue
        w0 = ((y[1] - y[2]) * (sx - x[2]) + (x[2] - x[1]) * (sy - y[2])) / den
        w1 = ((y[2] - y[0]) * (sx - x[2]) + (x[0] - x[2]) * (sy - y[2])) / den
        w2 = 1 - w0 - w1
        inside = (w0 >= 0) & (w1 >= 0) & (w2 >= 0)
        if not inside.any():
            continue
        iz = w0 / z[0] + w1 / z[1] + w2 / z[2]
        zz = 1 / np.where(iz <= 0, 1e-9, iz)
        sub = depth[y0:y1 + 1, x0:x1 + 1]
        win = inside & (zz < sub)
        if not win.any():
            continue
        n = np.cross(t[1] - t[0], t[2] - t[0]); n /= np.linalg.norm(n) + 1e-12
        pal = int(code) % 32
        pal = RECOLOUR.get(pal, pal)
        col = np.array(PAL[pal]) * (0.42 + 0.58 * abs(float(n @ light)))
        sub[win] = zz[win]
        rgb[y0:y1 + 1, x0:x1 + 1][win] = col
        alpha[y0:y1 + 1, x0:x1 + 1][win] = 1.0
    return rgb, alpha, depth


def load_posed(path):
    d = np.frombuffer(open(path, 'rb').read(), np.float32).reshape(-1, 3, 4).astype(float)
    tris, codes = d[:, :, :3], d[:, 0, 3]
    parts = (codes // 32).astype(int)
    keep = ~np.isin(parts, [10, 9])  # not his own head; the rifle is slung behind
    return tris[keep], codes[keep], parts[keep]


def box(c, ax, ay, az, hx, hy, hz):
    """Triangles of a box centred at c with half extents along the unit axes ax, ay, az."""
    v = [c + sx * hx * ax + sy * hy * ay + sz * hz * az for sx in (-1, 1) for sy in (-1, 1) for sz in (-1, 1)]
    q = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    return np.array([[v[a], v[b], v[c2]] for a, b, c2, d in q for (a, b, c2) in ((a, b, c2), (a, c2, d))])


def prism(a, b, ra, rb, n=8, phase=0.0):
    """Triangles of a tapered n-sided tube from a (radius ra) to b (radius rb), capped."""
    ax = b - a; ax = ax / np.linalg.norm(ax)
    u = np.cross(ax, [0, 0, 1.0]);
    if np.linalg.norm(u) < 1e-6: u = np.cross(ax, [1.0, 0, 0])
    u /= np.linalg.norm(u); w = np.cross(ax, u)
    ring = lambda c, r: [c + r * (np.cos(phase + k * 2 * np.pi / n) * u + np.sin(phase + k * 2 * np.pi / n) * w) for k in range(n)]
    A, B = ring(a, ra), ring(b, rb)
    t = []
    for k in range(n):
        k2 = (k + 1) % n
        t += [[A[k], A[k2], B[k2]], [A[k], B[k2], B[k]], [a, A[k2], A[k]], [b, B[k], B[k2]]]
    return np.array(t)


def save(img, path):
    Image.fromarray((np.clip(img, 0, 1) * 255).astype(np.uint8), 'RGBA').save(path, 'WEBP', quality=90, method=6)


def main():
    """Layers (all 1920x1080, same frame): fp_body (torso, legs), fp_left / fp_right (each arm holding the tablet),
    fp_lwalkie_back / _front (the left arm holding the walkie-talkie beside the tablet, split by the handset's face),
    fp_thumb (the right thumb lying on the tablet's frame, pointing at the side buttons). Prints the CSS."""
    import json
    posed, pda, out = sys.argv[1], sys.argv[2], sys.argv[3]
    walkie_fig, walkie_json = sys.argv[4], sys.argv[5]
    tris, codes, parts = load_posed(posed)
    cam = camera()
    light = np.array([0.3, 0.4, 0.86]); light /= np.linalg.norm(light)
    s, c = np.sin(TILT), np.cos(TILT)
    R, U, N = np.array([1.0, 0, 0]), np.array([0, s, c]), np.array([0, -c, s])
    LEFT, RIGHT = [3, 11], [4, 12]
    groups = {'body': ~np.isin(parts, LEFT + RIGHT), 'left': np.isin(parts, LEFT), 'right': np.isin(parts, RIGHT)}
    for name, m in groups.items():
        rgb, alpha, _ = raster(tris[m], codes[m], cam, light)
        save(np.dstack([rgb, alpha]), f'{out}/fp_{name}.webp')
    # The left arm raised beside the tablet, the handset in its palm: face towards the eye, upright.
    wt, wc, wp = load_posed(walkie_fig)
    m = np.isin(wp, LEFT)
    rgb, alpha, depth = raster(wt[m], wc[m], cam, light)
    hand = json.loads(open(walkie_json).read())
    palm, out_n = np.array(hand['palm']), np.array(hand['out'])
    WK_H, WK_ASPECT, WK_PAD = 0.20, 730 / 1542, 15 / 1542  # handset height with antenna (m); image aspect, padding
    WK_D = 0.032
    centre = palm - out_n * (WK_H * WK_ASPECT / 2 - 0.004) + np.array([0, 0, WK_H * 0.08])
    fwd = EYE - centre; fwd /= np.linalg.norm(fwd)            # the face looks at the eye
    cu = cam[1]                                               # upright on screen: along the camera's up
    up = cu - fwd * (cu @ fwd); up /= np.linalg.norm(up)
    rt = np.cross(up, fwd)
    centre = centre + up * 0.08 - fwd * WK_D / 2             # held low (the grille), palm at the back
    face_c = centre + fwd * WK_D / 2
    body_h = WK_H * 0.85                                     # without the antenna (the top 15%)
    proxy = box(centre - up * (WK_H - body_h) / 2, rt, up, fwd, WK_H * WK_ASPECT / 2, body_h / 2, WK_D / 2)
    _, _, wdepth = raster(proxy, np.zeros(len(proxy)), cam, light)
    front = alpha * (depth < wdepth - 0.002)
    save(np.dstack([rgb, alpha]), f'{out}/fp_lwalkie_back.webp')
    save(np.dstack([rgb, front]), f'{out}/fp_lwalkie_front.webp')
    hw = WK_H * WK_ASPECT / 2 * (1 + 2 * WK_PAD / WK_ASPECT * 0)
    q = np.array([project(face_c + a * rt + b * up, cam) for a in (-hw, hw) for b in (-WK_H / 2, WK_H / 2)])
    wx0, wy0, wx1, wy1 = q[:, 0].min(), q[:, 1].min(), q[:, 0].max(), q[:, 1].max()
    # The right thumb over the frame: blocky like the hand's fingers (square, a gloved segment then a bare tip),
    # lying on the frame just right of the side buttons, ready to tap them.
    pd = np.frombuffer(open(pda, 'rb').read(), np.float32).reshape(-1, 4)[:, :3].astype(float)
    lo, hi = pd.min(0), pd.max(0)
    width = hi[0] - lo[0]
    mx = width * IMG_MARGIN_PX / (IMG_W_PX - 2 * IMG_MARGIN_PX)
    at = lambda xf, uf, h: TABLET_AT + R * (lo[0] - mx + xf * (width + 2 * mx)) + U * uf + N * h
    TIP_X = 0.973
    base, mid, tip = at(1.05, -0.006, 0.016), at(0.99, 0.0, 0.017), at(TIP_X, 0.0, 0.015)
    sq = np.pi / 4
    th = np.concatenate([prism(base, mid, 0.0175, 0.0165, 4, sq), prism(mid, tip, 0.0155, 0.0145, 4, sq)])
    tc = np.array([22.0 + 32 * 12] * (len(th) // 2) + [27.0 + 32 * 12] * (len(th) // 2))
    rgb, alpha, _ = raster(th, tc, cam, light)
    ys, xs = np.nonzero(alpha)
    tx0, tx1, ty0, ty1 = xs.min() - 2, xs.max() + 3, ys.min() - 2, ys.max() + 3
    save(np.dstack([rgb, alpha])[ty0:ty1, tx0:tx1], f'{out}/fp_thumb.webp')
    tipp = project(at(TIP_X - 0.012, 0.0, 0.015), cam)
    # CSS for layers placed inside #tablet (whose box is the tablet image): size and offset in % of that box.
    corners = [(lo[0] - mx, hi[1] + mx), (hi[0] + mx, hi[1] + mx), (hi[0] + mx, lo[1] - mx), (lo[0] - mx, lo[1] - mx)]
    pts = np.array([project(TABLET_AT + R * a + U * b, cam) for a, b in corners])
    bx0, by0, bx1, by1 = pts[:, 0].min(), pts[:, 1].min(), pts[:, 0].max(), pts[:, 1].max()
    bw, bh = bx1 - bx0, by1 - by0
    pc = lambda x, y: f'left {(x - bx0) / bw * 100:.3f}%; top {(y - by0) / bh * 100:.3f}%'
    print('tablet in render px', round(bx0, 1), round(by0, 1), round(bx1, 1), round(by1, 1), 'aspect', round(bw / bh, 4))
    print(f'fp css: width {W / bw * 100:.3f}%; height {H / bh * 100:.3f}%; left {-bx0 / bw * 100:.3f}%; top {-by0 / bh * 100:.3f}%;')
    print(f'walkie css: {pc(wx0, wy0)}; width {(wx1 - wx0) / bw * 100:.3f}%; height {(wy1 - wy0) / bh * 100:.3f}%;')
    print(f'thumb css: {pc(tx0, ty0)}; width {(tx1 - tx0) / bw * 100:.3f}%; height {(ty1 - ty0) / bh * 100:.3f}%;'
          f' tip at {pc(tipp[0], tipp[1])} (tip in sprite {(tipp[0] - tx0) / (tx1 - tx0) * 100:.1f}% {(tipp[1] - ty0) / (ty1 - ty0) * 100:.1f}%)')


main()
