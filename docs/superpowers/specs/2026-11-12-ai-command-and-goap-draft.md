# AI: command hierarchy and GOAP (draft for decision)

Status: **decided 2026-11-12** (see Decisions below). The options are kept for the record.

## What the project lead asked for

- Soldiers should seem independent: run to cover, take cover, shoot over it, and so on.
- A general, with ranks under him governing their structures.
- Goal-oriented action planning (GOAP) like the AI in F.E.A.R.

## What already exists

- **Groups are the truth; individuals are derived.** The core stores groups (position, composition, needs, one
  process: march, board, flight). Soldiers near the camera are computed on demand from the group, a seed and the
  time; they are never stored unless flagged persistent in the roster. This is what lets one shared server hold
  millions of soldiers.
- **Poses** for the look: stand, walk, run, crouch, cover aim, prone, standing aim (render only).
- **A deterministic GOAP planner** (`crates/scalia-ai`): facts as a 64-bit set, actions with preconditions, effects
  and integer costs, A* with node and depth budgets, ties resolved identically on every machine. Tested (7 tests).
  Measured on the sandbox (2 cores): a squad-sized plan (14 actions, 12 facts) takes about 1 microsecond typically and
  about 50 microseconds at worst.

## The core tension

F.E.A.R. runs a planner for each of a handful of soldiers. Scalia has millions. Planning for every soldier every
second would cost about one full CPU core per million soldiers on the server, before any pathfinding or line of
sight, and would force every soldier to be stored rather than derived. So the question is where planning happens.

## Options

**A. Plans at the command levels; soldiers react (recommended).**
- General -> officers (company) -> squad leaders each run GOAP over their own small action set
  (general: allocate forces, set objectives; officer: advance, hold, flank, withdraw; squad: bound with overwatch,
  suppress, flank, assault, take cover, regroup, call support).
- Soldiers carry out the squad's current action with cheap deterministic rules: pick the nearest free cover point on
  the squad's side, run to it, crouch, rise to fire when the squad suppresses, go prone under heavy fire.
- These soldier behaviours are derived like positions are now (seed + time + squad action), so they stay cheap and
  identical on every machine, and still look individual on screen.
- Cost: 100,000 squads replanning once a second is about 0.1 s of one core per second, by the measurement above.

**B. Every soldier plans (closest to F.E.A.R.).**
- Each soldier in contact runs its own GOAP with personal facts (ammo, wounded, in cover, has line of sight).
- Most faithful, but only affordable for a few thousand soldiers in contact at once, and those soldiers must be stored
  individually while fighting.

**C. A plus B.**
- Command levels and squads plan everywhere (A). Soldiers in direct contact, or near any player's camera, are promoted
  to stored individuals with their own planner (B) for the duration of the fight, then fold back into their squad.
- Richest result, most work: promotion and demotion must be deterministic and must not teleport anyone.

## Pieces every option needs

- **Cover points:** derived from the terrain (ridges, hollows, slopes facing away from the enemy) and from objects
  (vehicles, walls, buildings), deterministic per chunk.
- **Line of sight and threat:** at least squad-to-squad visibility and a threat direction.
- **Squads inside groups:** today a group is one formation; ranks need groups split into squads with leaders.
- **Combat resolution:** shooting, suppression, wounds; deterministic and aggregate-first.
- **Pose blending:** smooth transitions between poses (planned next, render only).

## Open questions for the project lead

1. Option A, B or C?
2. Is the player the general (giving orders down the chain), or do AI generals command whole armies too, or both?
3. How much can players override: order any squad directly, or only their own level of command?
4. Squad size and the ranks you want (for example: squad 8-12 with a sergeant, platoon of 3-4 squads with a
   lieutenant, company of 3-4 platoons with a captain, then a general).

## Decisions (2026-11-12)

Combat should feel like Company of Heroes 1: soldiers take cover by themselves, each in his own spot.

1. **Planning at the command levels, soldiers react individually.** This is option A, made possible because soldiers are now individual agents (see the navigation design). Squad, platoon, company and general each plan with GOAP. Each soldier claims a cover slot facing the threat, peeks and fires, and goes prone when pinned.
2. **The player is the general.** Players set objectives and give orders to the levels below the general. AI officers and sergeants carry them out. Players do not click individual squads.
3. **Realistic sizes.** A squad has 10–12 soldiers under a sergeant. A platoon has 3–4 squads under a lieutenant, a company has 3–4 platoons under a captain, and the general is above them.
4. **Cover shown as shields.** Hovering over ground shows green, yellow or red shields for heavy, light or no cover. For a general this appears when placing objectives, defensive lines and attack positions, and those officers then use that cover.
5. **First combat pass:** suppressed and pinned states, garrisoning buildings through window slots, and directional cover so that flanking works. Retreat comes later. Officers may still pull back by their own plan.
