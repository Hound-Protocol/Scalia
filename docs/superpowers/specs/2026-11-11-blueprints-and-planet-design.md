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

- One RON file per vehicle (decided). RON is pleasant to edit by hand and maps straight onto Rust structs. The browser cannot read RON itself, so `scalia-blueprint` is compiled into the wasm module and exposes `load_blueprint(text)`; the `ron` crate is pure Rust and builds for wasm32. All numbers in files are integers (mm, kg, mm/s) so loading stays deterministic.
- Fields: `id`, `version`, `name`, `class` (`walker`, `tank`, `truck`, `transport`, `ship`), `hull` (length, width, height in mm, mass in kg), `stats` (speed in mm/s, crew, passenger capacity, armour), `interior` (a `ShipLayout` plus a list of rooms with a floor rectangle and a purpose), `model` (a list of primitives: box, wedge, cylinder, each with position, size and a palette index).
- Outputs, all pure functions of the file: a validated `Blueprint` struct; the `ShipLayout` that boarding already consumes; a mesh (vertices and indices, with a triangle budget); a top-down SVG drawing.
- Validation: dimensions positive; interior rooms inside the hull; seat count at least the stated capacity; triangle count under the per-class budget (decided: 400 for a walker, 1,200 for a tank or truck, 4,000 for a ship).
- Six original test blueprints: three troop kinds (small, medium, large), one tank, one truck, one transport ship.
- Tests: each shipped blueprint loads and validates; golden mesh hash per blueprint; the drawing contains every room; a bad file reports which field is wrong.

## Part 2: procedural planet, CPU side (in `scalia-core`)

- A height function of (seed, position) built from integer value noise in fixed point, so it is deterministic and identical native and in WebAssembly.
- Projection (decided): cube-sphere (six faces) so a whole planet exists; the first scene only samples one face as a flat patch.
- Outputs: height at a point; a chunk of a regular grid for the renderer; spawn points on flat ground for the test units.
- Tests: same seed same chunk hash; neighbouring chunks agree along their shared edge; heights inside the stated range; golden hash; budget of 1 ms per 64 by 64 chunk.

## Decisions (made by the user on 2026-11-11)

1. Blueprint format: RON.
2. Triangle budgets: 400 walker, 1,200 tank or truck, 4,000 ship.
3. Planet: cube-sphere, one face used for the first scene.
4. Blueprints are findable items only; players do not design variants. Files carry `id` and `version` but no owner. Player-designed variants stay out of scope.

## Not in scope

Rendering, textures, combat, the ship flight model, and the planet-to-planet flight itself.
