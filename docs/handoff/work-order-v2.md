## Work order from 22:35, 14 Nov (user; no development until the limit resets)

Sounds already in the repo for this (commit 4fe59c5): `web/sfx/menu-flash1-door.mp3` (flash 1, "cinematic hit door"), `menu-flash2-thump.mp3` (flash 2, "massive thump"), `space-ambience-01.mp3`, `space-ambience-03.mp3`.

### A. One animation library for everything (top priority)
- **Problem:** the commander's motion (momentum, cut and skid, lean into turns and with acceleration, weighted turning with the foot shuffle, layer blends, aim pivot, recoil) lives in `viewer.rs`. Soldiers in `assault.rs` get a thinner copy, so they don't animate the same way.
- **Goal:** one animation library. Every figure (commander, soldiers, enemies, later crews) asks it for its pose from the same inputs: velocity, wanted velocity, heading, aim, stance, weapon, action, wound state. Nothing is posed by hand per caller.
- **Steps:**
  1. Audit and list what each path does today.
  2. Build a shared `Animator` per figure in a new module (`scalia-wasm/src/anim.rs` or `scalia-render`). It owns:
     - locomotion state: the pose with hysteresis, gait phase, cadence;
     - turn tracking: spin, shuffle, bank and pitch;
     - skid and brake;
     - the upper-body layer and its blend;
     - the shot kick;
     - transitions;
     - muzzle and bore.
  3. Port the commander onto it, with no change in look.
  4. Port the soldiers.
  5. Delete the duplicates.
  6. Tests: same inputs give the same pose for both; regression tests for everything ported.
- **Animation LOD:** full detail near the camera; distant figures get cheap poses (no shuffle, no IK, coarser updates). Performance stays first.

### B. Animation coverage: finish it, connect it, polish it
For each animation, report its real status: complete animation / static pose / partial / exists but not connected to AI. A static pose is never reported as done. Visual proof: a filmstrip or GIF from the model viewer and in game.
- **Locomotion:**
  - idle, walk, jog, run, sprint;
  - crouch idle, crouch walk, crouch run;
  - prone idle;
  - crawl forward, back and sideways, rifle placed correctly (crawl exists, forward only);
  - backpedal and strafe (have);
  - turn in place (shuffle exists);
  - stand ↔ crouch ↔ prone transitions in BF3 style, hand to the ground, including from the aim.
- **Combat:**
  - aim and fire standing, crouched, kneeling and prone (prone firing to check);
  - rifle reload (have; commander's R still missing);
  - pistol fire and reload;
  - weapon switch;
  - recoil (done for all);
  - raise and lower (done);
  - grenade throw (pose exists, connect it);
  - suppressive fire;
  - handling per weapon class: pistol, rifle, MG, launcher.
- **Tactical:**
  - drop to prone and to crouch;
  - dive behind cover;
  - peek and lean left and right (leans exist);
  - vault and step over low obstacles;
  - bounding;
  - kneel to observe;
  - get in and out of vehicles (later).
- **Casualty:**
  - wounded idle (have), wounded crawl, limp;
  - hit reactions, falls (have), death variations;
  - drag (have) and carry wounded;
  - kneel to treat (first aid exists), apply kit, help up.
- **Hooked to AI decisions:** incoming fire must be able to break into a walk and trigger a reaction (drop, dive, take cover).
- **Quality:**
  - transitions;
  - foot planting with no slide;
  - hands on the weapon;
  - variety between soldiers;
  - adapting to terrain;
  - readable combat.
- **Still open from earlier:** commander reload pose; gun aim pivot checked in game.

### C. AI: the reference autonomous squad (project lead's feedback, M2 and M3)
- **Before building:**
  - Audit what the battle AI already has: perception and sight, cover, GOAP capture, suppression and nerve, bounding, garrison, the leader's steps.
  - Find what is missing or disconnected and propose the steps.
  - Don't rewrite systems that work.
- **Reference scenario in the AI lab (`?lab`):**
  - 12 soldiers and a sergeant, one objective, an enemy formation, the player gives one order.
  - It must show, reproducibly and deterministically:
    1. taking the order;
    2. moving to the objective;
    3. spotting the enemy;
    4. each man picking his own cover;
    5. coordinated fire;
    6. reacting to incoming fire;
    7. suppressing;
    8. casualties handled;
    9. regrouping;
    10. completing or abandoning the objective depending on how it goes.
  - Each of these gets a test.
- **Gaps to check against the feedback:**
  - tactical memory (last known enemy positions, limited and contextual);
  - morale and psychology (confidence, fear, fatigue, cohesion, experience; Composure and veterancy are already planned);
  - knowledge only from sight or the radio;
  - reports travelling up the chain of command;
  - interruptible plans and recovery when an action fails;
  - soldiers not acting in lockstep (individual timing);
  - avoiding friendlies.
- **Then:** platoon (lieutenant), company (captain), general, all on the same principles.

### D. Main menu and intro sequence v2
- **Fixes:**
  - The menu music doesn't start right away and can't be heard in the menu: find out why (autoplay unlock, element reuse, preload) and fix it.
  - The loop cuts off early: make it gapless (Web Audio buffer loop, or trim the encoder padding) so it restarts the moment it ends.
- **Order:** the prototype caution window shows on its own (black background, no galaxy); after "Understood", the menu fades in.
- **Start button:** a slightly stronger pulse; on hover or focus the text changes from "Start" to "Deploy" (with a small transition).
- **On Start / Deploy (camera speeds follow the user's curve, slow then faster and faster):**
  1. **Pull-back.**
     - Flash 1 with `menu-flash1-door.mp3`; the menu music stops.
     - For 1 s the camera pulls back smoothly to see the galaxy at 20° tilt.
     - Letterbox bars slide in slowly (cinema black bars) and stay for the sequence.
  2. **Galactic view.**
     - Flash 2 with `menu-flash2-thump.mp3`.
     - The camera looks at home from an isometric galactic angle and orbits it extremely slowly to the right.
     - `space-ambience-01.mp3` plays until the next scene.
  3. **The planet.**
     - A flash, then the commander's planet; hold 2 s.
     - The deep-space ambience plays while it's in view.
     - Then the zoom in as today (travel sound, enter whoosh, eyes opening).
     - As it zooms, the ambience fades in volume and through a closing band-pass: highs and lows cut equally until only about 5 kHz is left, where it ends.
  - The engine needs:
    - a planet-from-orbit shot it can hold and orbit;
    - camera control from the page for the galactic and isometric angles (or keep those parts in the WebGL galaxy and hand over at the planet).
- **More polish overall** (particles, subtle grain or vignette on the menu, title reveal).

### E. Performance panel v2
- Same width; full height from the top of the screen (laid out for 16:9 1080p).
- Shows what is happening on screen, for both of us to analyse:
  - frame time broken down: CPU sim, battle step, animation, mesh build, uploads, GPU passes;
  - draw calls, instance and triangle counts;
  - counts of soldiers, effects and lights;
  - JS heap;
  - long frames with what caused them;
  - per-system timings over time;
  - errors and warnings inline;
  - copy-to-clipboard of a snapshot.

### F. ChatGPT feedback
The feedback doc (project lead) is folded into A to C above.

### G. Performance audit (ChatGPT, against 34054a9): fix plan
I checked the claims against the code on 14 Nov:
- **Confirmed: cover search is O(N²).**
  - `battle.rs` `think()` (about line 953) collects every soldier's cover each time one soldier picks cover.
  - Fix: build the occupied-cover set once per decision pass and update it as cover is claimed. Results must stay identical (determinism test: the battle hash is unchanged).
- **Partly confirmed: simulation timing.**
  - The battle already runs in fixed 50 ms steps, independent of frame rate (`assault.rs` STEP_MS).
  - The world needs-drain (`scalia-core/src/world.rs` `drain`) is ticked from the render path (`viewer.rs` around lines 1287 and 2388), and integer truncation loses the fractions.
  - Fix: fixed-step the world tick, keep the remainders, and add a test (the same game time at 30 fps and at 144 fps gives an identical state).
  - Lower stakes than "critical" (needs, not combat), but correct.
- **Shared scene preparation:** the main view and the tablet's view can rebuild the same instance lists. Fix: prepare the scene once per frame, with only the camera-specific parts per view. Measure first with the new performance panel (E).
- **Allocations:** temporary vectors and HashMaps every frame, position clones, string-keyed GPU buffers (`gpu.rs` `drop_buffer(format!(...))`). Fix:
  - persistent scratch buffers;
  - typed buffer handles instead of formatted strings;
  - reuse spatial buckets.
- **Navigation:** A* allocates fresh arrays and queues per search. Fix: reusable search scratch; use the existing squad flow fields more; cache paths.
- **GPU errors:** surface acquire failures are lumped together. Fix:
  - on Timeout, skip the frame;
  - on Outdated or Lost, reconfigure;
  - on OutOfMemory, or when the device is lost, go to the crash screen with the reason;
  - wire this into errors.js.
- **Big files:**
  - `viewer.rs` (4,191 lines) splits naturally: the commander and the gun go to the animation library work (A); then the menu and intro, the camera and cinematics, the tablet and map.
  - `leader.rs` (1,824) and `battle.rs` (1,727) split by system as the AI work (C) touches them.
- **Benchmarks:**
  - Add `battle_bench` (release, short run) to `scripts/verify.sh`, with a failure threshold.
  - Report median and p99 step time, allocations per step, and long runs with many squads.
  - Keep the aggregate and full-combat benchmarks separate.
- **Keep as is:** the struct-of-arrays groups, compact soldier records, integer determinism, spatial index, instancing, floating origin, terrain chunk cache, render LOD, GPU timestamps.
- **Order:**
  1. cover-scan fix and world fixed-step (small, safe, test-backed);
  2. benchmarks in verify;
  3. the performance panel (E) for measuring;
  4. allocations and shared scene preparation, guided by the panel;
  5. nav scratch;
  6. GPU error handling;
  7. splitting the big files alongside A and C.
