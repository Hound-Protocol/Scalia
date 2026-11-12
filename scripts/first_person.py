"""Renders what the commander sees looking down at his tablet: his own body, arms and gloved hands, from his eyes,
in two layers split by the tablet's plane. The back layer goes behind the high-resolution tablet close-up and the
front layer (fingers and thumbs over its edges) on top. Prints where the tablet's image falls, as CSS.

Input: the soldier in the tablet pose (examples/pose_dump 10) and the tablet mesh (assets/pda.fig, face-on frame).
Usage: python3 -I scripts/first_person.py posed.fig pda.fig out_dir
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
IMG_W_PX, IMG_MARGIN_PX = 2658, 32
SCREEN_X = (0.08542, 0.91687)  # (no longer used for a front layer; kept for reference)
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


def main():
    posed, pda, out = sys.argv[1], sys.argv[2], sys.argv[3]
    d = np.frombuffer(open(posed, 'rb').read(), np.float32).reshape(-1, 3, 4).astype(float)
    tris, codes = d[:, :, :3], d[:, 0, 3]
    parts = (codes // 32).astype(int)
    keep = ~np.isin(parts, [10, 9])  # not his own head; the rifle is slung behind
    tris, codes, parts = tris[keep], codes[keep], parts[keep]
    cam = camera()
    s, c = np.sin(TILT), np.cos(TILT)
    R, U, N = np.array([1.0, 0, 0]), np.array([0, s, c]), np.array([0, -c, s])
    # The tablet's plane, for splitting the layers: depth along each pixel's ray where it meets the plane.
    F = (W / 2) / np.tan(HFOV / 2)
    gx, gy = np.meshgrid(np.arange(W) + 0.5, np.arange(H) + 0.5)
    r, u, f = cam
    rays = ((gx - W / 2) / F)[..., None] * r + (-(gy - H / 2) / F)[..., None] * u + f
    denom = rays @ N
    t_plane = ((TABLET_AT - EYE) @ N) / np.where(np.abs(denom) < 1e-9, 1e-9, denom)
    z_plane = t_plane  # rays have unit forward component, so t is the depth along f
    rgb = np.zeros((H, W, 3)); alpha = np.zeros((H, W)); depth = np.full((H, W), np.inf)
    light = np.array([0.3, 0.4, 0.86]); light /= np.linalg.norm(light)
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
        # Perspective-correct depth.
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
    front = alpha * (depth < z_plane - 0.004)
    # Tablet image extent: the model plus a margin of IMG_MARGIN_PX pixels all round (glb_render_front, widen).
    pd = np.frombuffer(open(pda, 'rb').read(), np.float32).reshape(-1, 4)[:, :3].astype(float)
    lo, hi = pd.min(0), pd.max(0)
    width = hi[0] - lo[0]
    mx = width * IMG_MARGIN_PX / (IMG_W_PX - 2 * IMG_MARGIN_PX)
    corners = [(lo[0] - mx, hi[1] + mx), (hi[0] + mx, hi[1] + mx), (hi[0] + mx, lo[1] - mx), (lo[0] - mx, lo[1] - mx)]
    pts = np.array([project(TABLET_AT + R * a + U * b, cam) for a, b in corners])
    bx0, by0, bx1, by1 = pts[:, 0].min(), pts[:, 1].min(), pts[:, 0].max(), pts[:, 1].max()
    # Fingers may lie over the frame, never over the display.
    sx0, sx1 = bx0 + SCREEN_X[0] * (bx1 - bx0), bx0 + SCREEN_X[1] * (bx1 - bx0)
    sy0, sy1 = by0 + 0.1194 * (by1 - by0), by0 + 0.8771 * (by1 - by0)
    on_screen = (gx >= sx0) & (gx <= sx1) & (gy >= sy0) & (gy <= sy1)
    front = front * ~on_screen
    # Back layer: everything (the tablet image covers its own area); front layer: what is in front of its plane.
    back = np.dstack([rgb, alpha])
    fr = np.dstack([rgb, front])
    Image.fromarray((back * 255).astype(np.uint8), 'RGBA').save(f'{out}/fp_back.webp', 'WEBP', quality=90, method=6)
    Image.fromarray((fr * 255).astype(np.uint8), 'RGBA').save(f'{out}/fp_front.webp', 'WEBP', quality=90, method=6)
    prev = np.dstack([rgb, alpha])
    Image.fromarray((prev * 255).astype(np.uint8), 'RGBA').save(f'{out}/fp_preview.png')
    # CSS for layers placed inside #tablet (whose box is the tablet image): size and offset in % of that box.
    bw, bh = bx1 - bx0, by1 - by0
    print('tablet in render px', round(bx0, 1), round(by0, 1), round(bx1, 1), round(by1, 1), 'aspect', round(bw / bh, 4))
    print(f'css: width {W / bw * 100:.3f}%; height {H / bh * 100:.3f}%; left {-bx0 / bw * 100:.3f}%; top {-by0 / bh * 100:.3f}%;')


main()
