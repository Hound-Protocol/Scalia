# Auralis Audio Work Orders
Date: 2026-11-15
Status: Audit baseline; runtime changes not yet made
Owner: Scalia Audio Freelancer #2

## Scope
Auralis only: Rust acoustic model, browser audio renderer, audio content routing, lifecycle, mixing, performance, diagnostics and tests.

## Audit baseline

### Current components
- `crates/scalia-audio/src/lib.rs`: Rust acoustic model. Uses `scalia_ai::sight::SightGrid`; calculates listener environment, wall crossings, distance/air filtering, reflections, propagation delay and a Dijkstra-based occlusion field.
- `web/audio.js`: Web Audio renderer. Creates an AudioContext lazily; supports decoded-buffer voices, a 64-voice cap, priority/stealing, HDR loudness, HRTF panning, reverb/air tails, echo taps, gunfire dynamics and a media-element stream API.
- `web/game.js`: integration and sound event producers.
- `web/jukebox.js`: persistent media playback integration.
- `web/sfx/`: sound and music content.

### Confirmed architectural debt
There are at least three independently created browser audio contexts/graphs:
1. Auralis in `web/audio.js`.
2. `worldAudio` in `web/game.js`, used by music and wind ambience; owns a low-pass, wake filter and gain for tablet muffling/fades.
3. `uiCtx` in `web/game.js`, used by tablet/interface one-shots; connects decoded buffers directly to its destination.

Additional standalone HTMLAudioElement playback exists for effects such as the tablet dive whoosh and update alert. These must be inventoried before routing changes.

**Do not delete or bypass these paths until equivalent Auralis buses and behaviour exist.** In particular, `worldAudio.add(el)` creates a MediaElementAudioSourceNode in its own context. A later attempt to route that same element through Auralis will fail because a media element can only have one such source node.

### Existing strengths
- Rust acoustics has meaningful unit coverage for open fields, rooms, corridors, wall occlusion, elevation, echoes, doors and thick walls.
- Browser renderer already bounds active decoded-buffer voices at 64 and prioritises blast/shot/step.
- Auralis has a module-wide WeakMap to reuse media-element routing.
- Audio context creation is lazy to respect browser user-gesture restrictions.
- Existing regression work addressed duplicate MediaElementAudioSourceNode ownership and jukebox routing.

### Risks to validate
- AudioContext proliferation and inconsistent mute/volume/lifecycle behaviour.
- Stream routing can return null when an element was already claimed by another graph.
- Voice stealing stops a source, but graph-node cleanup and scheduled echo/reverb node lifetime need profiling.
- Each played voice may allocate multiple Web Audio nodes; dense firefights may create substantial churn.
- Rust `Field::build` allocates a grid and Dijkstra buffers; rebuild cadence and radius need profiling under listener movement.
- The browser's acoustic field integration, listener updates and source count need an end-to-end trace.
- No performance claims until measured on representative hardware/browser.

## Work orders

### AU-01 — Audio graph ownership and routing
**Goal:** one authoritative AudioContext and master output graph per session.
- Inventory every AudioContext, Audio(), cloneNode(), createMediaElementSource() and direct destination connection.
- Define Auralis buses: master, music, ambience, weapons, Foley, UI, radio and world.
- Migrate music, wind, UI one-shots, whoosh, alerts and jukebox without changing behaviour.
- Preserve tablet muffling, wake sweep, reboot fade, per-category volume and unlock behaviour.
- Remove parallel graphs only after route parity and regression tests.
**Acceptance:** one context; every audible source accounted for; no duplicate media-source errors; all existing controls audible and functional.

### AU-02 — Voice lifecycle and allocation
- Introduce explicit voice handles and categories.
- Define deterministic priority/stealing policy and cleanup on ended/stolen/cancelled voices.
- Reuse or pool nodes only where profiling proves beneficial.
- Verify the 64-voice cap and no orphaned graph nodes.
**Acceptance:** stress test at and above cap; bounded node count; no stale voices.

### AU-03 — Acoustic correctness
- Add focused tests for distance curves, wall/door/window edge cases, moving listener, source/listener height, propagation delay and reflection geometry.
- Verify Rust outputs are consumed consistently by browser playback.
- Review coupling to AI SightGrid without duplicating geometry.
**Acceptance:** repeatable tests and representative listening scenes.

### AU-04 — Spatial audio
- Verify HRTF direction mapping, listener orientation, source elevation, distance and zoom scaling.
- Test headphone and stereo fallback behaviour.
**Acceptance:** directional test scene produces correct perceived direction and stable movement.

### AU-05 — Mixing and dynamics
- Implement category buses, master controls, snapshots and ducking.
- Review HDR, limiter and transient behaviour to avoid pumping/clipping.
- Keep UI/radio intelligible under combat.
**Acceptance:** settings change actual output; controlled loudness tests pass.

### AU-06 — Performance
- Profile dense combat, high echo counts, sustained streams and rapid scene changes.
- Measure CPU, active voices, AudioNode creation, memory and context state.
- Bound expensive reflections/reverb/filters based on measurements.
**Acceptance:** published baseline and no regression against it.

### AU-07 — Diagnostics and compatibility
- Add debug readout for context state, bus levels, active voices, dropped/virtualized voices, clipping and stream ownership.
- Test supported Chromium browsers, suspended/resume, device changes and missing assets.
**Acceptance:** failures diagnosable without breaking gameplay.

## Execution rules
- Each work order gets its own isolated branch and focused commits.
- Run relevant tests plus `bash scripts/verify.sh` before merge.
- Perform browser listening checks for audible changes; automated tests alone do not prove sound quality.
- Never merge unverified runtime changes.
- Record exact test commands/results and known limitations in the PR.

## AU-01 execution record — 2026-11-15

**Implementation:** committed on `audio/au-01-unified-graph-2026-11-15`.

- Auralis now owns the shared browser AudioContext and master-limited UI output bus.
- Persistent music and wind route through a dedicated media/world path with tablet muffle and reboot fade controls.
- Menu music, cinematic sequence, UI one-shots, radio analyser playback, tablet whoosh, update alert and jukebox all use the Auralis context.
- Removed the separate world, UI and radio-analyser AudioContexts and the menu's independent AudioContext.
- Added media-element route ownership checks and tests for reuse, analyser routing and stream/route conflicts.

**Verification:** tests added; execution and in-browser listening checks remain pending. Do not merge until CI and manual playback checks pass.
