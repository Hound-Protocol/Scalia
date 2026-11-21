"""Reference targets for scalia-render's examples/fit_clip, from a Mixamo FBX (skinned or not):
scripts/clip_ref.py <clip.fbx> <out.txt> [samples=24] [--once] [--start s] [--end s]

The FBX goes through libassimp's glTF exporter (pyassimp crashes on skinned clips), then forward kinematics.
Each sample is a line of 13 unit segment directions in the figure frame (x right, y forward, z up), so the
fit does not depend on the reference skeleton's proportions: thigh L, shin L, thigh R, shin R, spine, shoulder
line (left to right), upper arm L, forearm L, upper arm R, forearm R, head, foot L, foot R; then the hip
height over the leg length. A loop is sampled over [start, end) and closes on itself; --once samples
[start, end] inclusive. Raw clips stay out of the repo (CONTRIBUTING: third-party assets)."""
import base64, ctypes, ctypes.util, json, os, struct, sys, tempfile
import numpy as np

def export_gltf(fbx, out):
    lib = ctypes.CDLL(ctypes.util.find_library('assimp'))
    lib.aiImportFile.restype = ctypes.c_void_p; lib.aiImportFile.argtypes = [ctypes.c_char_p, ctypes.c_uint]
    lib.aiExportScene.restype = ctypes.c_int
    lib.aiExportScene.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint]
    lib.aiReleaseImport.argtypes = [ctypes.c_void_p]; lib.aiGetErrorString.restype = ctypes.c_char_p
    s = lib.aiImportFile(fbx.encode(), 0)
    if not s: sys.exit(f"import failed: {lib.aiGetErrorString().decode()}")
    rc = lib.aiExportScene(s, b'gltf2', out.encode(), 0); lib.aiReleaseImport(s)
    if rc: sys.exit(f"export failed: {lib.aiGetErrorString().decode()}")

def quat(q):
    x, y, z, w = q
    return np.array([[1-2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w)], [2*(x*y+z*w), 1-2*(x*x+z*z), 2*(y*z-x*w)],
                     [2*(x*z-y*w), 2*(y*z+x*w), 1-2*(x*x+y*y)]])

class Clip:
    def __init__(self, path):
        g = json.load(open(path)); base = os.path.dirname(path); self.g = g
        self.bufs = [base64.b64decode(b['uri'].split(',')[1]) if b['uri'].startswith('data:')
                     else open(os.path.join(base, b['uri']), 'rb').read() for b in g['buffers']]
        self.nodes = g['nodes']; self.parent = {}
        for i, n in enumerate(self.nodes):
            for c in n.get('children', []): self.parent[c] = i
        an = g['animations'][0]; self.ch = {}
        for c in an['channels']:
            s = an['samplers'][c['sampler']]
            self.ch.setdefault(c['target']['node'], {})[c['target']['path']] = (self.acc(s['input'])[:, 0], self.acc(s['output']))
        bare = lambda i: self.nodes[i].get('name', '').replace('mixamorig:', '')
        self.animated = {bare(i) for i in self.ch}
        self.idx = {bare(i): i for i in range(len(self.nodes)) if '$' not in self.nodes[i].get('name', '')}
        self.duration = max(v[0][-1] for d in self.ch.values() for v in d.values())
    def acc(self, i):
        a = self.g['accessors'][i]; v = self.g['bufferViews'][a['bufferView']]
        n = {'SCALAR': 1, 'VEC3': 3, 'VEC4': 4}[a['type']]
        off = v.get('byteOffset', 0) + a.get('byteOffset', 0); st = v.get('byteStride', 4*n)
        return np.array([struct.unpack_from('<%df' % n, self.bufs[v['buffer']], off+k*st) for k in range(a['count'])])
    def local(self, i, t):
        n = self.nodes[i]; name = n.get('name', '')
        # assimp's FBX helper nodes: animated bones carry their translation and pre-rotation in the keys.
        if '_$AssimpFbx$_' in name and name.split('_$')[0].replace('mixamorig:', '') in self.animated: return np.eye(4)
        if 'matrix' in n and i not in self.ch: return np.array(n['matrix']).reshape(4, 4).T
        d = self.ch.get(i, {})
        def key(path, default):
            if path not in d: return np.array(n.get(path, default), float)
            ts, vs = d[path]
            if t <= ts[0]: return vs[0]
            if t >= ts[-1]: return vs[-1]
            k = np.searchsorted(ts, t) - 1; f = (t-ts[k])/(ts[k+1]-ts[k]); a, b = vs[k], vs[k+1].copy()
            if path == 'rotation':
                if a @ b < 0: b = -b
                q = a*(1-f)+b*f; return q/np.linalg.norm(q)
            return a*(1-f)+b*f
        m = np.eye(4); m[:3, :3] = quat(key('rotation', [0, 0, 0, 1])) * key('scale', [1, 1, 1]); m[:3, 3] = key('translation', [0, 0, 0])
        return m
    def joints(self, t):
        cache = {}
        def world(i):
            if i not in cache: cache[i] = (world(self.parent[i]) @ self.local(i, t)) if i in self.parent else self.local(i, t)
            return cache[i]
        # Mixamo: y up, the character faces +z with its left at +x. Figure: x right, y forward, z up.
        return {k: np.array([-world(i)[0, 3], world(i)[2, 3], world(i)[1, 3]]) for k, i in self.idx.items()}

SEGMENTS = [('LeftUpLeg', 'LeftLeg'), ('LeftLeg', 'LeftFoot'), ('RightUpLeg', 'RightLeg'), ('RightLeg', 'RightFoot'),
            ('HIPS', 'Neck'), ('LeftArm', 'RightArm'), ('LeftArm', 'LeftForeArm'), ('LeftForeArm', 'LeftHand'),
            ('RightArm', 'RightForeArm'), ('RightForeArm', 'RightHand'), ('Neck', 'HeadTop_End'),
            ('LeftFoot', 'LeftToeBase'), ('RightFoot', 'RightToeBase')]

def main():
    a = sys.argv[1:]; once = '--once' in a
    opt = lambda k, d: float(a[a.index(k)+1]) if k in a else d
    pos = [x for i, x in enumerate(a) if not x.startswith('--') and (i == 0 or a[i-1] not in ('--start', '--end'))]
    fbx, out = pos[0], pos[1]; n = int(pos[2]) if len(pos) > 2 else 24
    tmp = tempfile.mkdtemp(); gl = os.path.join(tmp, 'clip.gltf'); export_gltf(fbx, gl); c = Clip(gl)
    t0, t1 = opt('--start', 0.0), opt('--end', c.duration)
    times = [t0 + (t1-t0)*k/(n-1 if once else n) for k in range(n)]
    j0 = c.joints(t0)
    leg = np.mean([np.linalg.norm(j0[s+'Leg']-j0[s+'UpLeg'])+np.linalg.norm(j0[s+'Foot']-j0[s+'Leg']) for s in ('Left', 'Right')])
    rows = []
    for t in times:
        j = c.joints(t); j['HIPS'] = (j['LeftUpLeg']+j['RightUpLeg'])/2
        dirs = [(j[b]-j[a])/np.linalg.norm(j[b]-j[a]) for a, b in SEGMENTS]
        rows.append(' '.join(f'{v:.5f}' for d in dirs for v in d) + f' {j["HIPS"][2]/leg:.5f}')
    open(out, 'w').write(f'{n} {"once" if once else "loop"} {t1-t0:.4f}\n' + '\n'.join(rows) + '\n')
    print(f'{os.path.basename(fbx)}: {c.duration:.2f} s, sampled {t0:.2f}-{t1:.2f} s at {n}, leg {leg:.1f} units')

main()
