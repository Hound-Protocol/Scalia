# Scalia (working title)

A co-op tactical command game in the browser: you are the commander on foot, giving intent to squads whose leaders plan and fight on their own, in one shared universe that zooms from the galaxy down to a single soldier.

Built on **Pack Engine**, Hound Protocol's own engine in Rust: a deterministic integer simulation, a wgpu renderer, battle AI, and focused subsystems compiled to WebAssembly. Its dedicated audio subsystem is **Auralis Audio Engine**.

Live demo: https://hound-protocol.github.io/Scalia/

## Engine layout

| Path | What it is |
|---|---|
| `crates/scalia-core` | Deterministic simulation: world, groups, individuals, boarding, planets, flight and state hashes |
| `crates/scalia-ai` | Battle simulation: perception, cover, movement, fire, squad leaders, wounds and casualty care |
| `crates/scalia-render` | Native Rust/wgpu renderer, camera, meshes, procedural poses, lighting and shaders |
| `crates/scalia-audio` | **Auralis Audio Engine's acoustic world model**: occlusion, distance, air absorption, listener spaces, reflections and sound paths |
| `crates/scalia-blueprint` | Vehicle and object blueprints: RON data, validation, mesh generation and boarding layouts |
| `crates/scalia-world` | World object catalogues and map definitions |
| `crates/scalia-wasm` | WebAssembly bridge, viewer, animation and effects |
| `crates/scalia-bench` | Simulation and performance benchmarks |
| `web/` | Browser shell, interface, Web Audio implementation, assets and deployment |
| `docs/` | Design decisions, handoffs and implementation plans |

## Auralis Audio Engine

Auralis is Scalia's dedicated audio engine, not a standalone game engine. It owns the audio system's design and integration across Rust acoustics and the browser's Web Audio graph.

- Rust computes how sound travels through the world: distance, walls, air absorption, listener environment, reflections and propagation delay.
- The browser audio layer renders those results: voice routing, filters, spatialisation, reverb, dynamics, mixing and user controls.
- All gameplay and interface sounds should use the shared engine rather than creating independent audio graphs.

The current implementation is in active development. See [Auralis Audio Engine](docs/auralis-audio-engine.md) for its architecture, boundaries and work plan.

## Check everything

```
bash scripts/verify.sh
```

Needs Rust stable with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.130 and Node 22. See `STATUS.md` and the work-order documents for current verification and open tasks.
