# Blueprints and procedural planet: design for approval

Status: draft, written while the user was away. Nothing here is built. Approve, change or reject each point.

## Why

Milestone 0 gives deterministic groups and boarding. The user's next goal is units, then a test flight between two planets. Two things are missing before any flight: vehicles defined as data, and a ground to put units on.

## Intent (what the user said)

- Every vehicle gets a blueprint; the blueprint is the single source of truth for the 3D model, the drawing and the gameplay numbers.
- Original designs only. The Elite Dangerous gallery was inspiration, not a source. Style: industrial and utilitarian, a blend of Elite-like and Nostromo-like ships.
- Art: PS1-style low-poly, low-resolution textures, no vertex wobble.
- First scene: a plane with a procedural planet and test units (small, medium and large troops, a ship, tanks, vehicles).

## Part 1: blueprint format (new crate `scalia-blueprint`)

- One JSON file per vehicle. JSON because the browser and the Rust core both read it with no extra tooling.
- Fields: `id`, `name`, `class` (`walker`, `tank`, `truck`, `transport`, `ship`), `hull` (length, width, height in mm, mass in kg), `stats` (speed in mm/s, crew, passenger capacity, armour), `interior` (a `ShipLayout` plus a list of rooms with a floor rectangle and a purpose), `model` (a list of primitives: box, wedge, cylinder, each with position, size and a palette index).
- Outputs, all pure functions of the file: a validated `Blueprint` struct; the `ShipLayout` that boarding already consumes; a mesh (vertices and indices, with a triangle budget); a top-down SVG drawing.
- Validation: dimensions positive; interior rooms inside the hull; seat count at least the stated capacity; triangle count under the per-class budget (proposal: 400 for a walker, 1,200 for a tank or truck, 4,000 for a ship).
- Six original test blueprints: three troop kinds (small, medium, large), one tank, one truck, one transport ship.
- Tests: each shipped blueprint loads and validates; golden mesh hash per blueprint; the drawing contains every room; a bad file reports which field is wrong.

## Part 2: procedural planet, CPU side (in `scalia-core`)

- A height function of (seed, position) built from integer value noise in fixed point, so it is deterministic and identical native and in WebAssembly.
- Projection: proposal is a cube-sphere (six faces) so a whole planet exists, but the first scene only samples one face as a flat patch.
- Outputs: height at a point; a chunk of a regular grid for the renderer; spawn points on flat ground for the test units.
- Tests: same seed same chunk hash; neighbouring chunks agree along their shared edge; heights inside the stated range; golden hash; budget of 1 ms per 64 by 64 chunk.

## Decisions I need from you

1. JSON for blueprints. Recommended. Alternative: RON (nicer to edit by hand, Rust only).
2. Triangle budgets above, or other numbers.
3. Cube-sphere planet, one face used for the first scene. Alternative: a flat patch only, and add the sphere later.
4. Are blueprints also findable in-game items (loot), and can players design variants? This is open in the game design document. It does not block the format, but it decides whether files need an owner and a version.

## Not in scope

Rendering, textures, combat, the ship flight model, and the planet-to-planet flight itself.
