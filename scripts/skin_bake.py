"""Bakes the rigged soldier and its animation clips for the GPU skinning path (AN-03):
scripts/skin_bake.py <rig.fbx> <colours.obj> <clips.txt> <packs dir> <out dir>

<rig.fbx>: the Mixamo-rigged soldier (with skin). <colours.obj>: the same mesh as OBJ with its materials, for
colours (matched triangle by triangle). <clips.txt>: one clip per line, `name | path under <packs dir> | loop or
once`. Writes <out dir>/rigged.skin and <out dir>/anims.bin (formats in crates/scalia-render/src/skin.rs).

Frames: the figure frame (x right, y forward, z up, metres; Mixamo has y up and faces +z). Joint transforms
are stored local to their parent joint (root: the figure frame), rotations as i16 quaternions, 30 per second,
plus the root's position. Horizontal root travel is taken out (the game moves the soldier) and reported per
clip, so the game can match playback to speed. Raw clips stay out of the repo (CONTRIBUTING)."""
import os, struct, sys, tempfile
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from clip_ref import Clip, export_gltf  # noqa: E402

HEIGHT = 1.79   # the game's soldier, boots to helmet top (m)
FPS = 30.0
A = np.array([[-1.0, 0, 0], [0, 0, 1.0], [0, 1.0, 0]])  # Mixamo axes to the figure frame (a rotation)
# Material to kit palette index (crates/scalia-render/src/mesh.rs `kit`), part 1 (a body part).
KIT = {'Green': 16, 'Grey': 18, 'Shirt': 19, 'Bags': 20, 'Skin': 27, 'Mags': 28}

def gltf(path):
    tmp = tempfile.mkdtemp(); g = os.path.join(tmp, 'c.gltf'); export_gltf(path, g); return Clip(g)

def accessor(c, i):
    a = c.g['accessors'][i]; v = c.g['bufferViews'][a['bufferView']]
    n = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[a['type']]
    fmt, size = {5121: ('B', 1), 5123: ('H', 2), 5125: ('I', 4), 5126: ('f', 4)}[a['componentType']]
    off = v.get('byteOffset', 0) + a.get('byteOffset', 0); st = v.get('byteStride', size * n)
    d = c.bufs[v['buffer']]
    return np.array([struct.unpack_from('<%d%s' % (n, fmt), d, off + k * st) for k in range(a['count'])])

def rigid(m):
    """Rotation (scale removed) and translation of a similarity transform."""
    r = m[:3, :3]; s = np.cbrt(np.linalg.det(r)); return r / s, m[:3, 3], s

def quat(r):
    t = np.trace(r)
    if t > 0:
        s = np.sqrt(t + 1) * 2; q = [(r[2, 1] - r[1, 2]) / s, (r[0, 2] - r[2, 0]) / s, (r[1, 0] - r[0, 1]) / s, s / 4]
    elif r[0, 0] > r[1, 1] and r[0, 0] > r[2, 2]:
        s = np.sqrt(1 + r[0, 0] - r[1, 1] - r[2, 2]) * 2; q = [s / 4, (r[0, 1] + r[1, 0]) / s, (r[0, 2] + r[2, 0]) / s, (r[2, 1] - r[1, 2]) / s]
    elif r[1, 1] > r[2, 2]:
        s = np.sqrt(1 + r[1, 1] - r[0, 0] - r[2, 2]) * 2; q = [(r[0, 1] + r[1, 0]) / s, s / 4, (r[1, 2] + r[2, 1]) / s, (r[0, 2] - r[2, 0]) / s]
    else:
        s = np.sqrt(1 + r[2, 2] - r[0, 0] - r[1, 1]) * 2; q = [(r[0, 2] + r[2, 0]) / s, (r[1, 2] + r[2, 1]) / s, s / 4, (r[1, 0] - r[0, 1]) / s]
    q = np.array(q); return q / np.linalg.norm(q)

def world_mats(c, t):
    cache = {}
    def w(i):
        if i not in cache: cache[i] = (w(c.parent[i]) @ c.local(i, t)) if i in c.parent else c.local(i, t)
        return cache[i]
    return {k: w(i) for k, i in c.idx.items()}

def main():
    rig_path, obj_path, clips_path, packs, out = sys.argv[1:6]
    rig = gltf(rig_path)
    skin = rig.g['skins'][0]
    names = [rig.nodes[j]['name'].replace('mixamorig:', '') for j in skin['joints']]
    J = len(names)
    # Parent joint of each joint (nearest skinned ancestor), the root's is -1.
    jset = {j: k for k, j in enumerate(skin['joints'])}
    parents = []
    for j in skin['joints']:
        p = rig.parent.get(j)
        while p is not None and p not in jset: p = rig.parent.get(p)
        parents.append(jset[p] if p is not None else -1)
    def depth(k):
        d = 0
        while parents[k] >= 0: k = parents[k]; d += 1
        return d
    order = sorted(range(J), key=lambda k: (depth(k), k))
    assert all(parents[k] < 0 or order.index(parents[k]) < order.index(k) for k in range(J))
    # Reorder joints so every parent comes before its children (the GPU walks them in order).
    remap = {old: new for new, old in enumerate(order)}
    names = [names[k] for k in order]; parents = [remap[parents[k]] if parents[k] >= 0 else -1 for k in order]
    ibm = accessor(rig, skin['inverseBindMatrices']).reshape(-1, 4, 4).transpose(0, 2, 1)[order]

    prim = rig.g['meshes'][0]['primitives'][0]
    pos = accessor(rig, prim['attributes']['POSITION'])
    jnt = np.vectorize(lambda x: remap[int(x)])(accessor(rig, prim['attributes']['JOINTS_0']))
    wts = accessor(rig, prim['attributes']['WEIGHTS_0'])
    tri = accessor(rig, prim['indices']).reshape(-1, 3).astype(int)
    s_mesh = HEIGHT / (pos[:, 1].max() - pos[:, 1].min())
    ground = pos[:, 1].min()
    to_fig = lambda v: s_mesh * (A @ (v - np.array([0, ground, 0])))
    # Bind frames of the joints in the figure frame (rigid).
    bind = []
    for k in range(J):
        r, t, _ = rigid(np.linalg.inv(ibm[k])); bind.append((A @ r @ A.T, to_fig(t)))

    # Colours: each triangle takes the material of the nearest OBJ face (meshes matched by their bounds).
    from scipy.spatial import cKDTree
    ov, of, om, cur = [], [], [], None
    for line in open(obj_path):
        if line.startswith('v '): ov.append([float(x) for x in line.split()[1:4]])
        elif line.startswith('usemtl'): cur = line.split()[1]
        elif line.startswith('f '):
            ids = [int(x.split('/')[0]) - 1 for x in line.split()[1:]]
            for a in range(1, len(ids) - 1): of.append([ids[0], ids[a], ids[a + 1]]); om.append(cur)
    ov = np.array(ov); oc = ov[np.array(of)].mean(1)
    norm = lambda p: (p - p.min(0)) / (p.max(0) - p.min(0))
    tc = norm(pos)[tri].mean(1)
    d, near = cKDTree(norm(ov)[np.array(of)].mean(1)).query(tc)
    print(f'colours: triangles matched within {np.percentile(d, 95):.4f} (95 %) of the bounds')
    cols = [KIT.get(om[i], 18) for i in near]

    # Mesh: a triangle list. Vertex: position (3 f32), colour code (f32), 4 joints (u8), 4 weights (u8, sum 255).
    vb = bytearray()
    for t, col in zip(tri, cols):
        for i in t:
            w = np.clip(wts[i], 0, None); w = w / w.sum() if w.sum() > 0 else np.array([1.0, 0, 0, 0])
            wq = np.round(w * 255).astype(int); wq[np.argmax(wq)] += 255 - wq.sum()
            vb += struct.pack('<3ff4B4B', *to_fig(pos[i]), float(col + 32), *[int(x) for x in jnt[i]], *[int(x) for x in wq])
    with open(os.path.join(out, 'rigged.skin'), 'wb') as f:
        f.write(b'SKN1' + struct.pack('<II', len(tri) * 3, J) + vb)
    print(f'mesh: {len(tri)} triangles, {J} joints, {len(vb) / 1024:.0f} KiB')

    # Clips. World scale of the animation against the mesh: hips to head at bind vs in the rig's own clip.
    rw = world_mats(rig, 0.0)
    hb = np.linalg.norm(bind[names.index('Head')][1] - bind[names.index('Hips')][1])
    hw = np.linalg.norm(rw['Head'][:3, 3] - rw['Hips'][:3, 3])
    s_world = hb / hw
    clips = []
    for line in open(clips_path):
        line = line.split('#')[0].strip()
        if not line: continue
        name, rel, mode = [x.strip() for x in line.split('|')]
        c = gltf(os.path.join(packs, rel))
        missing = [n for n in names if n not in c.idx]
        if missing: sys.exit(f'{name}: missing joints {missing[:4]}')
        loop = mode == 'loop'
        n = max(2, int(round(c.duration * FPS)) + (0 if loop else 1))
        times = [min(c.duration, k / FPS) for k in range(n)]
        frames = []
        for t in times:
            wm = world_mats(c, t); fr = []
            for k, nm in enumerate(names):
                r, tt, _ = rigid(wm[nm]); fr.append((A @ r @ A.T, s_world * (A @ tt)))
            frames.append(fr)
        root = np.array([f[0][1] for f in frames])
        # Travel over the clip (a loop's last frame leads back into its first), taken out evenly.
        end = rigid(world_mats(c, c.duration)['Hips'])[1] if loop else None
        travel = (s_world * (A @ end) - root[0]) if loop else (root[-1] - root[0])
        travel[2] = 0.0
        span = n if loop else max(n - 1, 1)
        rq, rt = [], []
        for k, fr in enumerate(frames):
            loc = []
            for j in range(J):
                r, t = fr[j]
                if parents[j] >= 0:
                    pr, pt = fr[parents[j]]; r, t = pr.T @ r, pr.T @ (t - pt)
                else:
                    t = t - travel * (k / span) - np.array([root[0][0], root[0][1], 0.0])
                loc.append((quat(r), t))
            rq.append([q for q, _ in loc]); rt.append(loc[0][1])
        rq = np.array(rq)
        for k in range(1, n):  # keep each joint's quaternion on one hemisphere so frames interpolate the short way
            flip = np.sum(rq[k] * rq[k - 1], axis=1) < 0; rq[k][flip] *= -1
        clips.append((name, loop, c.duration, travel[:2], rq, np.array(rt)))
        print(f'{name:16} {"loop" if loop else "once"} {c.duration:5.2f} s {n:3} frames, travel {travel[0]:+.2f} {travel[1]:+.2f} m, hips {root[:, 2].mean():.2f} m')

    # Bind-local translations (bone offsets) for every joint but the root, which clips carry per frame.
    bl = []
    for j in range(J):
        r, t = bind[j]
        if parents[j] >= 0: pr, pt = bind[parents[j]]; t = pr.T @ (t - pt)
        bl.append(t)
    with open(os.path.join(out, 'anims.bin'), 'wb') as f:
        f.write(b'ANM1' + struct.pack('<II', J, len(clips)))
        for j in range(J):
            r, t = bind[j]
            f.write(struct.pack('<32si4f3f3f', names[j].encode()[:32], parents[j], *quat(r), *t, *bl[j]))
        data = bytearray()
        for name, loop, dur, travel, rq, rt in clips:
            f.write(struct.pack('<32sIIff2fI', name.encode()[:32], len(rq), int(loop), FPS, dur, *travel, len(data)))
            for k in range(len(rq)):
                data += struct.pack('<3f', *rt[k])
                data += struct.pack('<%dh' % (4 * J), *np.round(np.clip(rq[k], -1, 1) * 32767).astype(int).flatten())
        f.write(data)
    print(f'anims: {len(clips)} clips, {len(data) / 1024:.0f} KiB of frames')

if __name__ == '__main__':
    main()
