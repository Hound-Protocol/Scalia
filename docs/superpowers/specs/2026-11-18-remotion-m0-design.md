# ReMotion M0: motion file format and playback

Status: draft, awaiting owner approval. No code until approved.

## Goal

A player-readable motion file format, `.remotion.json`, and a way for the game to play one in place of a built-in
motion. M0 proves the format on the forward crawl baseline. It adds no editor.

## Scope

In M0:
- Format v1: schema, validation, readable errors.
- Sampler: a motion evaluated at any phase, wrapping at the loop.
- Baseline export: the forward crawl written as the first `.remotion.json`.
- Playback in the model viewer, preview only.

Not in M0:
- Editor UI (M1). Reaction and linked controls (M1). Constraints and IK (later).
- Mod loading, manifests and Steam distribution (M3).
- The GPU path (see Decision 2).

## Where it lives

- Crate `crates/remotion`: format types, load and validate, sampler, baseline export. Depends on `scalia-render`
  for `PoseParams`. No dependency on the browser, so the native build can reuse it.
- Test data: `crates/remotion/tests/data/`.

## Format v1

```json
{
  "format": 1,
  "name": "Crawl forward",
  "loop_seconds": 0.75,
  "stride_m": 0.6,
  "author": "Scalia team",
  "licence": "CC0",
  "keys": [
    { "at": 0.0,  "knee": [0.05, 1.00], "twist": -0.27 },
    { "at": 0.25, "knee": [0.25, 0.21], "twist": -0.26 }
  ]
}
```

Top level:
- `format`: integer, must be `1`.
- `name`: non-empty string.
- `loop_seconds`: number in (0, 10].
- `stride_m`: optional number in (0, 5]. Metres per loop, used for speed. Absent means the motion is an idle, not a walk.
- `author`, `licence`: optional strings, metadata only.
- `keys`: array of 2 to 256 keys, sorted by `at`.

Each key:
- `at`: fraction of the loop, in [0, 1). The first key must be at 0.
- Parameter fields, all optional. Missing fields keep the previous key's value; the first key must set every field it uses.

Parameter fields in M0 (names match `PoseParams`, units match the code, radians unless noted):

| Field | Type | Meaning |
|---|---|---|
| `lean` | number | upper body lean forward |
| `hip` | [left, right] | hip angles |
| `knee` | [left, right] | knee bend |
| `shoulder` | [left, right] | shoulder angles |
| `elbow` | [left, right] | elbow bend |
| `reach` | [left, right] | inward arm swing |
| `abduct` | [left, right] | arm raised out to the side |
| `side` | [left, right] | leg swung out to the side |
| `twist` | number | upper body turn |
| `head` | number | head nod |
| `look` | number | head turn |
| `rifle` | number | rifle pitch |
| `rifle_yaw` | number | rifle turn |
| `bob` | number, metres | whole-body rise and fall |

Other `PoseParams` fields (`prone`, `drop`, `sling`, `tilt`, `flat`) are not in M0. Adding one later is a format
change only if it is required; new optional fields are backward compatible.

Interpolation: linear between keys, wrapping from the last key back to the first across the loop. Easing is
reserved (`"ease": "linear"`, the only allowed value in v1).

## Validation

- Unknown top-level or key fields that are not listed above are ignored, so newer files load on older builds.
- A parameter name that is not in the table is ignored with a warning in the log, never an error.
- Errors are returned as a list, each with the file, key index, field and a plain sentence, e.g.
  `keys[2].knee[0]: 3.4 is outside the allowed range of -3.2 to 3.2 radians`.
- Angle fields are limited to [-π, π] at format level. Tighter, pose-specific limits are an M1 editor concern.
- A file with any error is rejected whole. Nothing partial is played.

## Sampler

- `sample(motion, phase) -> PoseParams`: `phase` in any real number, reduced modulo 1.
- For `phase` between keys `a` and `b`, each set field is linearly interpolated. Fields that `a` does not set are
  taken from the earlier key that does.
- The segment from the last key back to the first wraps at 1.0.
- Exact at each key's `at`.

## Baseline export

- Samples the current forward crawl (`Pose::Crawl`, direction 0) at 16 evenly spaced phases and writes keys for
  the M0 field set. Values are rounded to 4 decimals.
- Test: sampled playback, compared with `params_dir(Pose::Crawl, phase, 0)` at 64 phases not used for export,
  stays within 0.02 rad for every field the file sets.

## Playback in the viewer

- The model viewer gets a selection for "Motion: built-in / file", and a file picker.
- Playback sets the figure's phase from the loop, so `loop_seconds` controls speed.
- Built-in motions are unchanged when the selection is "built-in".

## Tests and checks

- Round-trip: load, save and load again gives the same motion.
- Validation: each error case in the Validation section has a test with the expected message.
- Sampler: exact at keys, wraps at the seam, unset fields carry over.
- Baseline export test as above.
- `scripts/check.sh` passes. `hash_battle` must not change, since no simulation code is touched.

## Decisions for the owner

1. **Location.** Inside the Scalia repo, as `crates/remotion`. Recommended.
2. **GPU path in M0.** The viewer's figure is drawn by the shader, so a file played through the GPU path needs
   the sampled values sent to the shader per figure. Recommended: M0 plays through the CPU path only, which is
   what the tests and the baseline check use. The GPU path comes in M2 with export and WGSL generation.
   Downside: the preview in the viewer will not show a file's motion until M2. If you want the viewer to show it
   in M0, that adds the per-figure upload to M0's scope.
3. **Baseline first.** The forward crawl is the first file. Recommended.
