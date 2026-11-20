# Scalia Agent Handoff

Start at the [README](README.md) for all workstreams and [CONTRIBUTING.md](CONTRIBUTING.md) for how to deliver changes. Docs index: [docs/README.md](docs/README.md). Verify this page against Git and CI before relying on it.

Last updated: 2026-11-20.

## Current animation lane (AN-02, prone and crawl)

Merged to `main`:
- AN-02-S03 ([#67](https://github.com/Hound-Protocol/Scalia/pull/67)): prone aim independent of crawl gait.
- AN-02-S04 ([#69](https://github.com/Hound-Protocol/Scalia/pull/69)): per-leg knee draw from the reference clips.
- AN-02-S05 ([#70](https://github.com/Hound-Protocol/Scalia/pull/70)): arms follow the knee draw.
- AN-02-S06 ([#71](https://github.com/Hound-Protocol/Scalia/pull/71), `ffcd47e`): forward crawl baked from the Mixamo reference (24-sample table in `pose.rs`, mirrored in `common.wgsl`).

- AN-02-S07 (this PR): smaller body twist and the elbow-on-ground test. This commit was pushed to the S05 branch after #70 merged, so it was missing from `main` until now.

Not verified: the in-game look. On 2026-11-18 the project lead reported the forward crawl looked unchanged in the model viewer after #71. The deploy for `ffcd47e` succeeded and the page referenced that build, so a cached page is the likely cause, but this was not confirmed. Visual QA is outstanding.

Known issues to fix next: the table also overrides the left shoulder and elbow (support hand may look wrong); the knee scaling is approximate; hips may sit too high; the near elbow is not planted.

Reference data: [mixamo-prone-reference.md](docs/handoff/mixamo-prone-reference.md). Raw Mixamo files are not in the repo and must not be committed. A scratch copy exists on the working machine only; delete it when tuning is finished.

## Next

1. Confirm the crawl in the model viewer after a hard refresh (check `models.js?v=` matches the live build).
2. Fix pass on S06 (above), then stand/crouch to prone transitions.
3. Implement the finished animations in the game so AI work can continue (project lead priority, 2026-11-20).

## Planned: ReMotion (motion file format and animation suite)

Approved by the project lead on 2026-11-20 as the plan, but implementation is deferred until the animation fixes and the animation-to-AI hookup are done. M0 spec: [docs/superpowers/specs/2026-11-18-remotion-m0-design.md](docs/superpowers/specs/2026-11-18-remotion-m0-design.md). No code written. Longer design in the Claude project doc `remotion-design.md`.

## Direction

After alpha, Scalia ships as a native Windows and Linux program for Steam, and the browser becomes a secondary tactical space. Keep the simulation, file formats and loaders free of browser dependencies.

## Open decisions (project lead)

- Licence for the repository (none is set yet).
- ReMotion M0: the three open decisions in the spec (repo location, GPU path, baseline first) stand at their recommended values unless changed.
- Which merged remote branches to delete (see [CONTRIBUTING.md](CONTRIBUTING.md#branch-retirement)).

Older handoff material: [docs/archive/](docs/archive/).
