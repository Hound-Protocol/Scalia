#!/usr/bin/env python3
"""Side-view contact sheet of a gait cycle: scripts/filmstrip.py <pose> <frames> <out.png> [legs].
Dumps the posed figure with the filmstrip_dump example, then draws each frame in a row, with the ground line,
the way a reviewer would scan a walk cycle. Frames advance with the figure travelling left to right."""
import os, subprocess, sys
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection

pose, frames, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
legs = len(sys.argv) > 4
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
raw = '/tmp/filmstrip.bin'
subprocess.run(['cargo', 'run', '-q', '--release', '-p', 'scalia-render', '--example', 'filmstrip_dump', '--', pose, str(frames), raw], cwd=root, check=True)
d = np.fromfile(raw, dtype='<f4').reshape(frames, -1, 4)
pal = [[0.10, 0.12, 0.15]] * 16 + [[0.62, 0.60, 0.44], [0.49, 0.47, 0.34], [0.17, 0.18, 0.19], [0.58, 0.60, 0.62],
       [0.40, 0.38, 0.28], [0.25, 0.26, 0.27], [0.12, 0.12, 0.13], [0.13, 0.14, 0.15], [0.20, 0.21, 0.22],
       [0.10, 0.11, 0.12], [0.63, 0.61, 0.45], [0.86, 0.70, 0.55], [0.08, 0.08, 0.09], [0.17, 0.18, 0.19],
       [0.72, 0.62, 0.42], [0.8, 0.1, 0.1]]
# Foot parts (13, 14) tinted so the ankle split is visible.
top = 0.95 if legs else 1.85
fig, axs = plt.subplots(1, frames, figsize=(1.6 * frames, 4.2 if not legs else 3.2))
for k, ax in enumerate(np.atleast_1d(axs)):
    v = d[k].reshape(-1, 3, 4)
    pts = v[:, :, :3]
    # Side view: forward (y) to the right, up (z) up.
    nrm = np.cross(pts[:, 1] - pts[:, 0], pts[:, 2] - pts[:, 0])
    nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-9
    light = np.array([0.5, 0.2, 0.8]); light /= np.linalg.norm(light)
    depth = pts[:, :, 0].mean(1)
    order = np.argsort(-depth)
    cols = []
    for i in order:
        c = int(v[i, 0, 3])
        base = np.array(pal[c % 32]) * (0.45 + 0.55 * abs(nrm[i] @ light))
        part = c // 32
        if part in (13, 14): base = base * 0.5 + np.array([0.3, 0.1, 0.0])
        cols.append(np.clip(base, 0, 1))
    ax.add_collection(PolyCollection(pts[order][:, :, [1, 2]], facecolors=cols, edgecolors='none'))
    ax.axhline(0, color='k', lw=0.6)
    ax.set_xlim(-0.55, 0.55); ax.set_ylim(-0.05, top); ax.set_aspect('equal'); ax.axis('off')
    ax.set_title(f'{k / frames:.2f}', fontsize=7)
fig.savefig(out, dpi=90, bbox_inches='tight')
