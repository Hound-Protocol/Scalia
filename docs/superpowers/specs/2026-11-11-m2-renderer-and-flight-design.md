# M2: renderer, units on the ground and the first flight (draft for approval)

Status: draft. Nothing here is built. Every "Recommended" is my default; say so if you want another.

## Where we are

Live on Pages: six blueprints as drawings and one terrain chunk, all from WebAssembly. Missing: anything 3D, any soldier on screen, and any way to leave a planet.

## Intent (what you said)

- Top-down isometric camera; zoom from the galaxy down to one soldier; one continuous world, no teleporting, no pause.
- PS1 look: low-poly, low-resolution textures, no vertex wobble.
- First units, then a test flight from one planet to another.
- Original ship designs only.

## Slices, in order, each ends on Pages

**A. Renderer foundation.** A WebGPU canvas, an isometric camera, a terrain patch and one blueprint mesh drawn with flat colours from the blueprint palette. Pure CPU parts (matrices, camera, mesh buffers) are unit-tested natively; the draw itself can only be seen on a real GPU.

**B. Units on the ground.** `derive_visible` output drawn as instanced low-poly figures (about 20,000 on screen), standing, marching and boarding; zoom out to see groups as blocks.

**C. First flight.** A new process kind `Flight` in the core: a timestamped record from a point above one planet to a point above another, in a shared system region, so position at any time is a pure function of the record and the time (no teleport). The Ox takes off, flies and lands; the page shows it.

## Decisions

1. **Graphics API.** Recommended: WebGPU only for now (matches the plan so far; recent Chrome and Edge). Alternative: also a WebGL2 fallback, which roughly doubles renderer work.
2. **PS1 look.** Recommended: render to a low internal resolution (about 480 by 270) and scale it up with nearest-neighbour, with a toggle to render at full resolution. Alternative: full resolution only, with the look coming from models and textures alone.
3. **Flight path.** Recommended: straight transfer between two fixed points in the system region, with an eased start and stop, because it keeps position a pure function of time. Alternative: real orbital mechanics, a much larger change.
4. **Planets in space.** Recommended: two planets at fixed positions for this milestone; moving orbits later. Alternative: circular orbits from the start (integer angle table).

## Testing limits

The sandbox has no GPU, so rendering can only be checked as far as: CPU-side maths and buffers (unit tests), shader compile in a browser (on Pages), and "no errors, frame drawn" reported by the page. Whether it looks right is for you to judge on Pages.

## Not in scope

Combat, economy, networking, persistence, textures beyond flat palette colours, sound, cross-face terrain continuity.
