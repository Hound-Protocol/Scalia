"""Joint positions of a reference clip (FBX) by forward kinematics: scripts/ref_fk.py <clip.fbx> <out.json>.
For animated bones, assimp's $AssimpFbx$ translation and pre-rotation helper nodes are skipped (the animation
keys already contain them). Raw clips are not kept in the repo; bring your own. Needs pyassimp and libassimp."""
import json, sys, numpy as np, pyassimp
fbx = sys.argv[1]
import os
ORDER = os.environ.get('QORDER', 'wxyz')
def qmat(q):
    w, x, y, z = q if ORDER == 'wxyz' else (q[3], q[0], q[1], q[2])
    return np.array([[1-2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w)], [2*(x*y+z*w), 1-2*(x*x+z*z), 2*(y*z-x*w)], [2*(x*z-y*w), 2*(y*z+x*w), 1-2*(x*x+y*y)]])
with pyassimp.load(fbx) as s:
    nodes = {}
    def walk(n, parent):
        name = n.name.replace("mixamorig:", "")
        nodes[name] = (parent, np.array(n.transformation, dtype=float).reshape(4, 4))
        for c in n.children: walk(c, name)
    walk(s.rootnode, None)
    a = s.animations[0]; tps = a.tickspersecond or 30.0
    ch = {c.nodename.data.decode().replace("mixamorig:", "") if isinstance(c.nodename.data, bytes) else str(c.nodename.data).replace("mixamorig:", ""): c for c in a.channels}
    names = list(nodes)
    print("nodes:", [n for n in names if "Assimp" in n][:6], "count", len(names))
    nf = len(ch["Hips"].rotationkeys)
    want = ["Hips", "Spine2", "LeftUpLeg", "LeftLeg", "LeftFoot", "LeftToeBase", "RightUpLeg", "RightLeg", "RightFoot", "RightToeBase", "LeftArm", "LeftForeArm", "LeftHand", "RightArm", "RightForeArm", "RightHand", "Head"]
    out = []
    for f in range(nf):
        glob = {}
        def g(name):
            if name in glob: return glob[name]
            parent, rest = nodes[name]
            m = rest.copy()
            MODE = os.environ.get('MODE', 'full')
            base = name.split('_$')[0]
            if os.environ.get('NOPRE', '1') and '_$AssimpFbx$_PreRotation' in name and base in ch:
                m = np.eye(4)
            if os.environ.get('NOTR', '1') and '_$AssimpFbx$_Translation' in name and base in ch:
                m = np.eye(4)
            if MODE == 'rot' and name in ch and name != 'Hips':
                c = ch[name]
                m[:3, :3] = qmat(c.rotationkeys[min(f, len(c.rotationkeys)-1)].value)
            elif MODE == 'rest':
                pass
            elif name in ch:
                c = ch[name]
                if len(c.rotationkeys): m[:3, :3] = qmat(c.rotationkeys[min(f, len(c.rotationkeys)-1)].value)
                if len(c.positionkeys): m[:3, 3] = c.positionkeys[min(f, len(c.positionkeys)-1)].value
            glob[name] = (g(parent) @ m) if parent else m
            return glob[name]
        out.append({k: [round(float(v), 2) for v in g(k)[:3, 3]] for k in want if k in nodes})
    json.dump({"fps": tps, "frames": out}, open(sys.argv[2], "w"))
    f0 = out[0]; L = lambda a, b: np.linalg.norm(np.array(f0[a]) - np.array(f0[b]))
    import numpy as _n
    def score(fr):
        P=lambda k:_n.array(fr[k]); length=_n.linalg.norm(P("Head")-(P("LeftFoot")+P("RightFoot"))/2)
        ys=[fr[k][1] for k in fr]; return round(length,1), round(min(ys),1), round(max(ys),1)
    print("ORDER", ORDER, "head-to-feet, min y, max y:", score(out[0]), score(out[35]))
    print("frames", nf, "thigh", round(L("LeftUpLeg", "LeftLeg"), 1), "shin", round(L("LeftLeg", "LeftFoot"), 1), "hips", f0["Hips"], "Lfoot", f0["LeftFoot"], "head", f0["Head"])
