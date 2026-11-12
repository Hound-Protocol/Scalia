"""Converts a rigged, T-posed soldier GLB into our figure format (crates/scalia-render/assets/*.fig).

Our figures are rigid parts posed in the shader around fixed pivots (see wgsl/common.wgsl): thighs hang from the
hips at 0.88 m, shins from the knees at 0.46 m, upper arms from the shoulders (x = +-0.238, 1.40 m), forearms from
the elbows at 1.09 m, the head nods at the neck (1.46 m), and the rifle sits in the right hand at 0.81 m. The figure
faces +y with z up, arms hanging straight down.

So the converter: skins the T-pose itself (the file's bind matrices carry a stray 19.55x scale that only the rifle
uses), swings both arms down around the shoulders, fits the joints onto our pivots with piecewise-linear maps,
gives each vertex the part of its strongest bone, maps the materials onto our kit palette, decimates the rifle and
lays it along +y in the right hand, and adds a team-colour armband on each upper arm.

Output: little-endian f32 (x, y, z, code) per vertex, three vertices per triangle.

Usage: python3 -I scripts/glb_figure.py model.glb out.fig [preview.png]
"""
import struct, sys
import numpy as np
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import glb_inspect as G  # noqa: E402

# Our rig (metres) and part ids (mesh.rs Part codes).
HIP, KNEE, SHOULDER_Z, SHOULDER_X, ELBOW, NECK, GRIP = 0.88, 0.46, 1.40, 0.238, 1.09, 1.46, 0.81
WRIST = 0.865
L_THIGH, R_THIGH, L_ARM, R_ARM, L_SHIN, R_SHIN, TORSO, RIFLE, HEAD, L_FORE, R_FORE = 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12
# Kit palette (mesh::kit).
UNIFORM, UNIFORM_DARK, VEST, STRAP, PACK, BOOT, GLOVE, BALACLAVA, LENS, GOGGLE, HELMET, SKIN, PAD, METAL, COYOTE_DARK, TEAM = \
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
HEIGHT = 1.79  # top of the helmet
# How close to the body each part is: seams go to the lower rank.
RANK = {8: 0, 10: 1, 1: 1, 2: 1, 3: 1, 4: 1, 6: 2, 7: 2, 11: 2, 12: 2, 9: 3}
RIFLE_CELL = 0.02  # vertex clustering cell for the rifle (m)


def code(pal, part):
    return float(pal + 32 * part)


def chain(names, root):
    """Joint indices of `root` and everything named after it on the same side (arm -> forearm, hand, fingers)."""
    side = root.split('_')[0].split('.')[-1]
    order = ['arm', 'forearm', 'hand', 'thumb', 'index', 'middle', 'ring', 'pinky']
    return [k for k, n in enumerate(names) if n.split('.')[0].split('_')[0] in order and f'.{side}' in n]


def piecewise(z, src, dst):
    """Piecewise-linear map through anchor pairs, extrapolating with the end segments' slopes."""
    src, dst = np.asarray(src, float), np.asarray(dst, float)
    out = np.interp(z, src, dst)
    lo, hi = z < src[0], z > src[-1]
    out[lo] = dst[0] + (z[lo] - src[0]) * (dst[1] - dst[0]) / (src[1] - src[0])
    out[hi] = dst[-1] + (z[hi] - src[-1]) * (dst[-1] - dst[-2]) / (src[-1] - src[-2])
    return out


def cluster(tris, cell):
    """Vertex-clustering decimation: snaps vertices to a grid and drops triangles that collapse."""
    v = tris.reshape(-1, 3)
    key = np.round(v / cell).astype(np.int64)
    uniq, inv = np.unique(key, axis=0, return_inverse=True)
    inv = inv.reshape(-1)
    mean = np.zeros((len(uniq), 3)); cnt = np.zeros(len(uniq))
    np.add.at(mean, inv, v); np.add.at(cnt, inv, 1)
    mean /= cnt[:, None]
    ids = inv.reshape(-1, 3)
    keep = (ids[:, 0] != ids[:, 1]) & (ids[:, 1] != ids[:, 2]) & (ids[:, 0] != ids[:, 2])
    ids = ids[keep]
    # Drop duplicates (same three corners in any order).
    _, first = np.unique(np.sort(ids, 1), axis=0, return_index=True)
    ids = ids[np.sort(first)]
    return mean[ids], keep


# Our skeleton's bones (skeleton.rs Bone), and which of them each of the model's bones feeds.
B_ROOT, B_SPINE, B_HEAD, B_UARM_L, B_UARM_R, B_FORE_L, B_FORE_R, B_THIGH_L, B_THIGH_R, B_SHIN_L, B_SHIN_R, B_RIFLE = range(12)


def our_bone(name):
    base = name.split('.')[0].split('_')[0]
    left = '.L' in name
    if base in ('hips', 'spine', 'chest', 'neck', 'shoulder'):
        return B_SPINE
    if base == 'head':
        return B_HEAD
    if base == 'arm':
        return B_UARM_L if left else B_UARM_R
    if base in ('forearm', 'hand', 'thumb', 'index', 'middle', 'ring', 'pinky', 'ellbowIK', 'wristIK'):
        return B_FORE_L if left else B_FORE_R
    if base == 'upleg':
        return B_THIGH_L if left else B_THIGH_R
    if base in ('leg', 'foot', 'toe', 'kneeIK', 'ankleIK'):
        return B_SHIN_L if left else B_SHIN_R
    return B_SPINE


def rotate_about(v, pivot, axis, angle):
    """Rodrigues rotation of points `v` about the line through `pivot` along `axis`."""
    a = axis / np.linalg.norm(axis)
    d = v - pivot
    c, s_ = np.cos(angle), np.sin(angle)
    return pivot + d * c + np.cross(a, d) * s_ + np.outer(d @ a, a) * (1 - c)


# The grip the hands are baked into: knuckles, middle joints, thumb tucked in (radians).
CURL_KNUCKLE, CURL_MIDDLE, CURL_THUMB = np.radians(75), np.radians(70), np.radians(35)


def curl(v, we, jo, names, J, side):
    """Closes one hand into a grip at bind (T-pose, palm down): each finger turns about the knuckle line (y) so the
    fingertips go down to the palm, the end joints bend further, and the thumb tucks in."""
    g = -1.0 if side == 'L' else 1.0
    def w_of(prefixes):
        ids = [k for k, n in enumerate(names) if any(n.startswith(p) for p in prefixes)]
        return sum(we[:, i] * np.isin(jo[:, i], ids) for i in range(4))
    out = v.copy()
    for f in ('index', 'middle', 'ring', 'pinky'):
        w1 = w_of([f'{f}.{side}', f'{f}_end.{side}'])
        w2 = w_of([f'{f}_end.{side}'])
        base, mid = J[f'{f}.{side}'], J[f'{f}_end.{side}']
        r1 = rotate_about(out, base, np.array([0.0, 1.0, 0.0]), g * CURL_KNUCKLE)
        mid1 = rotate_about(mid[None], base, np.array([0.0, 1.0, 0.0]), g * CURL_KNUCKLE)[0]
        r2 = rotate_about(r1, mid1, np.array([0.0, 1.0, 0.0]), g * CURL_MIDDLE)
        out = out + (r1 - out) * w1[:, None]
        out = out + (r2 - r1) * w2[:, None] * (w1[:, None] > 0)
    wt = w_of([f'thumb.{side}', f'thumb_end.{side}'])
    rt = rotate_about(out, J[f'thumb.{side}'], np.array([1.0, 0.0, 0.0]), -CURL_THUMB)
    return out + (rt - out) * wt[:, None]


def convert(path):
    j, b = G.load(path)
    sk = j['skins'][0]
    names = [j['nodes'][n]['name'] for n in sk['joints']]
    ibm = G.accessor(j, b, sk['inverseBindMatrices']).reshape(-1, 4, 4).transpose(0, 2, 1).astype(float)
    scale = np.cbrt(np.abs(np.linalg.det(ibm[:, :3, :3])))
    bind = np.array([np.linalg.inv(m) for m in ibm])
    jpos = np.array([bind[k][:3, 3] * scale[k] for k in range(len(names))]) @ G.GL_TO_OURS.T
    J = {n.rsplit('_', 1)[0]: jpos[k] for k, n in enumerate(names)}

    body_top = 0.0
    prims = []
    for m in j['meshes']:
        for p in m['primitives']:
            pos = G.accessor(j, b, p['attributes']['POSITION']).astype(float)
            jo = G.accessor(j, b, p['attributes']['JOINTS_0']).astype(int)
            we = G.accessor(j, b, p['attributes']['WEIGHTS_0']).astype(float)
            idx = G.accessor(j, b, p['indices']).reshape(-1, 3).astype(int)
            is_rifle = m.get('name', '').startswith('M4')
            if is_rifle:
                pos = pos * scale[jo[0, 0]]
            v = pos @ G.GL_TO_OURS.T
            prims.append((is_rifle, p.get('material'), v, idx, jo, we))
            if not is_rifle:
                body_top = max(body_top, v[:, 2].max())
    k = HEIGHT / body_top

    # Swing each arm down around its shoulder joint: (+-1, 0, 0) -> (0, 0, -1), blended by the arm chain's weight.
    def swing(v, w, side):
        piv = J[f'arm.{side}']
        s = -1.0 if side == 'L' else 1.0
        d = v - piv
        # Rotation about y by angle a with x -> -z for the right arm (a = +90 deg), x -> +z mirrored for the left.
        a = np.pi / 2 * s
        c, sn = np.cos(a), np.sin(a)
        rx = d[:, 0] * c + d[:, 2] * sn
        rz = -d[:, 0] * sn + d[:, 2] * c
        r = np.c_[rx, d[:, 1], rz] + piv
        return v + (r - v) * w[:, None]

    arms = {s: chain(names, f'arm.{s}') for s in 'LR'}
    fore = {s: [x for x in arms[s] if not names[x].startswith('arm.')] for s in 'LR'}

    def group_of(nm):
        base = nm.split('.')[0].split('_')[0]
        side = 'L' if '.L' in nm else 'R'
        if base in ('upleg',):
            return L_THIGH if side == 'L' else R_THIGH
        if base in ('leg', 'foot', 'toe', 'ankleIK', 'kneeIK'):
            return L_SHIN if side == 'L' else R_SHIN
        if base == 'arm':
            return L_ARM if side == 'L' else R_ARM
        if base in ('forearm', 'hand', 'thumb', 'index', 'middle', 'ring', 'pinky', 'ellbowIK', 'wristIK'):
            return L_FORE if side == 'L' else R_FORE
        if base in ('head',):
            return HEAD
        return TORSO

    groups = np.array([group_of(n) for n in names])

    # Anchors for the fits (model units, after the swing): body z, and each arm's z after hanging down.
    z_src = [0.0, J['leg.L'][2], J['upleg.L'][2], J['arm.L'][2], J['neck'][2], body_top]
    z_dst = [0.0, KNEE, HIP, SHOULDER_Z, NECK, NECK + (body_top - J['neck'][2]) * k]
    upper = np.linalg.norm(J['forearm.L'] - J['arm.L'])
    lower = np.linalg.norm(J['hand.L'] - J['forearm.L'])
    sh_z = J['arm.L'][2]
    a_src = [sh_z - upper - lower, sh_z - upper, sh_z]
    a_dst = [WRIST, ELBOW, SHOULDER_Z]

    out_tris, out_codes, rifle = [], [], []
    out_skin = []
    for is_rifle, mat, v, idx, jo, we in prims:
        if is_rifle:
            rifle.append((v[idx], mat))
            continue
        vv = v.copy()
        for s in 'LR':
            vv = curl(vv, we, jo, names, J, s)
        # Our bones' weights per vertex (the model's weights, summed per bone of ours).
        mapped = np.array([our_bone(n) for n in names])
        W = np.zeros((len(vv), 12))
        for i in range(4):
            np.add.at(W, (np.arange(len(vv)), mapped[jo[:, i]]), we[:, i])
        for s in 'LR':
            w = sum(we[:, i] * np.isin(jo[:, i], arms[s]) for i in range(4))
            vv = swing(vv, w, s)
        dom = groups[jo[np.arange(len(jo)), we.argmax(1)]]
        # A vertex mostly on the arm chain belongs to the arm even if no single arm bone dominates.
        for s, a_part, f_part in (('L', L_ARM, L_FORE), ('R', R_ARM, R_FORE)):
            wa = sum(we[:, i] * np.isin(jo[:, i], arms[s]) for i in range(4))
            wf = sum(we[:, i] * np.isin(jo[:, i], fore[s]) for i in range(4))
            on = wa > 0.5
            dom[on] = np.where(wf[on] > wa[on] - wf[on], f_part, a_part)
        # Fit: uniform scale, then the piecewise z maps; arms are moved out to the shoulder line.
        out = np.empty_like(vv)
        is_arm = np.isin(dom, [L_ARM, R_ARM, L_FORE, R_FORE])
        out[:, 0] = vv[:, 0] * k
        out[:, 1] = (vv[:, 1] - J['hips'][1]) * k
        out[:, 2] = piecewise(vv[:, 2], z_src, z_dst)
        if is_arm.any():
            out[is_arm, 2] = piecewise(vv[is_arm, 2], a_src, a_dst)
            sx = np.where(vv[is_arm, 0] < 0, -1.0, 1.0)
            ax = np.where(sx < 0, J['arm.L'][0], J['arm.R'][0]) * k
            out[is_arm, 0] = out[is_arm, 0] - ax + sx * SHOULDER_X
            out[is_arm, 1] = out[is_arm, 1] - (J['arm.L'][1] - J['hips'][1]) * k
        # Our rig moves whole parts rigidly, so a triangle across a joint seam must stay with the parent part
        # (body before limb, upper limb before lower): otherwise it flies out with the limb as a shard.
        tp = dom[idx]
        rank = np.vectorize(lambda q: RANK[int(q)])(tp)
        part = tp[np.arange(len(tp)), rank.argmin(1)]
        # Leg seams split at the joint's height instead: below the hip pivot a triangle swings with the thigh,
        # below the knee with the shin.
        cz = np.array([piecewise(v[t, 2], z_src, z_dst).mean() for t in idx])
        for thigh, shin in ((L_THIGH, L_SHIN), (R_THIGH, R_SHIN)):
            has = lambda q: (tp == q).any(1)
            hip_seam = has(TORSO) & has(thigh)
            part[hip_seam] = np.where(cz[hip_seam] < HIP, thigh, TORSO)
            knee_seam = has(thigh) & has(shin)
            part[knee_seam] = np.where(cz[knee_seam] < KNEE, shin, thigh)
        # Gear on the wrong bone moves with what it sits on (as the rigid parts do below): a holster weighted to the
        # shin goes with the thigh, hip cloth weighted to the thighs goes with the body.
        for thigh, shin in ((B_THIGH_L, B_SHIN_L), (B_THIGH_R, B_SHIN_R)):
            high = out[:, 2] > KNEE + 0.08
            W[high, thigh] += W[high, shin]; W[high, shin] = 0
        for thigh in (B_THIGH_L, B_THIGH_R):
            high = out[:, 2] > HIP + 0.05
            W[high, B_SPINE] += W[high, thigh]; W[high, thigh] = 0
        for t, pt in zip(idx, part):
            p = out[t]
            pal = palette(mat, pt, p.mean(0))
            out_tris.append(p); out_codes.append(code(pal, pt)); out_skin.append(W[t])

    # Rifle: decimate, lay the long axis along +y (muzzle forward, magazine down), pistol grip at the right hand.
    rv = np.vstack([t.reshape(-1, 3) for t, _ in rifle])
    centre = rv.mean(0)
    _, _, vt = np.linalg.svd(rv - centre, full_matrices=False)
    ax_long, ax_mid = vt[0], vt[1]
    mag = rifle[0][0].reshape(-1, 3).mean(0) if len(rifle) > 1 else centre  # the 'Amo' (magazine) primitive is first
    # The muzzle is the thin end: compare how far each end's points spread from the long axis.
    proj = (rv - centre) @ ax_long
    radial = np.linalg.norm((rv - centre) - np.outer(proj, ax_long), axis=1)
    lo_end, hi_end = proj < np.percentile(proj, 8), proj > np.percentile(proj, 92)
    if radial[hi_end].mean() > radial[lo_end].mean():
        ax_long = -ax_long
    down = (mag - centre) - ((mag - centre) @ ax_long) * ax_long
    down /= np.linalg.norm(down)
    side = np.cross(ax_long, -down)
    basis = np.array([side, ax_long, -down])  # rows: our x, y, z
    rifle_out = []
    for t, mat in rifle:
        loc = (t.reshape(-1, 3) - centre) @ basis.T
        rifle_out.append((loc.reshape(-1, 3, 3), mat))
    allr = np.vstack([t.reshape(-1, 3) for t, _ in rifle_out])
    length = allr[:, 1].max() - allr[:, 1].min()
    rk = 0.84 / length  # an M4 is about 0.84 m long
    # Pistol grip: the lowest points just behind the magazine.
    lr = allr * rk
    mag_y = ((mag - centre) @ basis.T)[1] * rk
    rear = lr[(lr[:, 1] < mag_y - 0.04) & (lr[:, 1] > mag_y - 0.2)]
    grip = rear[rear[:, 2] < np.percentile(rear[:, 2], 15)].mean(0)
    offset = np.array([SHOULDER_X, 0.05, GRIP - 0.06]) - np.array([0.0, grip[1], grip[2]])
    for t, mat in rifle_out:
        tt = t * rk + offset
        dec, _ = cluster(tt, RIFLE_CELL)
        for p in dec:
            out_tris.append(p); out_codes.append(code(METAL if mat == 0 else UNIFORM_DARK, RIFLE))
            wr = np.zeros((3, 12)); wr[:, B_RIFLE] = 1.0
            out_skin.append(wr)

    tris = np.array(out_tris)
    codes = np.array(out_codes)
    # Team armband round each upper arm.
    zs = tris[:, :, 2]
    parts = (codes // 32).astype(int)
    pals = codes - 32 * parts
    # Gear skinned to the wrong bone (a holster on the shin bone, hip cloth on the thighs) goes with the part it
    # sits on, so it doesn't swing out of place.
    cz = zs.mean(1)
    for shin, thigh in ((L_SHIN, L_THIGH), (R_SHIN, R_THIGH)):
        parts[(parts == shin) & (cz > KNEE + 0.08)] = thigh
    parts[np.isin(parts, [L_THIGH, R_THIGH]) & (cz > HIP + 0.05)] = TORSO
    codes = pals + 32.0 * parts
    band = np.isin(parts, [L_ARM, R_ARM]) & (zs.min(1) > 1.17) & (zs.max(1) < 1.33)
    codes[band] = [code(TEAM, p) for p in parts[band]]
    global LAST_SKIN
    LAST_SKIN = np.array(out_skin)
    return tris, codes


def write_skin(tris, codes, skin, path):
    """The skinned soldier: per vertex x, y, z, colour code (f32 each), then 4 bone ids (u8) and 4 weights (u8,
    summing to 255): 24 bytes, three vertices per triangle."""
    with open(path, 'wb') as f:
        for t, c, w in zip(tris, codes, skin):
            for p, wv in zip(t, w):
                top = np.argsort(-wv)[:4]
                ws = np.clip(wv[top], 0, None)
                ws = ws / ws.sum() if ws.sum() > 0 else np.array([1.0, 0, 0, 0])
                q = np.round(ws * 255).astype(int)
                q[0] += 255 - q.sum()
                f.write(struct.pack('<4f', p[0], p[1], p[2], c))
                f.write(bytes([int(x) for x in top]) + bytes([int(x) for x in q]))


def palette(mat, part, centre):
    """Material -> kit colour. The model's near-black material covers boots, vest, sling, gloves and rifle."""
    if mat == 4:
        return SKIN
    if mat == 1:
        return UNIFORM
    if mat == 2:
        return UNIFORM_DARK
    if mat == 3:
        return HELMET if part == HEAD else PAD
    # mat 0: dark
    if part in (L_SHIN, R_SHIN):
        return BOOT
    if part in (L_FORE, R_FORE):
        return GLOVE
    if part == HEAD:
        return GOGGLE
    return VEST


def write(tris, codes, path):
    with open(path, 'wb') as f:
        for t, c in zip(tris, codes):
            for p in t:
                f.write(struct.pack('<4f', p[0], p[1], p[2], c))


def preview(tris, codes, path):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from matplotlib.collections import PolyCollection
    pal = [[0.10, 0.12, 0.15]] * 16 + [[0.62, 0.60, 0.44], [0.49, 0.47, 0.34], [0.17, 0.18, 0.19], [0.58, 0.60, 0.62],
           [0.40, 0.38, 0.28], [0.25, 0.26, 0.27], [0.12, 0.12, 0.13], [0.13, 0.14, 0.15], [0.20, 0.21, 0.22],
           [0.10, 0.11, 0.12], [0.63, 0.61, 0.45], [0.86, 0.70, 0.55], [0.08, 0.08, 0.09], [0.17, 0.18, 0.19],
           [0.72, 0.62, 0.42], [0.8, 0.1, 0.1]]
    light = np.array([0.4, -0.5, 0.75]); light /= np.linalg.norm(light)
    fig, axs = plt.subplots(1, 3, figsize=(15, 7))
    for ax, (name, yaw) in zip(axs, [('front', 0.0), ('side', np.pi / 2), ('3/4 back', 2.4)]):
        c, s = np.cos(yaw), np.sin(yaw)
        R = np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])
        t = tris @ R.T
        n = np.cross(t[:, 1] - t[:, 0], t[:, 2] - t[:, 0]); n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-9
        depth = t[:, :, 1].mean(1)
        order = np.argsort(-depth)
        cols = [np.array(pal[int(cc) % 32]) * (0.45 + 0.55 * abs(nn @ (R @ light))) for cc, nn in zip(codes[order], n[order])]
        ax.add_collection(PolyCollection(t[order][:, :, [0, 2]], facecolors=np.clip(cols, 0, 1), edgecolors='none'))
        ax.set_xlim(-0.6, 0.6); ax.set_ylim(-0.05, 1.85); ax.set_aspect('equal'); ax.set_title(name)
        for z in (KNEE, HIP, ELBOW, SHOULDER_Z, NECK, GRIP):
            ax.axhline(z, color='c', lw=0.3)
    fig.savefig(path, dpi=80)


if __name__ == '__main__':
    tris, codes = convert(sys.argv[1])
    write(tris, codes, sys.argv[2])
    if sys.argv[2].endswith('.fig'):
        write_skin(tris, codes, LAST_SKIN, sys.argv[2][:-4] + '.skin')
    parts = (codes // 32).astype(int)
    print(len(tris), 'triangles;', {int(p): int((parts == p).sum()) for p in np.unique(parts)})
    print('bounds', tris.reshape(-1, 3).min(0).round(3), tris.reshape(-1, 3).max(0).round(3))
    if len(sys.argv) > 3:
        preview(tris, codes, sys.argv[3])
