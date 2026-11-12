# Testing grounds and navigation — design

Status: draft for review · 2026-11-12

## What the user asked for

- A testing ground **where the game starts** (during development). It covers:
  - soldiers navigating around obstacles;
  - soldiers navigating inside buildings;
  - vehicles navigating around obstacles.
- **Every soldier navigates individually**: each one finds his own way, everywhere. There are no formation blobs sliding around rocks.
- **Multi-storey buildings** in the first pass: rooms, doors, stairs, two floors, and cover at windows.
- Obstacles are **static for now**. Navigation must be able to update when something breaks later.
- Standing rules still hold:
  - 60 fps on the RTX 3050 Ti laptop; 30 fps must still play correctly.
  - The simulation is deterministic, because one shared server runs it.
  - Measure, then optimise.

## The one big consequence

Today **groups are authoritative** and soldiers are *derived*: their positions are computed from the group position plus a formation offset. Individual navigation means soldiers need their **own simulated positions**. So this design changes:

- A group stays the **unit of orders** ("squad 12, go there"). Each soldier becomes a **simulated agent** with a position, a velocity and a route.
- **Cost honesty:** 500,000 agents can't all do full steering 20 times a second inside a browser frame. Every soldier still moves on his own path, but **how often he is updated depends on where he is**:

| Agent tier | Where | Update rate | What runs |
|---|---|---|---|
| Hot | on screen, or within 50 m of an enemy | 20 Hz | flow field, steering against neighbours, door and stair logic |
| Warm | loaded, off screen | 5 Hz | flow field, cheap separation |
| Cold | far away, no contact | 1 Hz | follows its path, no neighbour checks |

  Positions are interpolated for drawing, so every tier still renders smoothly at 60 fps. The rate is deterministic: it comes from the tier rules, never from the frame time.
- Budget target: **navigation ≤ 4 ms per frame** with 20k hot plus 480k cold agents on your laptop. The benchmark must prove this before anything else is built on it.

## World objects

- `Prop { kind, pos_mm, rot: quarter turns, floor }` is stored in the core. Props load from RON, like blueprints.
- Kinds in the first pass: rock (convex outline), wall segment, fence, crate, trench (lowered ground), bridge (walkable deck over a gap), building.
- A building is a RON floor plan. It has a grid of rooms per floor, with walls, doors, windows (cover points) and a stair cell linking floor 0 to floor 1.
- Props are drawn as low-poly meshes in the same style as the soldiers. Roofs and upper floors fade out when the camera is above them, so you can see inside.

## Navigation

1. **Nav grid**: 0.5 m cells in 32 m tiles (64 × 64 cells).
   - Each cell holds walkable flags, a move cost and a **clearance** value: the distance to the nearest obstacle.
   - Clearance lets one grid serve every size of mover: a soldier needs ≥ 0.3 m, a truck ≥ 1.5 m, a tank ≥ 2 m.
   - Buildings add one grid layer per floor. The layers are joined by portals at stairs and doors.
2. **Long routes** use a tile graph (hierarchical A*). Each group plans one route across tiles.
3. **Flow fields**: within the tiles on that route, one shared direction map per destination. Every soldier of the group, and every other group sent to the same place, reads it. That makes thousands of individual movers cheap.
4. **Local steering** (hot tier): each soldier keeps his own spacing, avoids neighbours found through a spatial hash, and queues through doors instead of piling up. All of it uses integer maths and runs in a fixed order, so it is deterministic.
5. **Stuck detection**: if an agent moves less than 0.5 m in 5 s, it re-plans. After a second failure it reports "stuck", and the testing ground counts it.
6. **Vehicles** use the same grid at their own clearance.
   - Path following steers the front axle along the path with a turning-radius limit.
   - At dead ends they reverse and do a 3-point turn.
   - Soldiers give way to vehicles.
7. **Ready for destruction later**: changing a prop rebuilds only that 32 m tile and invalidates the flow fields that cross it.

## The testing grounds

A flat 400 × 400 m area at the start position, with labelled zones. Each zone has a button or key that sends its test squad or vehicle, plus a pass or fail readout.

| Zone | Contents | Pass when |
|---|---|---|
| A · Rock field | dense rocks, one 1 m gap | 100 soldiers cross with no overlap with any rock, all within 60 s |
| B · Maze | walled maze with a dead end | the squad exits; nobody is stuck |
| C · Trenches & bridge | 2 m trench and a narrow bridge | the squad uses the bridge in single file |
| D · Building | 2 floors, 6 rooms, 3 doors, stairs, windows | the squad clears the ground floor, goes upstairs, and takes cover at the windows |
| E · Vehicle course | slalom, 3.5 m gate, dead-end bay, U-turn | the truck and the tank finish without touching anything; the tank reverses out of the bay |
| F · Crowd swap | two 200-man companies swap sides through a 6 m gap | everyone arrives with no deadlock |
| G · Mass | 20k soldiers across the whole ground | navigation stays within the 4 ms budget |

- The same zones also exist as **automated headless tests** in the core. CI fails if any soldier ends up inside an obstacle, if a test doesn't finish in time, or if the state hash changes between two runs.
- Debug overlay (key **G**): nav grid, clearance, flow arrows, paths, and stuck agents in red.

## Build order

Each step has its own tests, and nothing moves on until its benchmark passes.

1. Props and nav grid with clearance, built from RON (core). Benchmark: tile build ≤ 1 ms.
2. Agent storage and tiers. Soldiers spawn as agents from their groups. The old derived formations stay available for the far zoom only.
3. Flow fields and hierarchical A*. Benchmark: 500k agents.
4. Steering, doors and stuck handling.
5. Buildings with floors and stairs, plus the see-inside camera.
6. Vehicles.
7. Testing-grounds map and HUD, debug overlay, headless zone tests.
8. Verification in your Chrome, then you review it.

## Out of scope for this pass

- Destruction (the design leaves room for it).
- Combat and shooting. Cover points exist, but firing comes with GOAP.
- Ladders and climbing.
- Boarding seats matching ship interiors.
