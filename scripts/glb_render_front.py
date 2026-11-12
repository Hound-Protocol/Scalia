"""Renders a textured GLB face-on (orthographic, along its thinnest axis) to a transparent PNG, and reports where
a sub-rectangle of its texture (the screen) lands in the image. Software rasteriser: no GPU needed.

Usage: python3 -I scripts/glb_render_front.py model.glb out.png width side(+1|-1) [u0 v0 u1 v1]
"""
import io, json, struct, sys
import numpy as np
from PIL import Image


def load(path):
    b = open(path, 'rb').read()
    jl = struct.unpack('<I', b[12:16])[0]
    j = json.loads(b[20:20 + jl])
    off = 20 + jl
    bl = struct.unpack('<I', b[off:off + 4])[0]
    return j, b[off + 8:off + 8 + bl]


def acc(j, bin_, i):
    a = j['accessors'][i]; v = j['bufferViews'][a['bufferView']]
    n = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}[a['type']]
    dt = {5126: np.float32, 5125: np.uint32, 5123: np.uint16}[a['componentType']]
    start = v.get('byteOffset', 0) + a.get('byteOffset', 0)
    stride = v.get('byteStride', 0); item = n * np.dtype(dt).itemsize
    if stride and stride != item:
        raw = np.frombuffer(bin_, np.uint8, a['count'] * stride, start).reshape(a['count'], stride)[:, :item]
        return np.frombuffer(raw.tobytes(), dt).reshape(a['count'], n).astype(float)
    return np.frombuffer(bin_, dt, a['count'] * n, start).reshape(a['count'], n).astype(float)


def image(j, bin_, k):
    v = j['bufferViews'][j['images'][k]['bufferView']]
    data = bin_[v.get('byteOffset', 0):v.get('byteOffset', 0) + v['byteLength']]
    return np.asarray(Image.open(io.BytesIO(data)).convert('RGB'), float) / 255.0


def main():
    path, out, width, side = sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4])
    rect_uv = [float(x) for x in sys.argv[5:9]] if len(sys.argv) > 8 else None
    j, bin_ = load(path)
    base = image(j, bin_, 0)
    occ = image(j, bin_, 1)[:, :, 0]
    tris_p, tris_uv = [], []
    for m in j['meshes']:
        for p in m['primitives']:
            pos = acc(j, bin_, p['attributes']['POSITION'])
            uv = acc(j, bin_, p['attributes']['TEXCOORD_0'])
            idx = acc(j, bin_, p['indices']).astype(int).reshape(-1, 3)
            tris_p.append(pos[idx]); tris_uv.append(uv[idx])
    P = np.concatenate(tris_p); UV = np.concatenate(tris_uv)
    lo, hi = P.reshape(-1, 3).min(0), P.reshape(-1, 3).max(0)
    thin = int(np.argmin(hi - lo))
    axes = [a for a in range(3) if a != thin]
    # Landscape: the longer remaining axis runs across.
    if (hi - lo)[axes[0]] < (hi - lo)[axes[1]]:
        axes = axes[::-1]
    ax_u, ax_v = axes
    ss = 2
    W = width * ss
    scale = W / (hi[ax_u] - lo[ax_u]) * 0.96
    H = int((hi[ax_v] - lo[ax_v]) * scale / 0.96 * 0.96 + 0.04 * W)
    pad = 0.02 * W
    def proj(p):
        # Looking from +side along the thin axis; image x follows ax_u (mirrored when looking from -side).
        x = (p[..., ax_u] - lo[ax_u]) * scale + pad
        if side < 0:
            x = W - x
        y = (hi[ax_v] - p[..., ax_v]) * scale + pad
        return x, y
    rgb = np.zeros((H, W, 3)); alpha = np.zeros((H, W)); depth = np.full((H, W), -np.inf)
    light = np.array([0.35, 0.55, 0.75]); light /= np.linalg.norm(light)
    for t, uv in zip(P, UV):
        x, y = proj(t)
        z = t[:, thin] * side
        x0, x1 = int(max(0, np.floor(x.min()))), int(min(W - 1, np.ceil(x.max())))
        y0, y1 = int(max(0, np.floor(y.min()))), int(min(H - 1, np.ceil(y.max())))
        if x1 < x0 or y1 < y0:
            continue
        gx, gy = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
        d = (y[1] - y[2]) * (x[0] - x[2]) + (x[2] - x[1]) * (y[0] - y[2])
        if abs(d) < 1e-9:
            continue
        w0 = ((y[1] - y[2]) * (gx - x[2]) + (x[2] - x[1]) * (gy - y[2])) / d
        w1 = ((y[2] - y[0]) * (gx - x[2]) + (x[0] - x[2]) * (gy - y[2])) / d
        w2 = 1 - w0 - w1
        inside = (w0 >= -1e-6) & (w1 >= -1e-6) & (w2 >= -1e-6)
        if not inside.any():
            continue
        zz = w0 * z[0] + w1 * z[1] + w2 * z[2]
        sub = depth[y0:y1 + 1, x0:x1 + 1]
        win = inside & (zz > sub)
        if not win.any():
            continue
        u = w0 * uv[0, 0] + w1 * uv[1, 0] + w2 * uv[2, 0]
        v = w0 * uv[0, 1] + w1 * uv[1, 1] + w2 * uv[2, 1]
        tx = np.clip((u % 1.0) * base.shape[1], 0, base.shape[1] - 1).astype(int)
        ty = np.clip((v % 1.0) * base.shape[0], 0, base.shape[0] - 1).astype(int)
        n = np.cross(t[1] - t[0], t[2] - t[0]); n /= np.linalg.norm(n) + 1e-12
        nv = np.zeros(3); nv[thin] = n[thin] * side; nv[ax_u] = n[ax_u] * (1 if side > 0 else -1); nv[ax_v] = n[ax_v]
        shade = 0.55 + 0.45 * abs(float(np.dot(np.array([nv[ax_u], nv[ax_v], nv[thin]]), light)))
        col = base[ty, tx] * occ[ty, tx][..., None] * shade
        sub[win] = zz[win]
        rgb[y0:y1 + 1, x0:x1 + 1][win] = col[win]
        alpha[y0:y1 + 1, x0:x1 + 1][win] = 1.0
    img = np.dstack([rgb, alpha])
    img = img.reshape(H // ss, ss, W // ss, ss, 4).mean((1, 3)) if H % ss == 0 else img[:H - H % ss].reshape(H // ss, ss, W // ss, ss, 4).mean((1, 3))
    Image.fromarray((np.clip(img, 0, 1) * 255).astype(np.uint8), 'RGBA').save(out)
    print('image', img.shape[1], img.shape[0], 'thin axis', thin, 'across', ax_u, 'up', ax_v)
    if rect_uv:
        # Where the screen's texture rectangle lands: the bounding box of every triangle point inside it.
        u0, v0, u1, v1 = rect_uv
        pts = []
        for t, uv in zip(P, UV):
            if t[:, thin].mean() * side < (hi[thin] if side > 0 else -lo[thin]) - 0.5 * (hi - lo)[thin]:
                continue
            m = (uv[:, 0] >= u0) & (uv[:, 0] <= u1) & (uv[:, 1] >= v0) & (uv[:, 1] <= v1)
            if m.all():
                x, y = proj(t)
                pts += list(zip(x / ss, y / ss))
        if pts:
            a = np.array(pts)
            print('screen px', a[:, 0].min().round(1), a[:, 1].min().round(1), a[:, 0].max().round(1), a[:, 1].max().round(1))


if __name__ == '__main__':
    main()
