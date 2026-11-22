#!/usr/bin/env python3
"""Close-up of joints on posed figures, for checking seams by eye:
scripts/closeup.py <out.png> <view>:<z lo>:<z hi> <pose@phase>...
view is the camera's yaw in degrees (0 side with forward to the right, 90 front, -90 back, -135 back three-quarter);
z lo..z hi is the band of the figure drawn (m above the ground). One row per view given with VIEWS="v:lo:hi,...",
one column per pose. Draws flat-shaded faces with their outlines, as stance_sheet.py does, but bigger."""
import os, subprocess, sys
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection

out, specs = sys.argv[1], sys.argv[2:]
views = [tuple(float(x) for x in v.split(':')) for v in os.environ.get('VIEWS', '0:0:1.1').split(',')]
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
raw = '/tmp/closeup.bin'
subprocess.run(['cargo', 'run', '-q', '--release', '-p', 'scalia-render', '--example', 'stance_dump', '--', raw, *specs], cwd=root, check=True)
d = np.fromfile(raw, dtype='<f4').reshape(len(specs), -1, 4)
import re
src = open(os.path.join(root, 'crates/scalia-render/src/gpu.rs')).read()
pal = src[src.index('PALETTE'):]
pal = pal[pal.index('= [') + 3:]
pal = [tuple(float(x) for x in m) for m in re.findall(r'\[\s*([0-9.]+),\s*([0-9.]+),\s*([0-9.]+)\s*\]', pal[:pal.index('];')])]
col = lambda c: pal[int(c) % 32] if int(c) % 32 < len(pal) else (0.5, 0.5, 0.5)
light = np.array([0.4, -0.5, 0.75]); light /= np.linalg.norm(light)
fig, axs = plt.subplots(len(views), len(specs), figsize=(3.2 * len(specs), 3.6 * len(views)))
axs = np.array(axs).reshape(len(views), -1)
for i, spec in enumerate(specs):
    tri = d[i].reshape(-1, 3, 4)
    pts, cc = tri[:, :, :3], tri[:, 0, 3]
    for row, (yaw, lo, hi) in enumerate(views):
        c, s = np.cos(np.radians(yaw)), np.sin(np.radians(yaw))
        R = np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])
        p = pts @ R.T
        nrm = np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0]); nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-9
        polys, cols = [], []
        for t in np.argsort(-p[:, :, 0].mean(axis=1)):
            if p[t, :, 2].max() < lo or p[t, :, 2].min() > hi: continue
            shade = 0.45 + 0.55 * abs(float(nrm[t] @ (R @ light)))
            polys.append(p[t][:, [1, 2]]); cols.append(np.clip(np.array(col(cc[t])) * shade * 1.6, 0, 1))
        ax = axs[row, i]
        ax.add_collection(PolyCollection(polys, facecolors=cols, edgecolors=(0, 0, 0, 0.15), linewidths=0.3))
        q = np.concatenate(polys).reshape(-1, 2)
        q = q[(q[:, 1] >= lo) & (q[:, 1] <= hi)]
        cx = (q[:, 0].min() + q[:, 0].max()) / 2
        h = (hi - lo) / 2
        ax.set_xlim(cx - h, cx + h); ax.set_ylim(lo, hi)
        ax.axhline(0, color='#5a6a38', lw=2)
        ax.set_aspect('equal'); ax.axis('off')
        if row == 0: ax.set_title(spec, fontsize=9)
plt.tight_layout(); plt.savefig(out, dpi=110, facecolor='#9fb0c4')
