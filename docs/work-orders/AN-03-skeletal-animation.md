# AN-03: soldiers play the reference clips directly (skeletal animation)

- **ID:** AN-03
- **Revision:** v1.0
- **Status:** in progress (M1 to M4 in the first PR)
- **Date:** 2026-11-21
- **Owner:** animation lane
- **Base:** main 660cf30

## Goal

Soldiers play the Mixamo clips exactly as authored, on the rigged soldier, instead of approximations fitted to the
procedural rig (AN-02-S12/S13). Built for scale: one compute pass poses every soldier, the vertex shader only skins.

## How it works

- `scripts/skin_bake.py` (with `scripts/skin_clips.txt`): rigged FBX and clips to `assets/rigged.skin` (triangle list
  with 4 bone weights) and `assets/anims.bin` (40 joints, local rotations as snorm16 quaternions at 30 fps, root
  position; horizontal travel taken out and kept per clip).
- `src/skin.rs`: loading, sampling, forward kinematics, skinning; the pose-to-clip map (`pose_clips`); the rifle
  fixed to the right hand (`rifle`); the CPU mirror of the compute pass (`pose_instance`, `joint_at`, `muzzle`).
- `src/wgsl/skin_cs.wgsl` + `src/skin_gpu.rs`: per instance buffer, one thread per soldier reads the same instance
  record the procedural shader reads (pose, phase, cadence, previous pose and blend time, upper layer, direction)
  and writes joint transforms; `vs_skin` in scene.wgsl and shadow.wgsl skins. Gaits blend the two nearest of 8
  direction clips; pose changes cross-fade over 0.35 s; the upper layer replaces the spine and above; the drop to
  prone plays `crouch_to_prone` through the drop keys; deaths hold their last frame.
- The game (`viewer.rs`) draws `skin` / `skin_enemy` up to 250 m; the 24-triangle procedural figure beyond.
  Muzzle flashes and tracers come from the skinned pose (`anim.rs` `barrel`).

## Milestones

- [x] M1 pipeline: bake, CPU reference, preview (`DUMP=skin_dump scripts/stance_sheet.py out.png walk_f@0.2`).
- [x] M2 GPU: compute pass, skinned scene and shadow pipelines; `tests/skin_gpu.rs` checks GPU against CPU and that
  the real pipelines build and draw (lavapipe; skipped without a native adapter).
- [x] M3 all states mapped (`pose_clips`); poses without a clip borrow the nearest.
- [x] M4 rifle on the hand bone; muzzle from the skinned pose.
- [ ] M5 performance at scale: measure 1k, 5k, 10k soldiers at 60 fps on the RTX 3050 Ti; LOD mesh and slower
  pose updates far away; muzzle lookup only for shooters (1.4 ms native for 1,000 every frame today).
- [ ] M6 clips we do not have: crawl left and right, dive, casualty drag and carry, standing to prone, radio,
  tablet, signal, drone, first aid; aim pitch, recoil and flinch as additive layers.
- [ ] Remove the fitted gait and crawl tables (AN-02-S12/S13) once nothing draws the procedural figure close up.

## Acceptance checks

- [x] `scripts/check.sh` passes.
- [x] `hash_battle` unchanged (594771974983721861): render only.
- [ ] In the browser: soldiers walk, run, sprint, crouch, crawl and go prone with the clips; rifle in hand;
  muzzle flashes on the muzzle.

## Next action

Look at it in the browser; then M5 measurements.
