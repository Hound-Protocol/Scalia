# scripts

## Verification (run these)

| Script | Purpose |
|---|---|
| `check.sh` | Quiet wrapper for `verify.sh`. Prints one line on success, the failing step on failure. Run before every push. |
| `verify.sh` | The full check: link check, release metadata tests, browser tests, cargo tests (debug and release), animation audit, budgets, wasm build and hash match. |
| `check_links.py` | Fails on broken relative links in Markdown files. |
| `release_metadata.py` | Stamps the build identity. Used by the Pages workflow. Tests in `tests/`. |

## Asset tools (run by hand, not in CI)

| Script | Purpose |
|---|---|
| `glb_figure.py`, `glb_inspect.py`, `glb_prop.py`, `glb_render_front.py` | Convert and inspect GLB models |
| `first_person.py`, `fp_hands.py`, `pda_relief.py`, `widen.py` | First-person arms and the command tablet |
| `filmstrip.py`, `stance_sheet.py` | Render pose sheets for review |
| `gun_sounds_split.py`, `radio_voices.py` | Audio asset preparation |

These need extra tools (Python imaging libraries, Blender-style inputs) that CI does not install. Each file's header says what it does.
