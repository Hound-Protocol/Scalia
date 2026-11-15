# Work orders: AI and animation (15 Nov)

Two tracks that can run in parallel (one session each). Every item: a test, visual proof for animation
(stance sheet or in-game capture), a reproducible lab scenario for AI, `scripts/check.sh` before each push,
`hash_battle` unchanged unless the change is meant to alter the battle. Cheap at scale: no per-soldier CPU cost that
grows with the battle; prefer shader-side and table work; LOD.

## Track AI

### AI-1. Re-apply batch 1 (done, 15 Nov, 88a9f7f)
- `git revert 36281b5` (brings back fire lanes, kneel when the enemy is known, spacing, staggered reactions).
- It was rolled back during the freeze hunt; the freeze was a stuck browser, not this.
- Re-run `lab_behaviour_report` and compare with the numbers in the AI audit doc.

### AI-2. Batch 2 (from the audit) (side-step and shake-out done, 15 Nov, 52f96a7)
- Blocked shooters side-step a pace to clear the lane instead of only waiting.
- Formation on contact: a squad in column shakes out into a line so more men can fire forward.
- Next ranked items from the audit doc after these.

### AI-3. Reference squad (the project lead's core ask)
- Lab scenario: 12 soldiers and a sergeant, one order, an enemy formation.
- Must show, each with a test:
  1. taking the order
  2. moving to the objective
  3. detecting the enemy
  4. own cover per man
  5. coordinated fire
  6. reacting to fire
  7. suppression
  8. casualties handled
  9. regrouping
  10. completing or abandoning the objective by circumstance

### AI-4. Deeper systems
- Tactical memory: last-known enemy positions; limited and contextual.
- Morale and psychology: fear, fatigue, cohesion and experience (ties into Composure and veterancy).
- Knowledge only by sight or the radio; reports travel up the chain of command, with delay.
- Interruptible plans and recovery when an action fails.
- Then platoon (lieutenant) and company (captain) levels on the same principles.

## Track Animation

### AN-1. Life in the still poses
- Cover, cover aim, prone, prone aim, stack, peeks: breathing and sway, weight shifts, scanning; peeks lean out and back.
- Shader-side (no per-soldier CPU).

### AN-2. Prone combat
- Crawl backwards and sideways (forward exists).
- Prone aim and fire; moving between crawling and aiming.

### AN-3. Casualties
- Hit reactions while standing, death variations, limping, wounded crawl, carrying a wounded man, helping one up.

### AN-4. Weapons (reference: BODYCAM; our figure's proportions win)
- Pistol fire and reload, weapon switching, handling per weapon class (rifle, pistol, MG, launcher).

### AN-5. Terrain moves
- Dive to cover, vault low walls, step over obstacles, kneel to observe.

### AN-6. Animation LOD
- Distant figures: no shuffle, no reach IK, coarser updates; measured with `battle_bench` and the renderer bench.

### AN-7. Small fixes
- Intro: the zoom-out after the eyes open ends at the closest zoom level, not the widest (work order v2, H).

## Lab (15 Nov)
- Clear house runs in a training compound on the range: a new house every run (`ground.rs` `compound`, `Killhouse::generate_within`), random defenders and cover. `compound_report` (ignored test) measures house clearing over many layouts: 30 runs gave 28/30 taken, 8.0 of 12 standing, 93 s.

## Order
- AI: 1, 2, 3, then 4.
- Animation: 1, 2, 7, then 3, 4, 5, 6.
- Hook-ups between tracks: AI decisions must drive the new animations (incoming fire → drop or dive; casualties →
  drag or carry). Whichever track lands second wires them.
