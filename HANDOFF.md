# Scalia Agent Handoff

Start at the [README](README.md) for all workstreams and [CONTRIBUTING.md](CONTRIBUTING.md) for how to deliver changes. Docs index: [docs/README.md](docs/README.md). Verify this page against Git and CI before relying on it.

Last updated: 2026-11-22 (Patch: Animations 5).

## Patches: Animations 3 to 5 (2026-11-22): the low crawl

The crawl is now the army low crawl, solved in `crawl.rs` and baked to `pose::HIGH_CRAWL` (name kept; 23 fields,
`crawl::FIELDS`; regenerate with `cargo run -q --release -p scalia-render --example crawl_solve rows`). Patch 5: the
body rolls away from the drawn knee and the spine bends toward it (`crawl::ROLL`, `BEND`, fields `twist`, `hipRoll`,
`bend`, `look`; the legs' roll fades out crawling sideways), `PUSH` 0.26 over `DRAW` 0.18 (1.35 strokes/s at
`CMD_CRAWL`), the rifle rolled as the prone carry (`RIFLE_ROLL` +pi/2) so crawl and prone blend without it swinging
through the ground, and the shoulder cut follows the arm whole (`pose::SHOULDER_SPLIT`; upper arms no longer stretch
into wedges, `examples/stretch_report`). Open: crawl and prone blend joint angles, so a knee can press about 5 cm into the
ground for a moment (fix: start and stop matched to the stroke); the rifle-arm forearm rests 3 to 4 cm into the vest
in the standing carries.

## Patch: Animations 2 (2026-11-22)

High crawl (`crates/scalia-render/src/crawl.rs`, baked to `pose::HIGH_CRAWL`), foot roll and planted boot heading in
the baked gaits (`pose::foot_roll`, clip buffer `GAIT_TOE_AT`), footsteps on contact (`pose::touchdowns`), arms clear of
the torso (`pose::clear_arms`), rifle-first turning (`anim.rs` `lag`, packed by `pose::pack_layer_lag`), prone turns
about the hips (`Frame::shift`), springs for the body's settle (`prims.rs`). Measuring tools: `contact_report`,
`foot_report`, `crawl_solve`; `anim_audit` and `gait_strides` now measure the point a boot stands on. Math map:
the SCALIA project doc `claude/animation-math-reference.md`.

## Earlier animation lane (AN-02, prone and crawl)

Merged to `main`:
- AN-02-S03 ([#67](https://github.com/Hound-Protocol/Scalia/pull/67)): prone aim independent of crawl gait.
- AN-02-S04 ([#69](https://github.com/Hound-Protocol/Scalia/pull/69)): per-leg knee draw from the reference clips.
- AN-02-S05 ([#70](https://github.com/Hound-Protocol/Scalia/pull/70)): arms follow the knee draw.
- AN-02-S06 ([#71](https://github.com/Hound-Protocol/Scalia/pull/71), `ffcd47e`): forward crawl baked from the Mixamo reference (24-sample table in `pose.rs`, mirrored in `common.wgsl`).

- AN-02-S07 (this PR): smaller body twist and the elbow-on-ground test. This commit was pushed to the S05 branch after #70 merged, so it was missing from `main` until now.

- AN-02-S08 (this PR): knee draw capped (left 0.40, right 0.60 of the old range) because the right shin rose to 0.43 m, as high as the torso; now 0.33 m. Measured by `crawl_keeps_both_shins_near_the_ground`.

- AN-02-S09 (this PR): prone no longer a plank on the toes. `PRONE_LIFT` 0.23 to 0.14, feet point back when settled (`PRONE_POINT`), chest arched up on the elbows (`PRONE_LEAN` -0.28, `PRONE_AIM_LEAN` -0.30) with rifle and head compensated, crawl hips held at the prone value. Transitions keep the old 0.23 lift until fully down (`prone_settle`). Mirrored in `common.wgsl` and `skeleton.rs`. Checked with a rendered side view (scripts/stance_sheet.py) and `prone_hips_and_legs_rest_on_the_ground`.

- AN-02-S10 (this PR): leg roll lying prone (`leg_roll_angle`): each leg turns up to a quarter turn about its own axis as the knee draws (0.1 to 0.4 rad), only when settled prone, so the bent knee folds sideways along the ground. Full reference knee draw restored (left 0.60, right 0.95). Highest shin in the crawl 0.32 m to 0.19 m. Test `crawl_draws_the_knee_up_sideways_without_lifting_the_shin`. Not in `skeleton.rs` (it also omits `side`; its exactness test covers poses 0 to 10 only).

- AN-02-S11 (this PR): forward crawl legs solved by position from the reference (`CRAWL_FWD_LEGS`: hip, knee, side per leg, 24 samples). Reference knees and ankles found by forward kinematics on the FBX (assimp: skip the `$AssimpFbx$` translation and pre-rotation helpers of animated bones), scaled to our leg length, then least-squares fitted with a Python port of the leg chain (port matches Rust exactly). Leg roll turned inward (fit error 7 cm against 30 cm outward). One knee draws out and tucks; the other trails. Tests: `forward_crawl_legs_follow_the_reference_not_a_splay`.

- AN-02-S12 (this PR): general clip baking. `scripts/clip_ref.py` turns any Mixamo FBX (skinned packs too, via assimp's glTF export) into per-sample segment directions in the figure frame; `examples/fit_clip` solves our pose params against them with the real `pose_vertex` (legs, torso, arms, head, bob; ground and low-knee constraints when prone; `grip` keeps the rifle within the left hand's reach); `scripts/clip_table.py` writes the table into pose.rs and its WGSL mirror between `// CLIP <NAME>` markers, sampled with `clip_at`. Test `baked_clip_tables_match` checks every mirror. First use: crawling backwards (`CRAWL_BACK`, from Moving Backward In Prone Position, 0.52 m per cycle, about the forward clip's), weighted by the backward component. Test `baked_backward_crawl_holds_the_rifle_and_stays_on_the_ground`. `stance_dump` takes `@phase~dir`.

- AN-02-S13 (this PR): rifle walk, run and sprint baked from the Pro Rifle Pack, 8 directions each, blended by travel direction (`gait_bake`). Baked tables now live only in pose.rs and reach the GPU in one storage buffer (`clip_buffer`, binding 3, uploaded once) instead of per-call WGSL arrays. New rig param `yaw`: the pelvis and legs turn about the vertical (the reference walks with the hips turned to its steps, chest on the aim); the bone rig (`skeleton.rs`) now also has side, leg roll, yaw and tilt. The fitter plants boots (second pass) and sets body lift from the reference contact (`height`), so the planted boot sweeps evenly and runs have a flight phase. `GAIT_STRIDE` is the pinned sweep. Audit: planted-boot slide 0.03 to 0.25 m/s (limit 0.2 or 5.5 % of speed); `scripts/bake_gaits.sh <pack>` reproduces the tables exactly. Crouched walk stays procedural (the reference's deep long steps slid or popped on our rig).

- AN-03 (this PR): skeletal animation, see `docs/work-orders/AN-03-skeletal-animation.md`. Soldiers up to 250 m
  are the Mixamo-rigged soldier playing the baked clips on the GPU; this replaces the S12/S13 fits for what you see.
- AN-04 (2026-11-22): reverted. Every soldier uses the procedural commander animation (#94); Patch: Animations removed
  the GPU skinning runtime (`skin_gpu.rs`, `skin_cs.wgsl`, the skinned draw path). `skin.rs`, `rigged.skin`,
  `anims.bin` and the skin_* examples stay as reference tools. Same patch: the baked gaits are mirrored and smoothed
  in foot space (`polish_loop`), and the figure's cuts are closed by `bake_seams` / `pose_mesh_vertex`.

Not verified: the in-game look. On 2026-11-18 the project lead reported the forward crawl looked unchanged in the model viewer after #71. The deploy for `ffcd47e` succeeded and the page referenced that build, so a cached page is the likely cause, but this was not confirmed. Visual QA is outstanding.

Known issues to fix next: crouched walk is procedural (try a shorter-stride crouch clip or fit with a stride limit); left and right crawl still use the procedural legs (no reference clip; to be built by hand); the rolled boot lies on its side. Also: the table also overrides the left shoulder and elbow (support hand may look wrong); other crawl directions are still procedural. Measured on 2026-11-20: hips and upper body sit at flat-prone height; only the shin was an outlier.

Reference data: [mixamo-prone-reference.md](docs/handoff/mixamo-prone-reference.md). Raw Mixamo files are not in the repo and must not be committed. A scratch copy exists on the working machine only; delete it when tuning is finished.

## Next

1. Without a browser, check poses with `python3 scripts/stance_sheet.py out.png 5+36 32+36@0.25` (side and three-quarter views). Confirm in the model viewer when possible.
2. Fix pass on S06 (above), then stand/crouch to prone transitions.
3. Implement the finished animations in the game so AI work can continue (project lead priority, 2026-11-20).

## Planned: ReMotion (motion file format and animation suite)

Approved by the project lead on 2026-11-20 as the plan, but implementation is deferred until the animation fixes and the animation-to-AI hookup are done. M0 spec: [docs/superpowers/specs/2026-11-18-remotion-m0-design.md](docs/superpowers/specs/2026-11-18-remotion-m0-design.md). No code written. Longer design in the Claude project doc `remotion-design.md`.

## Direction

After alpha, Scalia ships as a native Windows and Linux program for Steam, and the browser becomes a secondary tactical space. Keep the simulation, file formats and loaders free of browser dependencies.

## Open decisions (project lead)

- Licence for the repository (none is set yet).
- ReMotion M0: the three open decisions in the spec (repo location, GPU path, baseline first) stand at their recommended values unless changed.
- Which merged remote branches to delete (see [CONTRIBUTING.md](CONTRIBUTING.md#branch-retirement)).

Older handoff material: [docs/archive/](docs/archive/).
