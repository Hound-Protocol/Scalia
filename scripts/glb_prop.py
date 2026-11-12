"""Converts a small textured GLB prop (the command tablet) to our figure format: each triangle takes the average of
its texture colour, snapped to the nearest of a few palette entries. The prop is laid out face-on: x to its right,
y up its face, z out of its face, centred, scaled to `width` metres across.

Usage: python3 -I scripts/glb_prop.py model.glb out.fig width
"""
import os, struct, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import glb_render_front as R  # noqa: E402

# Candidate kit colours (palette index: sRGB), olive drab to near black.
CANDIDATES = {10: (0.25, 0.40, 0.20), 11: (0.32, 0.48, 0.24), 13: (0.48, 0.44, 0.30), 18: (0.17, 0.18, 0.19),
              21: (0.25, 0.26, 0.27), 25: (0.10, 0.11, 0.12), 2: (0.55, 0.58, 0.60)}


def main():
    path, out, width = sys.argv[1], sys.argv[2], float(sys.argv[3])
    j, bin_ = R.load(path)
    base = R.image(j, bin_, 0)
    P, UV = [], []
    for m in j['meshes']:
        for p in m['primitives']:
            pos = R.acc(j, bin_, p['attributes']['POSITION'])
            uv = R.acc(j, bin_, p['attributes']['TEXCOORD_0'])
            idx = R.acc(j, bin_, p['indices']).astype(int).reshape(-1, 3)
            P.append(pos[idx]); UV.append(uv[idx])
    P = np.concatenate(P); UV = np.concatenate(UV)
    lo, hi = P.reshape(-1, 3).min(0), P.reshape(-1, 3).max(0)
    mid = (lo + hi) / 2
    k = width / (hi[2] - lo[2])
    # Face-on frame (see glb_render_front: the face looks along +x, its right is -z, its up +y).
    loc = np.stack([-(P[..., 2] - mid[2]), P[..., 1] - mid[1], P[..., 0] - mid[0]], -1) * k
    keys = list(CANDIDATES); cols = np.array([CANDIDATES[c] for c in keys])
    with open(out, 'wb') as f:
        n = 0
        for t, uv in zip(loc, UV):
            # Average the texture over a few points of the triangle.
            ws = [(1/3, 1/3, 1/3), (0.6, 0.2, 0.2), (0.2, 0.6, 0.2), (0.2, 0.2, 0.6)]
            c = np.zeros(3)
            for w in ws:
                u, v = (np.array(w) @ uv) % 1.0
                c += base[min(int(v * base.shape[0]), base.shape[0] - 1), min(int(u * base.shape[1]), base.shape[1] - 1)]
            c /= len(ws)
            pal = keys[int(np.argmin(((cols - c) ** 2).sum(1)))]
            for p in t:
                f.write(struct.pack('<4f', p[0], p[1], p[2], float(pal)))
            n += 1
    print(n, 'triangles; size', (loc.reshape(-1, 3).max(0) - loc.reshape(-1, 3).min(0)).round(3))


main()
