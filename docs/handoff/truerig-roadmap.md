# TrueRig animation roadmap

## What TrueRig means in Scalia

TrueRig is the name for Scalia's one character-motion pipeline, not a third-party engine or a second animation
system. Keep building on `scalia-wasm/src/anim.rs` (intent-to-frame control), `scalia-render/src/pose.rs`
(procedural rig and pose solving), and its `common.wgsl` mirror (GPU rendering). Every figure should use this same
path. Do not fork a separate player, AI, or skeletal animation controller.

The character is not a cursor. Inputs express intent; collision, momentum, facing limits, balance, and context decide
what motion is actually possible. The animator receives that realized motion plus the requested action, then makes
the body respond. Combat input should feel immediate and forceful without skipping weight, contact, or recovery.

## Existing foundation

- Shared animator for the commander and battle figures; weighted turns, acceleration, run-up, braking, skids, gait
  phase, pose blending, reload, recoil/flinch instance values, and stance drops.
- Procedural per-part pose parameters, two-hand weapon grips, barrel placement, and a hand-maintained WGSL mirror.
- Directional walk/run/crouch gait and a forward prone crawl; a prone aim pose exists but still needs a rig-tuned pass.
- The commander already collides against prop footprints and closed doors. That prevents body penetration but does not
  yet report a nearby surface normal or drive weapon clearance, wall avoidance, or sticky cover.
- Battle figures set `Drive.in_cover`; the commander does not yet have a surface query to set it. His low-cover pose
  response therefore depends on the later surface-awareness work.

## Ordered work

### 0. Keep one source of motion truth — in progress

- Keep controller intent, realized movement, animation state, procedural rig, and shader responsibilities explicit.
- Tune Rust and WGSL together; retain parity tests, animator tests, and numeric pose audits.
- No gameplay or AI simulation changes just to make a pose look different.

### 1. Stable combat stances — current slice

- Use the grounded kneeling aim in the open; preserve the distinct pop-up shot when aiming over low cover.
- Finish a supported kneeling firing hold, then tune the prone firing pose and rifle support.
- Check muzzle/bore placement and planted feet at rest and while moving; blend into and out of each hold.

### 2. Responsive combat reactions

- Connect shot events to readable hit flinches, balance recovery, and death/knockdown variations; keep recoil on the
  actual firing axis as well as visible in the body.
- Make reload and stance-change motion urgent and decisive. Animation timing must agree with the gameplay action;
  do not silently alter combat simulation timers to make animation appear faster.
- Acceptance: bounded transitions, no pose snaps under rapid input changes, and tests for interrupt/re-entry.

### 3. Surface awareness and weapon clearance

- Add a small, bounded near-body/swept query that returns distance and surface normal for walls and props. Reuse
  existing collision geometry where possible; body collision alone cannot tell the rig which way to lean.
- Before contact, have the character shorten/redirect movement and raise, cant, or shift the weapon to avoid clipping.
- Acceptance: test against a wall, a corner, and a protruding object from both shoulders; body and muzzle remain clear
  without jitter or trapping the player.

### 4. Contextual cover

- Enter cover after about 1 s of sustained direct or near-direct pressure into a usable surface; store the contact
  normal and keep the current stance.
- Move/aim along the surface. Break away after more than 0.5 s of input into the configured broad outward angle;
  preserve posture and resume the intended movement smoothly.
- Acceptance: deterministic enter/hold/exit tests, including corners, invalid cover, and interrupted pushes. Do not
  treat every collision as cover.

### 5. Contextual prone and recovery

- Turn `X` from an instant stance toggle into a speed-aware drop: a planted step, forward weight transfer, left-hand
  brace, then ground contact. Walking, jogging, and sprinting should have distinct timing and travel.
- Support front/side/back ground orientations and recover using the available hand/foot support. Add supine as an
  explicit state, not a flipped prone pose.
- Add slow low crawl and fast, higher 360-degree crawl; keep the weapon forward and solve arms/legs for forward,
  reverse, and lateral travel.
- Acceptance: interrupt the drop/recovery at every phase, keep contact with the ground, and test all travel
  directions with a firing muzzle that does not snap.

### 6. Integration and scale

- Wire the same reactions and contextual states to player and AI events; the AI decides *what*, TrueRig decides *how
  the body moves*.
- Add measured animation LOD only after close-range motion and shader parity are correct.
- Acceptance: reproducible tests, pose-audit numbers, a browser visual check for each finished movement, and no
  battle-hash change unless the gameplay simulation intentionally changes.

## Working rule

Treat all timings above as initial targets. Tune from pose audits and in-game movement rather than accumulating
one-off pose IDs. Keep X and cover behavior behind explicit state transitions so they can be interrupted without
teleporting, changing stance unexpectedly, or fighting the movement controller.
