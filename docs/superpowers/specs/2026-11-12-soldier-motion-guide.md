# Soldier motion guide

Status: working guide, 2026-11-12. Studied from Battlefield 3: DICE's own descriptions of their animation work, and
gameplay footage watched in the project lead's browser. Everything here is our own work on our own rig; nothing is
copied from the game.

## What DICE did (sources at the end)

- **A layered system, not single clips.** Battlefield 3 put EA's ANT system (the one from FIFA) into Frostbite 2,
  with heavy motion capture. ANT blends any number of animations together. DICE's general manager credits it for
  "how the soldiers move, how they go into cover, how they look under suppressing fire".
- **A few core poses, with motion added on top.** DICE builds every weapon's animation set on three base poses:
  idle, aiming down the sights, and sprint. Firing, reloads and sway are layers added on top of those. While
  sprinting, a separate layer moves the gun around.
- **Their feel targets:**
  - *stressed*: under fire almost all the time;
  - *tactical*: to the point and in control;
  - *weight*: consistent, and different for each weapon type;
  - *timing*: hit and hold strong poses, and move efficiently between them.
- **Other details from their write-ups:**
  - hands grip the weapon tightly, with natural wrist angles;
  - elbows stay low;
  - the muzzle moves fluidly with no jerks;
  - in hip mode the weapon feels pushed towards the right shoulder;
  - when an action is interrupted, it resumes from where it stopped rather than restarting.

## What we saw in the footage

- **Crouch walk:**
  - knees deeply bent and the stance wide;
  - the upper body stays upright and steady while the legs work;
  - the rifle is shouldered, pointing where the soldier looks.

  The upper body and the legs are clearly two separate layers.
- **Weight:** the body sinks on each step as the stance knee flexes. Nothing glides.

## Rules for our soldiers

1. **Two layers: legs and torso.** The legs carry the gait (walk, run, sprint, crouch walk). The torso, arms and
   rifle hold a stance (low ready, aim, sprint carry) chosen separately, so a soldier can walk while aiming.
2. **Weight in every step.** The stance knee flexes at mid-step, so the hips dip twice per stride. Feet stay planted;
   the body moves, not the ground.
3. **The rifle leads.**
   - low ready when walking;
   - high port, close to the chest, when running;
   - angled down across the body when sprinting;
   - shouldered at eye height when aiming.

   The right hand never leaves the grip, and the left hand supports the handguard.
4. **Elbows low, wrists natural.** No arms sticking out like columns.
5. **Hold, then move.** Pose changes ease in and out (smoothstep, not linear), with a slight overshoot as a pose
   settles.
6. **No lockstep.** Each soldier has his own stride rate, phase and reaction delay, so a squad ripples rather than
   marching like a machine. This is already partly in place.
7. **Stress shows.** Under fire (later): the head drops, the shoulders hunch and steps get shorter.

## Implementation order

1. Gait rework: the stance knee flexes (weight), swing timing is closer to real gait, the sprint is a separate pose,
   and pose blends ease in and out.
2. Rifle carries per pose (low ready, high port, sprint carry, aim) with the left hand on the handguard.
3. Upper and lower body split, so soldiers can walk while aiming. This needs a separate torso stance in the
   instance data.
4. Stress and suppression reactions, which come with combat.

## Sources

- [Animating weapons for Battlefield V](https://ea_dice.artstation.com/blog/jMao/animating-weapons-for-battlefield-v), DICE core gameplay animator Oskar Wetterbrandt
- [Battlefield 3: the five key strengths of Frostbite 2](https://www.theguardian.com/technology/gamesblog/2011/sep/23/battlefield-3-frostbite-2-talk), The Guardian, 2011
- GDC 2012 talks: [Animation methodology for Battlefield 3](https://gdcvault.com/play/1016323/Animation-methodology-for-Battlefield) and [Modular Rigging in Battlefield 3](https://gdcvault.com/play/1015573/Modular-Rigging-in-Battlefield) (summaries only; the videos are members-only)
- Footage: [BF3 vs BF4 Soldier Animation](https://www.youtube.com/watch?v=WoGULTnalfY) (frames watched in the browser)
