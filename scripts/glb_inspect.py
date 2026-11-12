"""Prints a skinned GLB's joints (world bind pose, our axes: x right, y forward, z up) and its meshes' bone use."""
import json, struct, sys
import numpy as np

def load(path):
    b = open(path, 'rb').read()
    jl = struct.unpack('<I', b[12:16])[0]
    j = json.loads(b[20:20 + jl])
    off = 20 + jl
    bl = struct.unpack('<I', b[off:off + 4])[0]
    return j, b[off + 8: off + 8 + bl]

def accessor(j, bin_, i):
    a = j['accessors'][i]; v = j['bufferViews'][a['bufferView']]
    n = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[a['type']]
    dt = {5126: np.float32, 5125: np.uint32, 5123: np.uint16, 5121: np.uint8}[a['componentType']]
    stride = v.get('byteStride', 0); start = v.get('byteOffset', 0) + a.get('byteOffset', 0)
    item = n * np.dtype(dt).itemsize
    if stride and stride != item:
        raw = np.frombuffer(bin_, np.uint8, a['count'] * stride, start).reshape(a['count'], stride)[:, :item]
        return np.frombuffer(raw.tobytes(), dt).reshape(a['count'], n)
    return np.frombuffer(bin_, dt, a['count'] * n, start).reshape(a['count'], n)

def trs(n):
    if 'matrix' in n: return np.array(n['matrix'], float).reshape(4, 4).T
    t = np.array(n.get('translation', [0, 0, 0]), float); x, y, z, w = n.get('rotation', [0, 0, 0, 1]); s = np.array(n.get('scale', [1, 1, 1]), float)
    r = np.array([[1-2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w)], [2*(x*y+z*w), 1-2*(x*x+z*z), 2*(y*z-x*w)], [2*(x*z-y*w), 2*(y*z+x*w), 1-2*(x*x+y*y)]])
    m = np.eye(4); m[:3, :3] = r * s; m[:3, 3] = t; return m

def globals_(j):
    g = [None] * len(j['nodes'])
    def walk(i, parent):
        g[i] = parent @ trs(j['nodes'][i])
        for c in j['nodes'][i].get('children', []): walk(c, g[i])
    for r in j['scenes'][j.get('scene', 0)]['nodes']: walk(r, np.eye(4))
    return g

GL_TO_OURS = np.array([[-1, 0, 0], [0, 0, 1], [0, 1, 0]], float)  # glTF faces +z with y up; we face +y with z up

def skinned(j, bin_, g):
    """World positions of every skinned primitive in the bind (node) pose: list of (mesh name, material, verts, tris, joints, weights)."""
    skin = j['skins'][0]; joints = skin['joints']
    ibm = accessor(j, bin_, skin['inverseBindMatrices']).reshape(-1, 4, 4).transpose(0, 2, 1)
    jm = np.array([g[n] @ ibm[k] for k, n in enumerate(joints)])
    out = []
    for m in j['meshes']:
        for p in m['primitives']:
            pos = accessor(j, bin_, p['attributes']['POSITION']).astype(float)
            jo = accessor(j, bin_, p['attributes']['JOINTS_0']).astype(int)
            we = accessor(j, bin_, p['attributes']['WEIGHTS_0']).astype(float)
            idx = accessor(j, bin_, p['indices']).reshape(-1, 3).astype(int)
            h = np.c_[pos, np.ones(len(pos))]
            w = np.zeros((len(pos), 3))
            for k in range(4):
                w += we[:, k:k+1] * np.einsum('nij,nj->ni', jm[jo[:, k]], h)[:, :3]
            out.append((m.get('name'), p.get('material'), w @ GL_TO_OURS.T, idx, jo, we))
    return out

if __name__ == '__main__':
    j, bin_ = load(sys.argv[1]); g = globals_(j)
    names = [j['nodes'][n]['name'] for n in j['skins'][0]['joints']]
    meshes = skinned(j, bin_, g)
    allv = np.vstack([m[2] for m in meshes]); lo, hi = allv.min(0), allv.max(0)
    print('bounds', lo.round(3), hi.round(3))
    for k, n in enumerate(j['skins'][0]['joints']):
        p = (g[n][:3, 3] @ GL_TO_OURS.T)
        print(k, names[k], p.round(3))
    for name, mat, v, idx, jo, we in meshes:
        dom = jo[np.arange(len(jo)), we.argmax(1)]
        u, c = np.unique(dom, return_counts=True)
        print(name, mat, len(idx), 'tris; bones', {names[a]: int(b) for a, b in zip(u, c)})
