#!/usr/bin/env python3
"""Filmstrip of posed figures from any camera, for reviewing animation by eye:
scripts/anim_sheet.py <out.png> <spec>...

spec: anything `stance_dump` takes (`<pose>[+<upper>][@<phase>[~<dir>]]`, a blend `<a>><b>%<w>`, a leg lag `^<rad>`),
then optionally `*<heading rad>` to turn the figure that way (positive to his left) and `#<dy>` to draw it that far ahead
(m), so a crawl, a step or a turn drawn where it really is shows whether planted parts stay put on the ground grid.

VIEWS="top,side,front,back,q" (default) picks the rows; a view is a name or `<azimuth deg>:<elevation deg>` (azimuth 0
looks at his right side with forward to the right, 90 at his face, -90 at his back). Every column of a row shares one
frame, so motion between columns is real motion. The ground grid is 0.25 m; anything below the ground is outlined red.
LABELS="a,b,..." names the columns; TITLE sets a title; ZMAX=<m> draws only what is below that height.
"""
import os, re, subprocess, sys
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection, LineCollection

NAMED = {'top': (0, 90), 'side': (0, 0), 'front': (90, 0), 'back': (-90, 0), 'q': (-40, 30), 'left': (180, 0)}
out, specs = sys.argv[1], sys.argv[2:]
views = [NAMED[v] if v in NAMED else tuple(float(x) for x in v.split(':'))
         for v in os.environ.get('VIEWS', 'top,side,front,back,q').split(',')]
view_names = os.environ.get('VIEWS', 'top,side,front,back,q').split(',')
labels = os.environ.get('LABELS', '').split(',') if os.environ.get('LABELS') else specs
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
raw = '/tmp/anim_sheet.bin'
base, shift, turn = [], [], []
for s in specs:
    a, _, dy = s.partition('#')
    a, _, hd = a.partition('*')
    base.append(a); shift.append(float(dy) if dy else 0.0); turn.append(float(hd) if hd else 0.0)
subprocess.run(['cargo', 'run', '-q', '--release', '-p', 'scalia-render', '--example', 'stance_dump', '--', raw, *base],
               cwd=root, check=True)
d = np.fromfile(raw, dtype='<f4').reshape(len(specs), -1, 4)
src = open(os.path.join(root, 'crates/scalia-render/src/gpu.rs')).read()
pal = src[src.index('PALETTE'):]
pal = pal[pal.index('= [') + 3:]
pal = [tuple(float(x) for x in m) for m in re.findall(r'\[\s*([0-9.]+),\s*([0-9.]+),\s*([0-9.]+)\s*\]', pal[:pal.index('];')])]
col = lambda c: pal[int(c) % 32] if int(c) % 32 < len(pal) else (0.5, 0.5, 0.5)
light = np.array([0.4, -0.5, 0.75]); light /= np.linalg.norm(light)


def basis(az, el):
    az, el = np.radians(az), np.radians(el)
    dcam = np.array([np.cos(el) * np.cos(az), np.cos(el) * np.sin(az), np.sin(el)])
    f = -dcam
    r = np.array([-np.sin(az), np.cos(az), 0.0])
    u = np.cross(r, f)
    return r, u, f


tris = []
for i in range(len(specs)):
    t = d[i].reshape(-1, 3, 4).copy()
    c, sn = np.cos(turn[i]), np.sin(turn[i])
    x, y = t[:, :, 0].copy(), t[:, :, 1].copy()
    t[:, :, 0], t[:, :, 1] = x * c - y * sn, x * sn + y * c
    t[:, :, 1] += shift[i]
    tris.append(t)
allpts = np.concatenate([t[:, :, :3].reshape(-1, 3) for t in tris])
gx0, gx1 = np.floor(allpts[:, 0].min() / 0.25) * 0.25 - 0.25, np.ceil(allpts[:, 0].max() / 0.25) * 0.25 + 0.25
gy0, gy1 = np.floor(allpts[:, 1].min() / 0.25) * 0.25 - 0.25, np.ceil(allpts[:, 1].max() / 0.25) * 0.25 + 0.25
grid = [((x, gy0, 0), (x, gy1, 0)) for x in np.arange(gx0, gx1 + 1e-6, 0.25)] + \
       [((gx0, y, 0), (gx1, y, 0)) for y in np.arange(gy0, gy1 + 1e-6, 0.25)]
grid = np.array(grid)

n = len(specs)
fig, axs = plt.subplots(len(views), n, figsize=(2.6 * n, 2.6 * len(views)), squeeze=False)
for row, (az, el) in enumerate(views):
    r, u, f = basis(az, el)
    proj = [np.stack([t[:, :, :3] @ r, t[:, :, :3] @ u], -1) for t in tris]
    q = np.concatenate([p.reshape(-1, 2) for p in proj])
    lo, hi = q.min(0), q.max(0)
    c = (lo + hi) / 2
    h = max(hi - lo) / 2 * 1.08
    for i in range(n):
        ax = axs[row, i]
        g = np.stack([grid @ r, grid @ u], -1)
        ax.add_collection(LineCollection(g, colors=(0.35, 0.42, 0.25, 0.5), linewidths=0.5))
        t = tris[i]
        pts = t[:, :, :3]
        depth = (pts @ f).mean(1)
        nrm = np.cross(pts[:, 1] - pts[:, 0], pts[:, 2] - pts[:, 0])
        nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-9
        order = np.argsort(-depth)
        # ZMAX=<m>: leave out everything above that height (to see the feet from above).
        if os.environ.get('ZMAX'):
            order = [k for k in order if pts[k, :, 2].min() < float(os.environ['ZMAX'])]
        polys, cols, edges = [], [], []
        for k in order:
            shade = 0.45 + 0.55 * abs(float(nrm[k] @ light))
            polys.append(proj[i][k])
            cols.append(np.clip(np.array(col(t[k, 0, 3])) * shade * 1.5, 0, 1))
            edges.append((0.85, 0.05, 0.05, 1.0) if pts[k, :, 2].min() < -0.01 else (0, 0, 0, 0.12))
        ax.add_collection(PolyCollection(polys, facecolors=cols, edgecolors=edges, linewidths=0.35))
        ax.set_xlim(c[0] - h, c[0] + h); ax.set_ylim(c[1] - h, c[1] + h)
        ax.set_aspect('equal'); ax.axis('off')
        if row == 0:
            ax.set_title(labels[i] if i < len(labels) else specs[i], fontsize=8)
        if i == 0:
            ax.text(0.0, 0.98, view_names[row], transform=ax.transAxes, fontsize=8, va='top', color='#203040')
if os.environ.get('TITLE'):
    fig.suptitle(os.environ['TITLE'], fontsize=10)
plt.tight_layout()
plt.savefig(out, dpi=int(os.environ.get('DPI', '90')), facecolor='#b8c4d0')
