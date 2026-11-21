#!/bin/bash
# Re-bakes the rifle gaits (AN-02-S13) from the Mixamo Pro Rifle Pack: scripts/bake_gaits.sh <pack dir> [walk run sprint]
# For each gait and each eighth of a turn: scripts/clip_ref.py (48 samples from the left heel strike), then
# examples/fit_clip (torso, pelvis turn, legs, body lift, planted boots), then scripts/clip_table.py. Then the
# registry and GAIT_STRIDE (the sweep each fit pins the planted boot to). Raw clips stay out of the repo.
# Run examples/anim_audit afterwards. Needs libassimp and numpy.
set -e
cd "$(dirname "$0")/.."
P=${1:?pack directory with "walk forward.fbx" etc.}; shift
W=$(mktemp -d)
GAITS=${@:-walk run sprint}
# The table lookup is stubbed while fitting, so a change in table shape still builds.
python3 - <<'PY'
p='crates/scalia-render/src/pose.rs'; s=open(p).read()
a=s.index("fn gait_loops(pose: Pose) -> Option<[&'static GaitLoop; 8]> {"); b=s.index("\n}\n",a)+3
open('/tmp/gait_loops_body.txt','w').write(s[a:b])
open(p,'w').write(s[:a]+"fn gait_loops(_pose: Pose) -> Option<[&'static GaitLoop; 8]> {\n    None\n}\n"+s[b:])
PY
restore() { python3 - <<'PY'
p='crates/scalia-render/src/pose.rs'; s=open(p).read()
s=s.replace("fn gait_loops(_pose: Pose) -> Option<[&'static GaitLoop; 8]> {\n    None\n}\n", open('/tmp/gait_loops_body.txt').read())
open(p,'w').write(s)
PY
}
trap restore EXIT
python3 scripts/clip_table.py --registry > /dev/null
for key in $GAITS; do
  case $key in
    walk) NAME=WALK; id=1; label=Walk; TOL=0.05;;
    run) NAME=RUN; id=2; label=Run; TOL=0.07;;
    sprint) NAME=SPRINT; id=7; label=Sprint; TOL=0.07;;
    *) echo "unknown gait $key"; exit 1;;
  esac
  for d in forward:F forward_left:FL left:L backward_left:BL backward:B backward_right:BR right:R forward_right:FR; do
    dk=${d%%:*}; ab=${d##*:}; clip="$key ${dk//_/ }"
    python3 scripts/clip_ref.py "$P/$clip.fbx" "$W/$key-$dk.txt" 48 --strike > /dev/null
    PLANT_TOL=$TOL cargo run -q --release -p scalia-render --example fit_clip -- "$W/$key-$dk.txt" $id torso,yaw,legs,height,plant > "$W/$key-$dk.rows" 2> "$W/$key-$dk.err"
    python3 scripts/clip_table.py "$W/$key-$dk.rows" ${NAME}_$ab "AN-02-S13: $label, ${dk//_/ }, from the Pro Rifle Pack clip \"$clip\" (examples/fit_clip, torso,yaw,legs,height,plant)." hipL,hipR,kneeL,kneeR,sideL,sideR,yaw,lean,twist,tilt,bob --center lean,twist,tilt > /dev/null
    echo "$clip: $(grep -o 'sweep [0-9.]*' "$W/$key-$dk.err") m per cycle"
  done
done
restore; trap - EXIT
python3 scripts/clip_table.py --registry
python3 - "$W" <<'PY'
import re, sys
W = sys.argv[1]
rows = []
for key, name in [('walk', 'Walk'), ('run', 'Run'), ('sprint', 'Sprint')]:
    vals = [float(re.search(r'plant: boots sweep ([0-9.]+)', open(f'{W}/{key}-{d}.err').read()).group(1))
            for d in ['forward', 'forward_left', 'left', 'backward_left', 'backward', 'backward_right', 'right', 'forward_right']]
    rows.append(f"    [{', '.join(f'{v:.2f}' for v in vals)}], // {name}")
p = 'crates/scalia-render/src/pose.rs'; s = open(p).read()
s = re.sub(r"pub const GAIT_STRIDE: \[\[f32; 8\]; 3\] = \[.*?\];", "pub const GAIT_STRIDE: [[f32; 8]; 3] = [\n" + "\n".join(rows) + "\n];", s, count=1, flags=re.S)
open(p, 'w').write(s)
print("GAIT_STRIDE updated")
PY
