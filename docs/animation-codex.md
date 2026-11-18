# Scalia Animation Codex

Status: Canonical Reference
As of: 2026-11-17
Applies to: Soldiers, Hostile Forces, Commander, Boarding Crew, and Ship/Vehicle Personnel

## 1. Architecture and Rig Principles

Scalia renders thousands of soldiers across galactic, orbital, and planetary engagements using a unified procedural rig (`crates/scalia-render/src/pose.rs`) mirrored in WGSL (`crates/scalia-render/src/wgsl/common.wgsl`) and driven by a single per-figure state machine (`crates/scalia-wasm/src/anim.rs`).

- **Single Shared Rig:** Allies, enemies, and the commander share the same `Animator` pipeline (`Drive -> Frame -> 12-float instance record`). Role differences come from controller inputs (`Drive`), weapon class, equipment, and tactical state.
- **Shader-Side Evaluation:** Skeletal transforms, procedural gaits, two-bone IK (`crouch_gait`), left-hand handguard IK (`grip`), upper/lower body layering (`pack_layer`), recoil kicks (`pack_kick`), and near-miss flinches (`pack_flinch`) execute in the vertex shader so battle scale stays O(1) on the CPU.
- **Deterministic Separation:** Visual reactions (recoil, flinch, near-miss ducking, idle breathing, banking) never alter deterministic simulation hit rolls or `hash_battle`.

---

## 2. Implemented Animation Inventory

### 2.1 Base and Layered Poses (`Pose` enum `0`..`36`)

| ID | Variant | Type | Status | Roles | Description and Current Behaviour |
|---|---|---|---|---|---|
| 0 | `Stand` | Full / Base | Implemented | All | Upright rest stance with rifle carried at safe hang and subtle idle breathing. |
| 1 | `Walk` | Locomotion | Implemented | All | Directional walking gait (`gait_dir`) with planted-foot stride scaling (`stride_scale`) and hip bob. |
| 2 | `Run` | Locomotion | Implemented | All | Combat jog/run with forward lean, higher knee lift, and collapsed low-ready rifle carry. |
| 3 | `Crouch` | Full / Base | Implemented | All | Deep kneeling stance (right knee down, left foot planted) with low-ready rifle carry. |
| 4 | `CoverAim` | Full / Upper | Implemented | Soldiers, Enemies | Crouched behind low cover, leaning up/forward to aim over the lip. |
| 5 | `Prone` | Full / Base | Implemented | All | Belly-down grounded stance (`PRONE_LIFT = 0.23`), supported on elbows with relaxed leg splay. |
| 6 | `StandAim` | Full / Upper | Implemented | All | Bladed athletic firing stance (`AIM_LEAN`, cheek weld `AIM_HEAD`), layerable over `Walk` and `CrouchWalk`. |
| 7 | `Sprint` | Locomotion | Implemented | All | Full-speed sprint with rifle angled across chest (`Carry::Rush`) and strong forward pitch. |
| 8 | `Cover` | Full / Base | Implemented | Soldiers, Enemies | Standing pressed behind high cover with rifle at high ready (`Carry::HighReady`). |
| 9 | `Radio` | Full / Upper | Implemented | Leaders, Commander | Left hand holds radio handset to ear while right hand retains rifle; layerable while walking. |
| 10 | `Tablet` | Full / Action | Implemented | Commander, Leaders | Both hands hold command PDA (`TABLET_AT`) at chest height with rifle slung on back. |
| 11 | `CrouchWalk` | Locomotion | Implemented | All | Two-bone IK crouched gait (`crouch_gait`) keeping boots planted flat in any travel direction. |
| 12 | `LeanLeft` | Full / Cover | Implemented | All | Weight shifted onto left foot with upper body tilted left (`-LEAN_TILT`) aiming around corner. |
| 13 | `LeanRight` | Full / Cover | Implemented | All | Weight shifted onto right foot with upper body tilted right (`+LEAN_TILT`) aiming around corner. |
| 14 | `CrouchAim` | Full / Upper | Implemented | All | Half-crouch firing stance for shooting over waist-high walls; layerable over `CrouchWalk`. |
| 15 | `Stack` | Full / Tactical | Implemented | Soldiers, Enemies | Stacked at breach point: half-crouch, weight forward, low-ready rifle. |
| 16 | `Sweep` | Full / Tactical | Implemented | Soldiers, Enemies | Room-clearing aim pose sweeping upper body left and right (`SWEEP = 0.6 rad`) over cycle. |
| 17 | `Throw` | One-Shot Action | Implemented | All | Five-keyframe overhand grenade throw with rifle slung on back and follow-through. |
| 18 | `Reload` | Upper / Action | Implemented | All | Five-keyframe magazine change: left hand moves from well to chest pouch and back; layerable. |
| 19 | `Signal` | Upper / Action | Implemented | Leaders, Commander | Left-hand tactical gesture (raised fist hold followed by forward advance chop); layerable. |
| 20 | `Drone` | Full / Action | Implemented | Specialists, Cmdr | Kneeling drone launch: rifle slung, both hands raise drone overhead and release. |
| 21 | `Wounded` | Full / Casualty | Implemented | Soldiers, Enemies | Incapacitated on back (`prone = -1.0`), one knee drawn up, hand pressed to abdomen, weak breathing. |
| 22 | `Drag` | Upper / Action | Implemented | Soldiers | Crouched backwards casualty drag gripping vest straps; layers over `CrouchWalk` legs. |
| 23 | `Aid` | Full / Action | Implemented | Medics, Soldiers | Kneeling over casualty applying two-handed pressure/treatment over cycle. |
| 24 | `Dead` | Full / Casualty | Implemented | Soldiers, Enemies | Face-down limp ragdoll pose with splayed limbs and slung rifle. |
| 25 | `FallBack` | One-Shot Transition | Implemented | Soldiers, Enemies | Five-keyframe stagger and collapse onto back, landing seamlessly in `Wounded` (`21`). |
| 26 | `FallForward` | One-Shot Transition | Implemented | Soldiers, Enemies | Five-keyframe lethal crumple forward onto face, landing seamlessly in `Dead` (`24`). |
| 27 | `DeadBack` | Full / Casualty | Implemented | Soldiers, Enemies | Supine death pose for wounded soldiers who bleed out or take lethal fire while down. |
| 28 | `Slung` | Full / Base | Implemented | All | Non-combat posture with rifle slung diagonally across back and arms relaxed at sides. |
| 30 | `Ready` | Full / Base | Implemented | All | Core combat-ready athletic stance (`Carry::LowReadyFwd`), staggered feet, flexed knees. |
| 31 | `Brake` | Transition | Implemented | All | Hard-stop skid pose with lead foot braced wide and torso leaning back against momentum. |
| 32 | `Crawl` | Locomotion | Implemented | All | Four-direction low crawl (`0`, `90`, `180`, `-90` deg) with elbow surge, knee draw, and Shift fast-crawl. |
| 33 | `Drop` | One-Shot Transition | Implemented | All | Three-stage get-down/get-up transition (`Ready -> DropReach -> DropPlant -> Prone`). |
| 34 | `DropReach` | Transition Key | Implemented | All | Intermediate keyframe of `Drop`: dropping to knee with left hand reaching toward ground. |
| 35 | `DropPlant` | Transition Key | Implemented | All | Intermediate keyframe of `Drop`: both knees down with left palm planted on ground. |
| 36 | `ProneAim` | Full / Upper | Implemented | All | Prone firing pose with rifle shouldered; layerable over `Crawl` legs. |

### 2.2 Procedural and Additive Systems (`anim.rs` and `pose.rs`)

- **Turn-in-Place Foot Shuffle:** Weighted angular acceleration (`28 rad/s^2`) with automatic foot shuffle when spinning faster than `SHUFFLE_SPIN` (`1.3 rad/s`).
- **Momentum Banking and Pitch:** Lateral roll into high-speed turns and fore-aft pitch during acceleration/braking (`motion.rs`).
- **Step-Out Initiation:** Smooth step-out blend when transitioning from stationary stances into `Walk` or `Run`.
- **Weapon Recoil Kick:** Progressive shoulder/muzzle climb (`recoil`) packed into instance fraction (`pack_kick`).
- **Direct Hit and Near-Miss Flinch:** Full flinch on hit (`flinch`) and graded head/shoulder duck when enemy rounds pass within 1.5 m (`pack_flinch`).
- **Support-Hand Grip IK:** Continuous solver (`grip` / `reach_pitch`) pinning the left palm to the rifle handguard across all two-handed poses and blends.

---

## 3. Current Audit: Defects and Polish Required

1. **Prone Visual Polish (`AN-02`):** `Prone` (`5`) and `Crawl` (`32`) now share `ProneAim` (`36`) as an upper layer, so starting or stopping the crawl does not restart the aim blend. The model viewer exposes stationary aim and all four aimed crawl directions. Elbow/ground contact and cadence are still separate visual-tuning items.
2. **Prone-to-Tablet Joint Velocity (`AN-01`):** `anim_audit` flags `Prone -> Tablet` at `10.5 rad/s` (highest joint speed in the rig) because the figure stands up and pulls the rifle onto the back simultaneously; needs an intermediate `Drop` stand-up phase before raising the PDA.
3. **Cover Peek Dynamics (`AN-01`):** `Cover`, `CoverAim`, `LeanLeft`, and `LeanRight` hold static angles once blended; need shader-side weight shifts, subtle head checks, and smooth peek-in/peek-out micro-motion.
4. **Idle Life (`AN-01`):** `Stand`, `Ready`, `Crouch`, and `Stack` need low-cost shader breathing, weight shifting, and occasional weapon/horizon scans so squads in hold positions look alive.
5. **Sprint-to-Prone Dive (`AN-05`):** Pressing prone while sprinting currently plays the deliberate `Drop` sequence instead of a forward tactical dive onto the chest.

---

## 4. Target Animation Roster (To Be Built)

### 4.1 Locomotion and Stance Transitions

| ID | Animation Name | Roles | Priority | Description and Trigger |
|---|---|---|---|---|
| LOC-01 | `SprintDive` | All | High | Forward tactical dive from `Sprint` (`7`) onto chest (`Prone` `5`), hands catching fall (`AN-05`). |
| LOC-02 | `HighCrawl` | Soldiers, Enemies | Medium | Knee-and-elbow fast crawl variant for crossing low cover under fire without full upright run. |
| LOC-03 | `TacticalUp` | All | High | Crisp combat get-up from `Prone` (`5`) directly into `Sprint` (`7`) or `Ready` (`30`) when bounding under fire. |
| LOC-04 | `RollLeft` / `RollRight` | Soldiers, Commander | Low | Prone lateral roll to shift behind narrow cover without standing up. |
| LOC-05 | `SlopeAdapt` | All | Medium | Shader-side hip/ankle pitch compensation on steep planetary terrain grades (`ground.rs` normal). |
| LOC-06 | `StairGait` | All | Medium | Adjusted knee lift and foot placement when ascending/descending ship ramps, ladders, and building stairs. |

### 4.2 Combat, Weapons, and Equipment

| ID | Animation Name | Roles | Priority | Description and Trigger |
|---|---|---|---|---|
| WPN-01 | `WeaponSwap` | All | High | Smooth transition between primary rifle (`Carry`), slung (`28`), and sidearm/secondary (`AN-04`). |
| WPN-02 | `PistolStance` / `PistolAim` / `PistolReload` | Officers, Cmdr, Crew | High | One- and two-handed sidearm hold, sights aim, recoil kick, and slide/mag reload (`AN-04`). |
| WPN-03 | `HeavyWeaponCarry` / `HeavyAim` / `BipodDeploy` | Gunners | High | Squad automatic weapon / LMG hip carry, braced shoulder aim, and prone bipod deployment (`AN-04`). |
| WPN-04 | `LauncherAim` / `LauncherFire` | AT Specialists | Medium | Shoulder-fired anti-armor/anti-ship tube stance with backblast check and reload (`AN-04`). |
| WPN-05 | `DryReload` vs `TacticalReload` | All | Medium | Distinct bolt-release slap on empty magazine versus retained-mag tactical swap. |
| WPN-06 | `MalfunctionClear` | All | Low | Tap-rack-bang clearing drill triggered when low morale or weapon wear causes a stoppage (`AI-4`). |
| WPN-07 | `UnderhandToss` / `LowThrow` | All | Medium | Crouched or prone low grenade roll/toss through doorways and windows without exposing head. |
| WPN-08 | `MeleeStrike` / `Buttstroke` | Soldiers, Enemies | Medium | Close-quarters rifle buttstroke or bayonet thrust when hostiles close inside 1.5 m (`AN-04`). |

### 4.3 Cover, Breaching, and Terrain Traversal

| ID | Animation Name | Roles | Priority | Description and Trigger |
|---|---|---|---|---|
| TRV-01 | `VaultLow` | All | High | One-handed vault over waist-high walls, barricades, and ship bulkheads without losing momentum (`AN-05`). |
| TRV-02 | `ClimbMantle` | All | High | Two-handed pull-up onto chest-high ledges, crates, and cargo platforms (`AN-05`). |
| TRV-03 | `WindowEntry` | Soldiers, Enemies | Medium | Crouched step-through and duck into ground-floor building windows (`Killhouse`). |
| TRV-04 | `DoorBreachKick` / `DoorHandleOpen` | Soldiers, Enemies | High | Breach kick for locked/contested doors vs controlled handle turn for stealth entry. |
| TRV-05 | `BlindFireHigh` / `BlindFireCorner` | Soldiers, Enemies | Medium | Suppressive blind-fire over cover or around corners when suppressed (`Morale::Pressured`). |
| TRV-06 | `LadderClimb` | All | Medium | Vertical rung climb for planetary watchtowers, compound roofs, and multi-deck starships. |

### 4.4 Command, Squad Leadership, and Galactic Operations

| ID | Animation Name | Roles | Priority | Description and Trigger |
|---|---|---|---|---|
| CMD-01 | `SignalPack` | Leaders, Commander | High | Expanded upper-body hand signals: `Halt` (fist), `Advance` (chop), `TakeCover` (palm down), `FormLine` (spread), `HoldFire` (cross). |
| CMD-02 | `PointTarget` | Leaders, Commander | High | Extended arm/weapon point designating enemy contacts, sectors, or breach points to squadmates. |
| CMD-03 | `TabletProne` / `TabletCrouch` | Commander, Leaders | High | Low-profile PDA inspection while kneeling or prone so inspecting the tactical map does not force standing up (`AN-01`). |
| CMD-04 | `BinocularScan` | Leaders, Recon, Cmdr | Medium | Raising optics/rangefinder to eyes in standing, crouch, or prone for long-range reconnaissance. |
| CMD-05 | `JukeboxInteract` / `ConsoleOperate` | Commander, Crew | Low | Reaching out to operate field terminals, airlock controls, bridge stations, or camp jukebox. |
| CMD-06 | `Salute` / `AtEase` | Soldiers, Officers | Low | Non-combat garrison, staging bay, and briefing postures on capital ships and forward bases. |

### 4.5 Vehicles, Dropships, and Starship Boarding

| ID | Animation Name | Roles | Priority | Description and Trigger |
|---|---|---|---|---|
| VEH-01 | `SeatSit` / `SeatBraced` | All | High | Seated troop posture on dropship benches, APCs, and shuttles with rifle held vertical between knees and reentry/turbulence brace. |
| VEH-02 | `PilotControls` / `DriverSteer` | Pilots, Crew | High | Seated hands-on-yoke/stick and throttle posture for dropship pilots, vehicle drivers, and starship helm stations. |
| VEH-03 | `TurretGunner` | Gunners | Medium | Standing or seated pintle/remote turret grip with upper body tracking turret traverse and elevation. |
| VEH-04 | `MountStepUp` / `DismountJump` | All | High | Stepping up into vehicle hatches/ramps and jumping out onto ground when ramp drops (`boarding`). |
| VEH-05 | `ZeroGFloat` / `MagBootWalk` | Boarding Crew | Medium | Heavy magnetic-boot step and low-gravity drift posture for orbital ship-to-ship boarding actions. |
| VEH-06 | `AirlockCycle` | Boarding Crew | Low | Bracing against bulkhead handholds during decompression and boarding tube docking. |

### 4.6 Casualties, Medical, and Morale States

| ID | Animation Name | Roles | Priority | Description and Trigger |
|---|---|---|---|---|
| CAS-01 | `HitReactionDirectional` | Soldiers, Enemies | High | Distinct upper-body/leg stagger based on hit zone (left shoulder spin, leg buckle, chest punch) (`AN-03`). |
| CAS-02 | `WoundedLimp` | Soldiers, Enemies | High | Asymmetric limping walk/hobble for soldiers with leg injuries who can still move (`AN-03`). |
| CAS-03 | `WoundedCrawl` | Soldiers, Enemies | Medium | One-arm/weakened crawl dragging an injured leg toward nearby cover (`AN-03`). |
| CAS-04 | `FiremanCarry` | Soldiers | Medium | Lifting and carrying an incapacitated comrade over the shoulders at faster speed than `Drag` (`AN-03`). |
| CAS-05 | `BuddyAssistUp` | Soldiers, Medics | Medium | Pulling a stabilized or downed teammate back to their feet (`AN-03`). |
| CAS-06 | `DeathVariations` | Soldiers, Enemies | Medium | Additional terminal crumples (knees-first collapse, backward spin, slumped against cover wall) (`AN-03`). |
| MOR-01 | `SuppressedHunch` | Soldiers, Enemies | High | Continuous low-head, hunched-shoulder posture under heavy incoming fire (`combat-readiness.md`). |
| MOR-02 | `FracturedPanic` / `Surrender` | Enemies, Soldiers | Low | Breaking/routing sprint with weapon held low or hands-up surrender when squad cohesion collapses. |

### 4.7 Scale and Performance LOD (`AN-06`)

| ID | System Name | Scope | Priority | Description and Trigger |
|---|---|---|---|---|
| LOD-01 | `DistancePoseLOD` | All | High | Disable foot shuffle, support-hand IK, and fine finger/head micro-sway beyond mid-range camera distance (`AN-06`). |
| LOD-02 | `BattalionProxy` | Distant Squads | Medium | Simplified two-state stride/aim interpolation for 1,000+ soldier galactic/planetary engagements (`battle_bench`). |

---

## 5. Implementation Order

1. **Phase 1 (Immediate Polish):** Finish `AN-02` (`ProneAim` alignment, `Crawl` browser polish) and `AN-01` (shader idle life, cover peeks, `Prone -> Tablet` intermediate stand-up).
2. **Phase 2 (Traversal and Boarding):** Implement `VEH-01` (`SeatSit` / `SeatBraced`), `VEH-04` (ramp mount/dismount), `LOC-01` (`SprintDive`), and `TRV-01`/`TRV-02` (vault and mantle) for planetary landings and compound assaults.
3. **Phase 3 (Casualties and Morale):** Implement `CAS-01` through `CAS-05` and `MOR-01` wired directly to `scalia-ai` wound and readiness states.
4. **Phase 4 (Weapons and Command Expansion):** Implement `WPN-01` through `WPN-04` (sidearms, LMGs, launchers) and `CMD-01` through `CMD-03` (expanded signals and low-stance PDA).
5. **Phase 5 (Galactic Scale LOD):** Implement `LOD-01` and `LOD-02` validated against `battle_bench` at 1,000+ active combatants.

## Design References

- **Building the AI of F.E.A.R. with Goal Oriented Action Planning | AI 101** (`https://www.youtube.com/watch?v=PaOLBOuyswI`) — Reference for decoupling tactical AI planning (preconditions, effects, and dynamic replanning when world state changes) from a compact execution layer (`Goto`, `Animate`, and `UseSmartObject` smart-object interactions).
- **Three States and a Plan: The A.I. of F.E.A.R.** — Jeff Orkin, GDC 2006 (`https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf`): Canonical paper on pairing a three-state execution FSM (`Goto`, `Animate`, `UseSmartObject`) with cost-weighted A* GOAP, shared working memory across postures/smart objects, procedural preconditions/effects, and emergent squad behaviors.
