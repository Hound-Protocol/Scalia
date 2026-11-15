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
  - Low: Crouch 3, CrouchWalk 11, CrouchAim 14, CoverAim 4, Cover 8, Prone 5, Crawl 32.
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
  - crawl forward only (low crawl on elbows; no back or sideways yet; the rifle is cradled, not aimed);
  - wounded idle; drag (crouch-walk legs plus drag arms);
  - prone drop/get-up transition: the controller and pose path are implemented and unit-tested for standing and crouched entry, but aimed-state coverage and current visual sign-off are still required (anim.rs tests; pose.rs drop keys).
- **Static poses (no motion of their own):**
  - cover, cover aim, prone, prone aim;
  - leans (peek), stack, signal, the radio and tablet holds, dead.
- **Exist but not driven by AI decisions yet:** grenade throw (there is an action path, but it needs checking in the lab), drone launch.
- **Missing:**
  - dive to cover, vault and step over obstacles, kneel to observe;
  - pistol (fire, reload) and weapon switching; handling per weapon class;
  - hit reactions while standing, limp, wounded crawl, carry, help up, death variations;
  - animation LOD (cheap poses for distant figures);
  - foot IK on slopes.
- **Hook still missing:** incoming fire can already change a soldier's mind (cover first), but there is no visual "flinch / drop" reaction yet.

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

- **AI-3 reference squad — complete 15 Nov.** `crates/scalia-ai/tests/reference_squad.rs` contains the fixed-seed ten-step engagement test and a separate outmatched-squad-abandons test. Do not rebuild this scenario; keep both tests as regression coverage.
- **AI-4 — active next work.** Continue from `docs/handoff/work-orders-ai-animation.md` in its stated order:
  1. tactical memory and contact ageing;
  2. sight/hearing/radio-only knowledge with delayed reports;
  3. Combat readiness;
  4. interruptible plans and recovery;
  5. platoon, then company command.
- Keep the reference squad as a regression test and preserve `hash_battle` unless a deliberate behavior change is documented.

## Rules of the road
- Every completed feature has a test; animation work gets visual proof (stance sheet or in-game capture); AI work gets a reproducible lab scenario.
- Don't rewrite working systems; keep the simulation deterministic (check `hash_battle` before and after).
- `scripts/verify.sh` must pass before each push; commits carry an `Update: <Area>` trailer, and the commit description goes into the in-game changelog, so write it for players.
