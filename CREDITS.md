# Credits

## Third-party assets

- **"low poly soldier"** by [Kolos Studios](https://sketchfab.com/KolosStudios), licensed under
  [CC BY 4.0](http://creativecommons.org/licenses/by/4.0/).
  Source: https://sketchfab.com/3d-models/low-poly-soldier-daf5de38902e458aa57ff5ba9460ca02
  Changes: re-posed (arms down), fitted to our rig, recoloured to our palette, rifle decimated, team armband added
  (`scripts/glb_figure.py`; original kept at `assets/source/low_poly_soldier.glb`).
- **Radio key click** (`web/sfx/radio-key.mp3`): "alexis_gaming_cam radio 338296", supplied by the project lead
  (Pixabay sound library naming).
- **"Military Rugged PDA / Tactical Tablet"** by [Snupp](https://sketchfab.com/snuupp), licensed under
  [CC BY 4.0](http://creativecommons.org/licenses/by/4.0/).
  Source: https://sketchfab.com/3d-models/military-rugged-pda-tactical-tablet-24b5e19dcb9e4970b5c211787c7f3fd0
  Changes: rendered face-on for the tablet close-up (`scripts/glb_render_front.py`, `web/img/pda_front.webp`) and
  recoloured to our palette for the commander's hands (`scripts/glb_prop.py`); original at
  `assets/source/military_rugged_pda.glb`.
- **Grass footsteps** (`web/sfx/steps-grass.mp3`): "joentnt walk on grass 1 291984", supplied by the project lead
  (Pixabay sound library naming).
- **Concrete footsteps** (`web/sfx/steps-concrete-1.mp3`, `-2`, `-3`): "yarzur_ofc st1, st2, st3 footstep sfx
  323053, 323055, 323056", supplied by the project lead (Pixabay sound library naming).
- **UI sounds** (`web/sfx/pda-open.mp3`, `pda-close.mp3` reversed from it, `pda-tap.mp3`): "juniorsoundays ui sound 10
  527836" and "ui sound 39 527833"; **dive swoosh** (`web/sfx/dive-whoosh.mp3`): "stereogenicstudio swish swoosh woosh
  sfx 25 357177"; all supplied by the project lead (Pixabay sound library naming).
- **Weapon sounds** (`web/sfx/weapons/`): "jigokukarano_sisya ar15 168855", "ak47 168856", "mp5 168858", "m1911a1
  168854" and "desert eagle 168857", supplied by the project lead (Pixabay sound library naming). Changes: each
  recording cut into its single shot and handling pieces (magazine out, in, bolt or slide, dry click), levelled,
  mono (`scripts/gun_sounds_split.py`).
- **Menu sounds** (`web/sfx/menu-*.mp3`): "soundreality cinematic tension 2 504666" (menu music), "submority boom
  geomorphism cinematic trailer sound effects 123876" (start, cut to 8 s), "chrysalyn intense cinematic wind whoosh
  transition 543526" (the travel in, cut to 6 s) and "biww whoosh fly by sound effect 561945" (entering the
  commander); supplied by the project lead (Pixabay sound library naming). Re-encoded at 128 kbit/s.
- **Intro sequence sounds** (`web/sfx/menu-flash1-door.mp3`, `menu-flash2-thump.mp3`, `space-ambience-01.mp3`,
  `space-ambience-03.mp3`): "soundreality cinematic hit door 619512" (first flash), "blendertimer massive thump 116359"
  (second flash), "fronbondi_skegs sfx deep space travel ambience 01 358466" and "... ambience 03 358469"; supplied by
  the project lead (Pixabay sound library naming). Not yet wired in.
- **"walkie talkie"** by [FayBall](https://sketchfab.com/FayBall), licensed under
  [CC BY 4.0](http://creativecommons.org/licenses/by/4.0/).
  Source: https://sketchfab.com/3d-models/walkie-talkie-88d0177b7801448397b69fcd81220c3a
  Changes: rendered face-on with its display blanked for the game's live radio screen (`web/img/walkie.webp`).

## Music
- "Operation Vance" — temporary placeholder track supplied by the project lead (`web/sfx/music-vance.mp3`).
