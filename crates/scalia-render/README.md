# scalia-render

Camera maths, meshes, the procedural pose rig and the wgpu renderer.

- Depends on: nothing in this workspace.
- `src/pose.rs`: `PoseParams` and every pose (very large; grep). `src/wgsl/common.wgsl` mirrors it for the GPU. Change both together; `tests/wgsl_parity.rs` keeps them equal.
- Other files: `skeleton.rs`, `mesh.rs`, `camera.rs`, `gpu.rs`, `sky.rs`, `bend.rs`.
- Design doc: [docs/animation-codex.md](../../docs/animation-codex.md).
- Tests: `cargo test -q --release -p scalia-render`.
