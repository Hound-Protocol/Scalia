# AN-04: polish prone and the procedural low crawl

- **ID:** AN-04
- **Revision:** v1.0
- **Status:** shipped (Patch: Animations 6, Alpha 0.1.49); player verdicts: crawl arms right, roll right, starts and stops smooth
- **Date:** 2026-11-25
- **Owner:** animation lane
- **Base:** main 56b0f13 (Patch: Animations 5, Alpha 0.1.48); branch anim-an04

## Goal

The unaimed crawl reads as a soldier crawling: the body rolls and bends with each stroke, the rifle stays in its carry,
the forearms stay planted, and setting off and stopping show no dip of a knee, elbow or rifle into the ground. Judged by
the four-view renders (front, side, back, three-quarter) and by the player in the game.

## Background (measured on main)

- Upper arms: worst edge stretch in the crawl is x1.8 (was x5.1), guarded by `upper_arms_keep_their_shape`.
- Rhythm: 1.35 strokes/s at `CMD_CRAWL` (0.55 m/s), guarded by `the_crawl_keeps_an_unhurried_rhythm`.
- Blends: no joint faster than 15 rad/s; the knee, elbow and boot may press up to 5 cm deeper than either end of a blend,
  and the rifle no more than 1 cm below its lowest end (`starting_and_stopping_the_crawl_blend_smoothly`). The 5 cm is a
  known allowance, not a target.
- Roll: `ROLL` 0.17 rad and `BEND` 0.09 rad, about 3 cm lift at the hip and 4 cm swing at the shoulders. The player's
  verdict is "too little".
- Open defect: at phase 0.75 the rear elbow and rifle drop toward the ground line (three-quarter view). Not measured yet.

## Scope

In:
- Measurement tooling: an elevation option for `scripts/closeup.py` (true top view), and a numeric dump of lowest points
  per joint and per phase.
- Start and stop timed to the stroke (`anim.rs` blend start, `pose::blend` use in the crawl).
- Rifle carry during the stroke, so no part of it drops at phase 0.75.
- Roll and bend: a larger hip share and a smaller chest share, so forearms do not slide.

Out:
- Standing turns and standing/crouch transitions (AN-05).
- The rifle forearm resting in the vest in standing carries (AN-06).
- Crawl sideways and backward beyond the current table.
- Any change to `hash_battle`'s battle logic (render only).
- The M5 performance and LOD work and the M6 missing clips (AN-03).

## Files and entry points

- `scripts/closeup.py`: add an elevation parameter (for example `ELEV` in degrees) so a row can be a true top view.
- `crates/scalia-render/examples/`: a new `crawl_low_points.rs` that prints, for phases 0.00 to 1.00 in steps of 0.01, the
  lowest point of each knee, elbow, boot, the rifle and the head, and each forearm's horizontal slide per stroke.
- `crates/scalia-render/src/crawl.rs`: `ROLL`, `BEND`, `HEAD_STEADY`, `ROLL_LAG`, `DRAW`, `PUSH`, the rifle carry, and
  the `drawn()` timing used by the roll. Regenerate `HIGH_CRAWL` with `/tmp/crawl_regen.sh` or the `crawl_solve rows`
  example after any change.
- `crates/scalia-render/src/pose.rs`: `blend`, `CRAWL_STRIDE`, `RIFLE_PITCH`, `RIFLE_ROLL`, and `SHOULDER_SPLIT`. Mirror
  every change in `crates/scalia-render/src/wgsl/common.wgsl`; `tests/wgsl_parity.rs` must stay green.
- `crates/scalia-wasm/src/anim.rs`: the start and stop timing (the stroke phase at which a transition starts or finishes).
- `crates/scalia-render/tests/crawl.rs`, `tests/arms.rs`: tests for each change (see Acceptance).

## Work items

Each item ends with a test that failed before the change and passes after it.

1. **Measure first.** Add the elevation option and the numeric dump. Record the 0.70 to 0.80 rifle and rear-elbow lowest
   points as the baseline. No code change in the crawl yet.
2. **Rifle at phase 0.75.** Find what drops the rifle and rear elbow in that window, and fix it so the lowest point stays
   at or above its value at phase 0. Test: `rifle_and_elbow_do_not_drop_in_the_stroke`.
3. **Start and stop matched to the stroke.** Setting off begins at the phase where both legs lie straight back; stopping
   completes the current stroke before the blend to prone. Test: tighten `starting_and_stopping_the_crawl_blend_smoothly`
   so the knee and elbow allowance drops from 5 cm to 2.5 cm.
4. **Stronger roll, hip-led.** Raise the hip share of the roll toward twice its current amount and lower the chest share,
   so the forearms stay planted. Gate: the forearm slide per stroke stays within the current limit (5 cm, test
   `planted_forearms_hold_their_place`). Test: the body-roll test asserts a hip lift of at least 0.05 m.
5. **Head.** Keep the head low and steady (`HEAD_STEADY`). Test: the existing eyes-level test stays green.
6. **Crawl bob.** The body's height over a stroke varies by no more than 1.5 cm (no bobbing). Test:
   `the_body_does_not_bob`.
7. **Prone turns.** Turning prone pivots on the hips: the boots and the rifle muzzle stay within their arcs, and nothing
   slides sideways. Test in `anim.rs`: `prone_turns_keep_the_hips_put`.
8. **Prone aim clearance.** Lying and aiming, the rifle, forearms and head stay above the ground at every aim pitch the
   game uses. Test: `prone_aim_clears_the_ground`.
9. **Prone idle.** Lying still, a slow breath (chest rise under 1 cm, period about 4 s) so he reads as alive, with no
   limb leaving its contact. Test: `prone_breathes_without_moving_his_contacts`.

## Acceptance checks

- [ ] `scripts/check.sh` passes.
- [ ] `cargo run -q --release -p scalia-bench --bin hash_battle` is 594771974983721861 (render only).
- [ ] Each work item's test is red before its change and green after it (shown in the commit messages).
- [ ] Numeric dump: no rifle or rear-elbow point drops in phases 0.70 to 0.80 below its phase-0 value.
- [ ] Numeric dump: start and stop blends stay within 2.5 cm for knee and elbow.
- [ ] Four-view render at phases 0, 0.25, 0.5, 0.75 (front, side, back, top): no visible rifle or elbow contact with the
  ground line, and the roll is visible from the front and back.
- [ ] Player verdict in the game on the crawl, roll, transitions and rifle carry.

## Evidence

Branch anim-an04 (Patch: Animations 6). Tools added: `scripts/anim_sheet.py` (filmstrips from any camera incl. top,
travelled distance, heading, ZMAX), examples `crawl_joints`, `crawl_blend`, `low_parts`, `prone_fit`.

Done, each with a test (red before, green after unless noted):
- Forearms pull half a cycle and reach half (`crawl::arm_stroke`): `forearms_never_drag_along_the_ground` (85 of 128
  steps dragged before, at most 8 now).
- Nothing pokes through the ground in the stroke (`ELBOW_H` 0.11, `ARM_CLEAR`): `nothing_sinks_into_the_ground_through_the_stroke`.
  The phase-0.75 "rifle drop" measured: the rifle never went under (lowest +0.045 m); the rear elbow did (-1.2 cm), fixed.
- Roll and bend about 1.5x Patch 5 (`ROLL`/`CHEST_ROLL` 0.26, `BEND` 0.14; sideways x0.6): body-roll test, hip lift >= 4 cm.
  Twice failed (rifle on the ground, forearms skating); the torso is one rigid part, so hips cannot lead the chest.
- Start and stop: the crawl sets off at stroke 0 and finishes its stroke before settling (`anim.rs` `settle_left`), the
  chest props up 0.1 rad mid-blend (`pose::crawl_prop`): blend test tightened from 5 cm to 1 cm; anim tests
  `the_crawl_sets_off_with_both_legs_back`, `stopping_the_crawl_finishes_the_stroke`.
- Prone fitted to the ground (`PRONE_REST_FIT`, `PRONE_AIM_FIT`): `tests/prone.rs` (aim support elbow was 7 cm up).
- Guards that passed on first run (no change needed): `the_body_does_not_bob`, `prone_breathes_without_moving_his_contacts`,
  and the existing `prone_turns_pivot_on_the_hips_and_rising_takes_the_shift_back` (item 7).

Open: at rest prone the right forearm still rests 2 cm into the ground (an arm fit that keeps the rifle low did not
converge); deployed and visually verified are pending the player's verdict.

## Next action

Player verdict in the game on the crawl, prone, starts and stops.
