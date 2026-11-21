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
def col(c):
    k = int(c) % 32
    return {26: (0.8, 0.1, 0.1)}.get(k, (0.16 + 0.02 * (k % 5), 0.17, 0.19)) if k not in (27,) else (0.86, 0.70, 0.55)
light = np.array([0.4, -0.5, 0.75]); light /= np.linalg.norm(light)
fig, axs = plt.subplots(2, len(specs), figsize=(2.4 * len(specs), 6.4))
axs = np.array(axs).reshape(2, -1)
for i, spec in enumerate(specs):
    tri = d[i].reshape(-1, 3, 4)
    pts, cc = tri[:, :, :3], tri[:, 0, 3]
    for row, yaw in enumerate([np.pi / 2, np.pi / 4]):
        # Camera turned round the figure: row 0 a pure side view (forward to the right), row 1 three-quarter.
        c, s = np.cos(yaw), np.sin(yaw)
        R = np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])
        p = pts @ R.T
        nrm = np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0]); nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-9
        vis = nrm[:, 0] < 0  # facing the camera on -x... looking along +x
        depth = p[:, :, 0].mean(axis=1)
        order = np.argsort(-depth)
        polys, cols = [], []
        for t in order:
            if not vis[t]: continue
            shade = 0.45 + 0.55 * max(0.0, float(nrm[t] @ (R @ light)))
            base = np.array(col(cc[t]))
            polys.append(p[t][:, [1, 2]]); cols.append(np.clip(base * shade * 1.6, 0, 1))
        ax = axs[row, i]
        ax.add_collection(PolyCollection(polys, facecolors=cols, edgecolors='none'))
        ax.axhline(0, color='#7a6a48', lw=1)
        ax.set_xlim(-0.9, 1.3); ax.set_ylim(-0.05, 1.95); ax.set_aspect('equal'); ax.axis('off')
        if row == 0: ax.set_title(spec, fontsize=9)
plt.tight_layout(); plt.savefig(out, dpi=90, facecolor='#d9d4c4')
