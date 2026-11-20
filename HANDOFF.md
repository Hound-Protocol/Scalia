# Scalia Agent Handoff

Start at the [README](README.md) for all workstreams and [CONTRIBUTING.md](CONTRIBUTING.md) for how to deliver changes. Docs index: [docs/README.md](docs/README.md). Verify this page against Git and CI before relying on it.

Last updated: 2026-11-20.

## Current animation lane (AN-02, prone and crawl)

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

Not verified: the in-game look. On 2026-11-18 the project lead reported the forward crawl looked unchanged in the model viewer after #71. The deploy for `ffcd47e` succeeded and the page referenced that build, so a cached page is the likely cause, but this was not confirmed. Visual QA is outstanding.

Known issues to fix next: back, left and right crawl still use the procedural legs (bake them the same way from `Crawl Backwards In Prone`, `Prone Left Turn` etc.); the rolled boot lies on its side. Also: the table also overrides the left shoulder and elbow (support hand may look wrong); other crawl directions are still procedural. Measured on 2026-11-20: hips and upper body sit at flat-prone height; only the shin was an outlier.

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
