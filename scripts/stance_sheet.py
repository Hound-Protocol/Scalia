#!/usr/bin/env python3
"""Contact sheet of stances, side and three-quarter views: scripts/stance_sheet.py <out.png> <pose[+upper]>...
Poses are ids as in pose.rs (e.g. 0 stand, 6 stand aim, 14 crouch aim, 30 ready); each is posed as drawn in game
(layer, then grip) by the stance_dump example."""
import os, subprocess, sys
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection

out, specs = sys.argv[1], sys.argv[2:]
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
raw = '/tmp/stance.bin'
# DUMP=skin_dump draws the skinned soldier from clip@seconds specs instead (AN-03).
subprocess.run(['cargo', 'run', '-q', '--release', '-p', 'scalia-render', '--example', os.environ.get('DUMP', 'stance_dump'), '--', raw, *specs], cwd=root, check=True)
d = np.fromfile(raw, dtype='<f4').reshape(len(specs), -1, 4)
import re as _re
# The game's palette (gpu.rs PALETTE), so the sheet shows the kit as the game colours it.
_src = open(os.path.join(root, 'crates/scalia-render/src/gpu.rs')).read()
_pal = _src[_src.index('PALETTE'):]
_pal = _pal[_pal.index('= [') + 3:]
_pal = [tuple(float(x) for x in m) for m in _re.findall(r'\[\s*([0-9.]+),\s*([0-9.]+),\s*([0-9.]+)\s*\]', _pal[:_pal.index('];')])]
def col(c):
    k = int(c) % 32
    return _pal[k] if k < len(_pal) else (0.5, 0.5, 0.5)
light = np.array([0.4, -0.5, 0.75]); light /= np.linalg.norm(light)
fig, axs = plt.subplots(4, len(specs), figsize=(2.2 * len(specs), 9.2))
axs = np.array(axs).reshape(4, -1)
for i, spec in enumerate(specs):
    tri = d[i].reshape(-1, 3, 4)
    pts, cc = tri[:, :, :3], tri[:, 0, 3]
    # Rows: front (facing the camera), side (forward to the right), back, top (forward up the page).
    for row, (view, yaw) in enumerate([('front', np.pi / 2), ('side', 0.0), ('back', -np.pi / 2), ('top', None)]):
        if yaw is None:
            p = pts.copy()
            nrm = np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0]); nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-9
            vis, order, L, xy = np.abs(nrm[:, 2]) > 0.0, np.argsort(p[:, :, 2].mean(axis=1)), light, [0, 1]
        else:
            # Camera on -x looking along +x; the screen is y (right) and z (up).
            c, s = np.cos(yaw), np.sin(yaw)
            R = np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])
            p = pts @ R.T
            nrm = np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0]); nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-9
            vis, order, L, xy = np.abs(nrm[:, 0]) > 0.0, np.argsort(-p[:, :, 0].mean(axis=1)), R @ light, [1, 2]
        polys, cols = [], []
        for t in order:
            if not vis[t]: continue
            shade = 0.45 + 0.55 * abs(float(nrm[t] @ L))
            base = np.array(col(cc[t]))
            polys.append(p[t][:, xy]); cols.append(np.clip(base * shade * 1.6, 0, 1))
        ax = axs[row, i]
        ax.add_collection(PolyCollection(polys, facecolors=cols, edgecolors='none'))
        if yaw is None:
            ax.set_xlim(-1.1, 1.1); ax.set_ylim(-1.0, 1.4)
        else:
            ax.axhline(0, color='#7a6a48', lw=1); ax.set_xlim(-1.1, 1.1); ax.set_ylim(-0.05, 1.95)
        ax.set_aspect('equal'); ax.axis('off')
        if row == 0: ax.set_title(spec, fontsize=9)
        if i == 0: ax.text(-1.05, 1.85 if yaw is not None else 1.3, view, fontsize=8)
plt.tight_layout(); plt.savefig(out, dpi=90, facecolor='#d9d4c4')
