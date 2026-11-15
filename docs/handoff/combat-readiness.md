# Combat readiness (design, project lead, 15 Nov)

Scalia's stress system, called **Combat readiness**: a per-soldier meter (100% fresh .. 0%) that drains while a
unit is kept active in combat conditions without being stood down, and recovers when its leader puts it **at ease**.

## States
| State | Range | Effects |
|---|---|---|
| **Good** | 100 to 70% | Calm, no penalties. |
| **Pressured** | 70 to 50% | Aim accuracy reduced; reloads and item interactions slightly slower or fumbled. Tactical sense (taking good cover, peeking at the right moments) unaffected. |
| **Fractured** | 50 to 25% | Aim impaired and judgement too: takes cover at the wrong times, worse decisions. Fumbles equipment often; doors may take several tries; kicking doors and driving are poor and reckless. |
| **Severe** | 25 to 0% | Aim drops to rookie level. Very poor cover decisions (casualties follow). Fumbling, dropped magazines, rarely a grenade dropped while drawing it. Items have a medium to high chance of slipping while doing an action. |

## Rates
- Reaching Severe takes about **3 in-game days of continuous deployment without being stood down**, which at the
  game's clock is about **3 real hours** of continuous combat-condition deployment.
- Drain should scale with what the men go through (time in contact, incoming fire, casualties nearby, nights without
  rest), on top of the base rate.

## At ease (recovery)
- The leader stands his unit down ("at ease"), even in the middle of a sandbag position.
- The men sit down, smoke if they have cigarettes, eat, drink, socialise, clean their weapons.
- Recovery rate depends on what is available (food, water, cigarettes, safety, sleep).

## Ties into
- Composure and veterancy (veterans drain more slowly and hold better in the lower states).
- The base-life work order (idle life, rest and recovery) and the animation work order (sitting, smoking, eating,
  cleaning weapons; fumbles, dropped magazines, dropped grenade).
- The AI: accuracy (`hit_permille`), decision quality (cover choice, peeking), action times and failure chances
  (reload, doors, throws).

## Reference
The project lead is referencing **BODYCAM** (animations, systems, audio) for ideology and premise; see the
Animation & Loadout devlog (https://youtu.be/6YUWQXv0MTk).
