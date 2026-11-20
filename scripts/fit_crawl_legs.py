"""Solves our prone leg params so knee and ankle match a reference crawl by position (AN-02-S11):
scripts/fit_crawl_legs.py <ref_fk.json> [samples=24] [out.json]. Rows: hip L, hip R, knee L, knee R, side L, side R.
The leg chain is a Python port of pose.rs (leg, leg_roll, sideways, prone rotation); keep it in step."""
import json, sys, numpy as np
from scipy.optimize import minimize
HIP_Z, KNEE_Z, ANKLE_Z, HIP_X = 0.88, 0.46, 0.085, 0.1
def rot(q, py, pz, a):
    s, c = np.sin(a), np.cos(a); dy, dz = q[1]-py, q[2]-pz
    return np.array([q[0], py+dy*c-dz*s, pz+dy*s+dz*c])
def sideways(q, a, px):
    s, c = np.sin(a), np.cos(a); dx, dz = q[0]-px, q[2]-HIP_Z
    return np.array([px+dx*c-dz*s, q[1], HIP_Z+dx*s+dz*c])
def roll_angle(knee, i):
    t = np.clip((knee-0.1)/0.3, 0, 1); r = np.pi/2*t*t*(3-2*t)
    return r if i == 0 else -r   # inward, as pose.rs leg_roll_angle
def roll(q, a, px):
    s, c = np.sin(a), np.cos(a); dx = q[0]-px
    return np.array([px+dx*c-q[1]*s, dx*s+q[1]*c, q[2]])
def leg_point(z0, hip, knee, side, i):
    px = -HIP_X if i == 0 else HIP_X
    q = np.array([px, 0.0, z0])
    q = rot(rot(q, 0, KNEE_Z, -knee), 0, HIP_Z, hip)
    q = sideways(roll(q, roll_angle(knee, i), px), side, px)
    q = np.array([q[0], q[2], -q[1]])          # prone: forward = old up, up = -old forward
    return q - np.array([px, HIP_Z, 0.0])      # relative to the hip joint
ref = json.load(open(sys.argv[1]))['frames']; N = int(sys.argv[2]) if len(sys.argv) > 2 else 24
nf = len(ref) - 1                               # last frame repeats the first
def body(f):
    P = lambda k: np.array(ref[f][k], float)/100
    fw = P('Spine2')-P('Hips'); fw[1] = 0; fw /= np.linalg.norm(fw); up = np.array([0, 1.0, 0]); rt = np.cross(fw, up)
    out = {}
    for i, side in enumerate(['Left', 'Right']):
        h = P(side+'UpLeg')
        rel = lambda k: np.array([(P(k)-h)@rt, (P(k)-h)@fw, (P(k)-h)@up])
        out[i] = (rel(side+'Leg'), rel(side+'Foot'))
    return out
ref_len = np.mean([np.linalg.norm(np.array(ref[0][s+'Leg'])-np.array(ref[0][s+'UpLeg']))+np.linalg.norm(np.array(ref[0][s+'Foot'])-np.array(ref[0][s+'Leg'])) for s in ['Left','Right']])/100
scale = ((HIP_Z-KNEE_Z)+(KNEE_Z-ANKLE_Z))/ref_len
rows, errs, x0 = [], [], {0: [0.2, 0.3, -0.3], 1: [0.2, 0.3, 0.3]}
for k in range(N):
    t = k*nf/N; f0 = int(t); w = t-f0; b0, b1 = body(f0), body(min(f0+1, nf))
    row = {}
    for i in (0, 1):
        Kt = ((1-w)*b0[i][0]+w*b1[i][0])*scale; At = ((1-w)*b0[i][1]+w*b1[i][1])*scale
        def cost(x):
            hip, knee, side = x
            ek = leg_point(KNEE_Z, hip, knee, side, i)-Kt; ea = leg_point(ANKLE_Z, hip, knee, side, i)-At
            ea[2] *= 0.3                          # ankle height matters less (reference stands on its toes)
            return 4*ek@ek + ea@ea + 0.002*((x-np.array(x0[i]))@(x-np.array(x0[i])))
        r = minimize(cost, x0[i], method='L-BFGS-B', bounds=[(-0.6, 1.6), (0.0, 2.4), (-1.7, 1.7)])
        x0[i] = list(r.x); row[i] = r.x
        errs.append((np.linalg.norm(leg_point(KNEE_Z, *r.x, i)-Kt), np.linalg.norm(leg_point(ANKLE_Z, *r.x, i)-At)))
    rows.append([row[0][0], row[1][0], row[0][1], row[1][1], row[0][2], row[1][2]])
e = np.array(errs)
print(f"scale {scale:.3f}; knee error mean {e[:,0].mean()*100:.1f} cm max {e[:,0].max()*100:.1f}; ankle mean {e[:,1].mean()*100:.1f} max {e[:,1].max()*100:.1f}")
print("hipL hipR kneeL kneeR sideL sideR")
for r in rows[::3]: print(" ".join(f"{v:6.3f}" for v in r))
json.dump(rows, open(sys.argv[3] if len(sys.argv) > 3 else 'legs.json', 'w'))
