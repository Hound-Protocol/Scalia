# Scalia: AI and animation handoff (15 Nov, 01:00)

Short state of play for whoever picks up AI or animation work next. The repo is `Hound-Protocol/Scalia` (live at hound-protocol.github.io/Scalia). The work order is `docs/handoff/work-order-v2.md` (also `claude/scalia-work-order-v2.md` in the claude.ai project); the AI design is `claude/scalia-ai-design.md`; the AI audit from 14 Nov is `claude/scalia-ai-audit-2026-11-14.md`.

## Animation: how it works now
- **One library for every figure:** `crates/scalia-wasm/src/anim.rs` (commit 6f3eee6).
  - The commander (`viewer.rs` `update_commander`) and every soldier, ours and the enemy's (`assault.rs` `instances` via `drive_of`), each own an `Animator`.
  - The controller fills a `Drive`:
    - real velocity, and the velocity he is trying for (used for skids);
    - `Face`: Keep, Turn with a rate cap, or Aim;
    - `Stance`: Stand, Crouch, Prone;
    - `Rifle`: Ready, Aim, Reload, Slung;
    - whether sprinting is allowed, in cover, down or dead;
    - an optional `action`: a whole-body pose such as tablet, radio, throw, aid, drag, or the leader's cues (stack, peek, sweep, kneel, brief).
  - `step()` returns a `Frame`: pose, upper layer, move direction, phase and cadence, heading, lean, skid, footfall.
  - `record()` builds the 12-float instance record; `barrel()` gives the muzzle and bore.
- **What every figure now gets:**
  - weighted turning (28 rad/s², capped per situation);
  - an aim pivot that keeps pace with the mark (`motion.rs` `AimTrack`);
  - a foot shuffle when turning on the spot (only real turns, not scanning jitter);
  - a hard-stop brake pose and the low pivot in a cut;
  - a step out from still;
  - leg hysteresis: walk, run and sprint at 0.2/0.05, 2.4/2.1 and 4.0/3.7 m/s;
  - leaning into turns and pitching with acceleration;
  - pose blends and blends when the rifle comes up or down (0.3 s);
  - falls (back when shot down, forward when killed; dead on his back stays on his back);
  - footfalls, and the recoil kick (packed in the pose id's fraction).
- **Rendering:**
  - Poses are procedural: `crates/scalia-render/src/pose.rs` builds `PoseParams` per pose and phase.
  - The shader mirror is `crates/scalia-render/src/wgsl/common.wgsl`. Parity tests keep the two equal.
  - Any pose change goes in both files.
  - `scripts/stance_sheet.py` and `examples/stance_dump.rs` render poses (spec `pose[+upper][@phase]`).
  - `examples/anim_audit` checks foot slide, loop seams, grips and blends.
- **Pose ids:**
  - Standing and moving: Stand 0, Walk 1, Run 2, Sprint 7, StandAim 6, Ready 30, Slung 28, Brake 31.
  - Low: Crouch 3, CrouchWalk 11, CrouchAim 14, CoverAim 4, Cover 8, Prone 5, Crawl 32, ProneAim 36.
  - Leans and CQB: LeanLeft 12, LeanRight 13, Stack 15, Sweep 16.
  - Actions: Radio 9, Tablet 10, Throw 17, Reload 18, Signal 19, Drone 20, Aid 23.
  - Casualties: Wounded 21, Drag 22, Dead 24, DeadBack 27, FallBack 25, FallForward 26.

## Animation: honest status
- **Complete and moving:**
  - walk, jog/run, sprint and crouch walk, in any direction relative to facing (strafe and backpedal);
  - turn in place (shuffle), Ready idle, aim standing and crouched, recoil;
  - raising and lowering the rifle (blend), falls, the commander's hard-stop brake;
  - reload (now the commander's too, through the battle's reload state).
- **Partial:**
  - wounded idle; drag (crouch-walk legs plus drag arms); wounded crawl.
- **Static poses (no motion of their own):**
  - cover, cover aim; leans (peek), stack, signal, the radio and tablet holds, dead.
- **Exist but not driven by AI decisions yet:** grenade throw (there is an action path, but it needs checking in the lab), drone launch.
- **Missing:**
  - dive to cover, vault and step over obstacles, kneel to observe;
  - pistol (fire, reload) and weapon switching; handling per weapon class;
  - hit reactions while standing, limp, wounded crawl, carry, help up, death variations;
  - animation LOD (cheap poses for distant figures);
  - foot IK on slopes.
- **Hook still missing:** incoming fire can already change a soldier's mind (cover first), but there is no visual "flinch / drop" reaction yet.

### AN-2 implementation notes (15 Nov)
- Prone crawl forward/back/side, the ProneAim pose and aimed get-down/get-up transitions are implemented; automated checks are the merge gate, with visual in-game sign-off still outstanding.
- `ProneAim` is pose 36; it is a real two-handed prone firing pose and can layer over directional `Crawl`.
- `Crawl` uses the animator's travel direction to reverse diagonal timing when backing up and to change support width when crawling sideways.
- Aimed drop/get-up transitions use aimed endpoints and latch their animation path so reversing halfway up/down does not switch to the non-aimed keys.
- `scripts/check.sh` and the animation audit are the merge gates; visual browser review remains a separate sign-off.

## AI: what exists (crate `scalia-ai`)
- **`battle.rs`:** the close battle in fixed 50 ms steps, deterministic (integer mm/ms; native and WASM hashes match).
  - Covers perception (sight grid with height bands, hearing, contacts with confidence, shared per squad), fire, hits, suppression and nerve, wounds and bleeding, first aid and dragging, cover use, bursts, reloads, personal space, and doors.
- **`mind.rs`:** each soldier's F.E.A.R.-style mind: facts, goals with live priorities, a small planner. Wants: survive (take cover), fight, reload, and so on. Thought every perception tick.
- **`leader.rs`:** the squad leader turns intent into plans and drills:
  - GOAP capture;
  - move with security on arrival;
  - caution, and bounding when hostiles are confirmed;
  - garrison (windows, doors, sandbags);
  - building clearing (stack, breach, sweep, second element);
  - approach through dead ground; defenders posted back from openings.
- **`tactics.rs`:** the cover map (spots along walls, crates and sandbags; protection against threats). **`goap.rs`:** the planner. **`grid.rs`:** the spatial index. **`save.rs`:** commands, replay, save/load, hash.
- **Lab:** `?lab` in the browser is the AI lab for watching battles; `scaliaField()` in the console lists the squads.
- **Benchmarks:** `cargo run --release -p scalia-bench --bin battle_bench` (now with p50/p99/worst). The 1,000-in-contact p99 is checked in `scripts/verify.sh` (budget 35 ms). Today: 1,000 in contact about 5 ms median and 10 ms p99; 2,000 about 11 ms median.
- **Today:** cover search builds the taken list once per pass (was O(N²)); the battle hash is unchanged.

## AI: what is next (work order C, project lead's feedback)
1. Audit against the feedback's list before building (most of perception, cover, suppression and the leader exist).
2. The **reference squad** in the lab: 12 soldiers and a sergeant, one order, an enemy formation. Reproducible, with a test for each step:
   1. order
   2. move
   3. detect
   4. own cover
   5. coordinated fire
   6. react to fire
   7. suppress
   8. casualties
   9. regroup
   10. complete or abandon
3. **Gaps:**
   - tactical memory (last known positions; limited);
   - morale and psychology (fear, fatigue, cohesion, experience; ties into Composure and veterancy);
   - knowledge only by sight or radio, with reports up the chain;
   - interruptible plans and recovery from a failed action;
   - individual timing so men don't act in lockstep.
4. Then platoon (lieutenant) and company (captain) levels on the same principles.

## Rules of the road
- Every completed feature has a test; animation work gets visual proof (stance sheet or in-game capture); AI work gets a reproducible lab scenario.
- Don't rewrite working systems; keep the simulation deterministic (check `hash_battle` before and after).
- `scripts/verify.sh` must pass before each push; commits carry an `Update: <Area>` trailer, and the commit description goes into the in-game changelog, so write it for players.
