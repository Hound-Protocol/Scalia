# AN-05: standing turns step round smoothly

- **ID:** AN-05
- **Revision:** v1.0
- **Status:** merged pending (branch anim-an04, Patch: Animations 6)
- **Date:** 2026-11-26
- **Owner:** animation lane
- **Base:** main 56b0f13; branch anim-an04 (shipped with AN-04)

## Goal

Turning on the spot, standing or crouched, the chest and rifle lead and the feet step round under him one at a time,
smoothly, with no frame where the legs or body jump. Today the legs rotate rigidly with the pelvis (`pp.yaw += lag`)
and the player sees the body and legs skip.

## Background (measured on main, probe in `anim.rs`)

- Per frame at 60 fps the legs' heading changes 0.05 to 0.1 rad while catching up: no jump in the maths.
- `Animator::dir` jumps 90 degrees in one frame when the shuffle starts and back to 0 when it ends (shuffle branch and
  the final `else { 0.0 }`). The standing pose ignores `dir`, crouch uses it in its gait.
- No foot is lifted while the legs turn: the lower body spins on the spot.
- Not yet checked: whether the facing that drives the turn arrives in 50 ms battle steps.

## Scope

In:
- A facing and record trace over a turn (battle soldier and commander) to find the source of the skips.
- Stepping round: a short in-place step cycle that lifts and places each foot while the lag closes.
- Easing `dir` into and out of the shuffle.
- Crouch turns (crouch uses `dir`).
- Heading overshoot check for the aim tracker and the turn spring.

Out:
- Gait changes while moving (walk, run, sprint loops).
- Prone turns (AN-04 item 7).

## Files and entry points

- `crates/scalia-wasm/src/anim.rs`: `steer`, the rifle-first lag, `catching`, `shuffling`, `dir`.
- `crates/scalia-wasm/src/assault.rs`: how battle facing reaches `Drive::face`.
- `crates/scalia-render/src/pose.rs` and `wgsl/common.wgsl`: the step-round pose (feet lifted and placed in turn).

## Work items

1. **Trace.** Per frame during a turn: the facing given, `heading`, `lag`, `dir`, the pose and phase sent. Find any
   value that moves in 50 ms steps or jumps.
2. **Smooth facing.** If facing arrives in battle steps, the animator follows it smoothly between them. Test:
   `turning_follows_stepped_facing_smoothly` (no frame-to-frame heading change above 2x the average).
3. **Ease `dir`.** No frame-to-frame `dir` change above 0.25 rad while standing or crouched. Test: `dir_eases_on_the_spot`.
4. **Step round.** While catching up, the feet step in turn: one foot lifts at least 4 cm and lands turned toward the
   chest, then the other; a planted foot does not turn more than 0.05 rad. Test: `feet_step_round_one_at_a_time`.
5. **Overshoot.** Aimed turns stop on the mark without swinging past by more than 0.03 rad. Test:
   `aim_turns_stop_on_the_mark`.

## Acceptance checks

- [ ] `scripts/check.sh` passes.
- [ ] `hash_battle` is 594771974983721861.
- [ ] Each test red before its change and green after.
- [ ] Turn filmstrip (frames every 1/30 s through a 90 degree turn, top and side) shows steps, no jumps.
- [ ] Player verdict in the game.

## Evidence

- Cause of the skip: on the spot he side-stepped in the walk gait with `dir` set straight to +-90 degrees, snapping back
  to 0 mid-blend as the shuffle ended; meanwhile the whole lower body spun on the pelvis yaw.
- New `Pose::TurnStep` (37, `pose::turn_step`, shader mirror): per-foot headings in `anim.rs` (`feet`, `step`); planted
  feet hold their heading, the foot on the turning side steps first and lands slightly into the turn, then the other.
  `LEG_LAG_MAX` 0.9 -> 1.3 (packing range). Test `standing_turns_step_round_one_foot_at_a_time`.
- Stepped facing (50 ms battle steps) was already followed smoothly: `turning_follows_stepped_facing_smoothly` (guard).
- Overshoot: aimed turns swung 0.054 rad past, unaimed 0.040: braking on half the pickup now, `aim_turns_stop_on_the_mark`.
- Crouched turns: no `dir` snap any more (crouch keeps the ease-round, no shuffle).

## Next action

Player verdict in the game on turning on the spot.
