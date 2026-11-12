# M3 decisions: own engine on wgpu, wrapping planets

Status: decided by the project lead on 2026-11-12.

1. **Engine:** our own, built from the ground up on `wgpu` (the Rust WebGPU library) with focused libraries
   (`glam` for maths, `naga` to validate shaders). No general engine (Bevy considered and declined: the
   deterministic core must stay ours, the pixel-art pipeline is custom, and the web build must stay small).
   Browser features cover UI (HTML/CSS) and audio (Web Audio).
2. **Renderer moves from JavaScript into Rust** (`crates/scalia-render`), compiled to WebAssembly; the page only
   forwards input and the animation frame. Same look: 480x270 target with nearest upscale, flat 4-step lighting,
   1-pixel silhouettes and convex-edge highlights, texel-snapped camera, hard sun shadows, instanced figures.
3. **Planets are wrapping maps:** each planet is one flat rectangle that wraps east-west; the top and bottom
   edges are impassable polar ice. From orbit the same map is drawn as a globe. This replaces the cube-sphere.
4. **Priority for the look:** mood and atmosphere (lighting, fog, haze, light shafts, day and night).

Open: what "triangle-based surface" means for the terrain (faceted grid, adaptive density, or cut features).
