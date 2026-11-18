# AN-02 reference: prone and crawl joint ranges (measured)

Status: reference notes for tuning the crawl and prone poses in `pose.rs` (mirrored in `common.wgsl`). These are our
measurements of a set of reference clips, not the clips themselves. The raw files are not in this repository and are
not redistributed. Our figure's proportions win over the reference, as always.

Method: each clip's joint swing is measured as the angle from its first frame, per joint, across the whole clip
(minimum to maximum, degrees). The swing is the total rotation, so a large number means the joint moves a lot.

| Clip | Dur (s) | Hips | Spine | Head | L UpLeg | L Leg | R UpLeg | R Leg | L Arm | L ForeArm | R Arm | R ForeArm |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Crawl Backwards In Prone | 0.97 | 0-50 | 0-30 | 0-36 | 0-62 | 0-44 | 0-66 | 0-27 | 1-71 | 0-18 | 0-42 | 0-31 |
| Prone Forward | 2.33 | 0-28 | 0-17 | 0-27 | 1-22 | 0-18 | 0-70 | 1-91 | 1-53 | 1-25 | 1-60 | 0-33 |
| Prone Left Turn | 1.03 | 0-16 | 0-7 | 1-7 | 1-30 | 1-14 | 0-18 | 0-15 | 0-50 | 0-18 | 1-47 | 0-23 |
| Prone Right Turn | 1.33 | 0-8 | 0-12 | 0-11 | 1-17 | 1-21 | 2-30 | 0-24 | 1-46 | 0-52 | 0-40 | 0-19 |
| Prone Backwards Stop | 1.23 | 0-25 | 0-19 | 0-31 | 0-55 | 0-51 | 0-49 | 0-32 | 1-57 | 1-34 | 0-47 | 0-32 |
| Prone Idle | 4.93 | 0-1 | 0-0 | 0-2 | 0-1 | 0-2 | 0-1 | 0-1 | 0-3 | 0-2 | 0-2 | 0-1 |
| Prone Firing Rifle | 0.43 | 0-1 | 0-1 | 0-2 | 0-1 | 1-2 | 0-0 | 0-2 | 0-3 | 0-6 | 1-5 | 0-2 |

## What this says for our rig

- **Knee draw is the largest leg move.** Forward crawl has the right knee swing up to about 91 degrees, the left
  only about 18. Our current crawl (`pose.rs`, knee `0.85` rad at full draw) is in range, but the clip's asymmetry
  suggests a per-leg draw rather than one shared draw.
- **Elbows move little.** Forearm swing is small in every clip (under 35 degrees except left forearm in the turns and
  crawl-backward). Planted elbows should stay planted, which matches the current crawl note in HANDOFF.
- **Spine and head stay low.** Spine swing is under 30 degrees and the head moves less, so the chest should stay
  close to the ground while the arms and legs do the work.
- **Idle and firing barely move** (under 6 degrees everywhere). Keep them static; the breathing and sway belong to
  AN-01.

## Not yet done

- These are single-clip ranges. Cadence (the period of the cycle) and the timing between left and right are not in
  this note yet.
- Stand-to-prone and crouch-to-prone transitions are not measured here.
- The rig mapping (reference bone names to our joints) is not written yet.
