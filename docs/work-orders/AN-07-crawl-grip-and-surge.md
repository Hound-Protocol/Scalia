# AN-07: the crawl grips the ground and the body surges over it

- **ID:** AN-07
- **Revision:** v1.0
- **Status:** shipped (Patch: Animations 7), in-game verdict pending
- **Date:** 2026-11-26
- **Owner:** animation lane
- **Base:** main 63a0efe (Alpha 0.1.49)

## Goal

The low crawl reads as a body pulling and pushing itself along, not a figure sliding with its limbs waving. Player
report after Patch: Animations 6: "it just looks like sliding with animation".

## Background (measured on main, `examples/crawl_motion`, root moving at constant speed)

- Planted contacts skated: a "pushing" knee swept 32 cm in toward the body while it pushed, a "pulling" elbow swept
  20 cm out. The old hold test only checked the forward axis, so the sideways sweep went unseen.
- The body moved at exactly the root's speed (1.00 to 1.00) and its height varied by 0.0 cm.
- A planted elbow cannot hold its spot under a passing shoulder: the chest is about 15 cm above the elbow and the
  upper arm 31 cm long, so the elbow must swing out as the arm folds.
- The shipped table had joints jumping between rows: thigh roll 1.12 rad, elbow 1.26 rad, the arm's swing 1.58 rad
  (a straightening arm flipping between two solutions).

## Work items (all done)

1. The pushing **boot** grips (inside edge on the ground, as a real low crawl pushes) and holds its spot while the leg
   straightens; the knee lies out on the ground and comes in. Resting legs lie straight, turned out, dragged with the
   body.
2. The **hand** (front of the forearm) grips and holds its spot through the pull; the elbow swings out round it.
3. **Surge:** the body moves at 1 - 0.6 cos(4 pi phase) times the crawl's average speed (fastest mid push, slowest as a
   knee draws up), zero offset with both legs back so starts and stops are unchanged. `PoseParams::bob` carries it
   (lying prone it runs along the body); common.wgsl mirrors it (`CRAWL_SURGE`).
4. **Rise:** hips lift 2.5 cm (peak at 0.22 of a cycle) and the chest 3.5 cm (0.31) per stroke: a wave from hips to
   chest.
5. **Head lag:** the head tips back 0.06 rad as the body speeds up, 0.04 of a cycle late.
6. Solver steadiness: joints resist moving from the previous row (`STEADY`), straight legs lie turned out
   (`LEG_TURN`), the elbow never bends past straight, the arm's swing is wrapped into (-pi, pi].

Not done: counter-rotation (shoulders against hips). The roll was rated right in Patch 6, and the propped pulling
forearm lifting its own shoulder matches the stroke.

## Acceptance checks (tests/crawl.rs)

- `pushing_boots_and_pulling_hands_hold_their_spot`: under 1.5 cm in both ground axes.
- `forearms_never_drag_along_the_ground`: the hand never moves ahead with the body while down.
- `the_body_surges_and_rises_with_the_stroke`: pelvis speed under 0.6 and over 1.4 times average; hips and chest rise
  1.5 to 4 cm, the chest after the hips; head nod over 0.06 rad, lagging the surge.
- Kept: nothing sinks, rifle clear, both legs back at the start and stop, blends smooth.
- `the_body_lies_flat_on_the_ground`: hips under 22 cm (measured 21.1 with the rise; on knees and elbows ~30).
- Battle hash unchanged (594771974983721861).

## Evidence

| Measure | Alpha 0.1.49 | AN-07 |
|---|---|---|
| Boot / knee slip while pushing | 32 cm (knee) | 0.9 cm (boot) |
| Hand / elbow slip while pulling | 20 cm (elbow) | 0.3 cm (hand) |
| Pelvis speed over a stroke | 1.00 to 1.00 x | 0.40 to 1.60 x |
| Hips / chest height span | 0.0 / 0.0 cm | 2.0 / 2.8 cm |
| Largest joint jump between rows | 1.58 rad | 0.44 rad |
| Rifle clearance | > 1 cm | > 1.5 cm |

Filmstrips: `scripts/anim_sheet.py` with specs `32@<phase>#<STRIDE*phase>` (the figure drawn where he is).

## Next action

Player verdict in game: does the crawl now read as pulling and pushing? Then AN-06 (rifle forearm in standing
carries).
