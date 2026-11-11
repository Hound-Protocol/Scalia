# M2: renderer, units on the ground and the first flight (draft for approval)

Status: decisions made by the user on 2026-11-11 (below). Built in two parts: M2a (orbits and flight in the core and wasm, testable natively) then M2b (renderer and page).

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

**C. First flight.** A new process kind `Flight` in the core: a timestamped, eased record from a point above one planet to a point above another on a moving orbit, in a shared system region, so position at any time is a pure function of the record and the time (no teleport). The Ox flies it; the page shows it.

## Decisions (made by the user)

1. **Graphics API:** WebGPU only for now. No WebGL2 fallback.
2. **PS1 look:** render at a low internal resolution (about 480 by 270), scale up with nearest-neighbour, with a toggle for full resolution.
3. **Flight path:** an eased transfer between two points. The path leads the destination: the arrival point is where the planet will be when the ship gets there. Easing is smootherstep, so speed is zero at both ends.
4. **Planets:** on circular orbits from the start. Angles come from an integer sine table, so positions are a pure function of time and identical native and in WebAssembly.

## Design notes that follow from the decisions

- A new `system` region holds a star and bodies on circular orbits (radius, period in real milliseconds, starting phase). `body_pos(body, t)` is a pure function of time.
- `ProcessKind::Flight { from, to }` is a timestamped record like a march, but its fraction of the way is eased. `plan_flight` picks the duration by iterating a fixed number of times on the moving target, in fixed point.
- Soldiers on a transport are not drawn in flight; the ship is. The ship's pose comes from the flight record.
- The renderer's presentation maths (camera, GPU buffers) lives in a new f32 crate, `scalia-view`, kept out of the integer-only core. Shaders are checked natively by parsing them with `naga`, since the sandbox has no GPU.

## Testing limits

The sandbox has no GPU, so rendering can only be checked as far as: CPU-side maths and buffers (unit tests), shader compile in a browser (on Pages), and "no errors, frame drawn" reported by the page. Whether it looks right is for you to judge on Pages.

## Not in scope

Combat, economy, networking, persistence, textures beyond flat palette colours, sound, cross-face terrain continuity.
