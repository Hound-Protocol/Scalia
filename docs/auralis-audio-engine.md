# Auralis Audio Engine

**Official subsystem name:** Auralis Audio Engine  
**Repository crate:** `scalia-audio`  
**Browser implementation:** `web/audio.js`  
**Owner:** Scalia / Pack Engine

Auralis is Scalia's dedicated audio engine. It is a first-class subsystem of Pack Engine, not a separate game engine and not merely a collection of sound effects.

## Responsibility

Auralis owns the complete path from a sound event to the listener's ears:

1. Accept sound events and their world-space source information.
2. Determine how the world changes the sound: distance, occlusion, air absorption, geometry, environment, reflections and propagation delay.
3. Render the result through a shared audio graph: spatialisation, filtering, reverb, dynamics, mixing and output.
4. Expose consistent controls for buses, voice priority, loudness, accessibility and audio settings.
5. Provide diagnostics and tests so acoustic behaviour can be reproduced and tuned.

## Current architecture

| Layer | Location | Responsibility |
|---|---|---|
| Acoustic world model | `crates/scalia-audio` | Rust calculations for listener environments, sound paths, walls, echoes and air absorption |
| Web Audio renderer | `web/audio.js` | AudioContext lifecycle, voice routing, spatialisation, filters, reverb, HDR/dynamics and mix |
| Game integration | `web/game.js`, `web/jukebox.js` | Creates sound events, manages world ambience and jukebox interaction |
| Content | `web/sfx/` | Music, ambience and sound assets |

The Rust crate computes acoustic properties; it does not play audio. The browser layer renders those properties. Both are parts of Auralis and should be treated as one engine.

## Architectural rules

- One shared AudioContext and one authoritative master graph per game session.
- Every audible source should route through Auralis, unless a documented technical exception exists.
- A media element must never be connected to more than one MediaElementAudioSourceNode.
- Game systems submit sound events; they should not construct independent competing audio graphs.
- Acoustic calculations remain separate from deterministic gameplay state.
- Audio processing must degrade gracefully when browser APIs, devices or assets are unavailable.
- Voice allocation, CPU cost and memory use must be bounded and measured.
- User-facing controls must affect the actual signal path, not just their displayed values.
- Changes require automated regression tests and real-browser listening checks where appropriate.

## Known integration debt

`web/game.js` currently creates a separate `worldAudio` AudioContext for world ambience, while `web/audio.js` owns the main Auralis graph. This splits ownership and risks duplicated routing, inconsistent mixing and separate lifecycle handling.

**Target:** migrate world ambience and all other world sounds into Auralis, then remove the parallel graph once parity is verified. Do not simply silence routing errors or remove existing ambience.

The tactical sight grid and Auralis now share the core prop-geometry raster and height-band rules. Auralis receives the shared grid plus battle door state; material-specific transmission and material preservation through battle map saves remain future work.

## Development roadmap

1. **Ownership and routing:** unify every sound source under one graph and lifecycle.
2. **Voice system:** explicit voice handles, priorities, categories, pooling, stealing and cleanup. Keep bounded capacity for Foley so dense combat cannot consume every live voice.
3. **Acoustic correctness:** route sounds through open doorways and over walls only when the elevated path is clear and roofless; define roof footprints in world-map data and validate distance, occlusion, reflections, air absorption and listener movement.
4. **Mixing:** master, music, ambience, weapons, Foley, UI and radio buses; ducking and snapshots.
5. **Spatial audio:** stable camera-relative 2D stereo panning; the camera is the listener, while distance, occlusion and zoom are handled by the acoustic model.
6. **Performance:** profile dense firefights, cap expensive voices/effects, avoid per-frame graph churn.
7. **Tooling:** diagnostics for active voices, routing, peak levels, clipping, context state and acoustic path values.
8. **Quality:** browser/device compatibility, accessibility, settings persistence and listening-based tuning.

## Definition of done

Auralis is considered production-ready only when:

- all game audio uses the shared engine;
- no duplicate media-element routing errors occur;
- controls and buses affect audible output reliably;
- occlusion and spatialisation behave consistently as the listener and world move;
- dense scenes remain within measured CPU and voice budgets;
- browser regression tests and real-browser listening checks pass;
- engine responsibilities and public interfaces are documented.
