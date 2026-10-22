# B647: the court load's peak in the browser

B646 r2 measured `hst::load` at `peak +38.5 MB, kept +14.3 MB` (menus → match, random court). Here the court is
fixed (`?play&stage=N&pads=-,-,-,-&log`, the menus' letter sheet is built first as on the menus path), and temporary
`span_peak` marks inside `load_court` (removed again) showed where it was held:

1. `court_library` parsed each kept model twice (once for the raw model, once to build), kept every kept model's
   `Mtl` (decoded textures with their levels) in `raw_models` until the end of the load, and collected an archive's
   meshes and materials before adding them.
2. `hst_sim::court::world` (the shade map's collision world, read at the end, on top of everything still live)
   parsed every model of the three archives (every hole/season variant) with decoded textures and drawn geometry and
   held them all: +11 MB on its own. `play::setup` calls the same (`court_load::moving`).
3. `court::drawn` held every model's drawn geometry the same way (only their nodes are used).
4. `hst_sim::shade::build` adds the casters' reds up in a 1280×896 `u32` buffer (4.6 MB).

## Changes

- `gs_models_anim` hands each model to a callback as it's built (with its parsed model and materials); `court_library`
  does its box/shape/library work there and adds the meshes and materials at once, one pass per model. Each court
  archive is dropped after its pass. `raw_models` keeps only the models used later: the posed structures'
  (`court::drawn` is now called before the library, to know them) and the hole's, and the materials/textures only
  for the hole (the shade map's).
- `load_court` reads the shade map's collision world before the models are built (court 7 has none), so its read
  doesn't stack on top of the built library.
- `court::moving` drops each model's drawn geometry and textures right after parsing (collision reads nodes,
  collision faces, wrap and the materials' attribute maps only); `court::drawn` drops the drawn geometry.
- `shade::build` adds the reds saturating in a `u8` buffer: every sample is ≤ 0xff and the sums were only ever read
  clamped to 0xff, so `min(sum, 255)` is the same byte and the map is unchanged.

## Results (headless Chromium, SwiftShader WebGL2, `context/B647/run.mjs`; `r0` HEAD, `r10` this)

| court | `hst::load` peak HEAD → B647 | kept | `play::setup` peak HEAD → B647 |
| --- | --- | --- | --- |
| 1 | +41.0 → +23.0 MB | +14.3 (same) | +15.3 → +8.7 MB |
| 5 | +29.5 → +16.7 MB | +10.3 (same) | +11.9 → +8.7 MB |
| 10 | +42.3 → +20.9 MB | +13.9 (same) | +16.5 → +8.8 MB |

Heap after the match start: 371 → 358 MB (court 1), 359 → 353 (court 5), 372 → 357 (court 10). What's left of the
peak is what the library keeps (meshes, textures waiting for upload) plus the hole model and its textures held until
the shade map is built (~4 MB) and GAME.BIN (1 MB).

## Verification

- `tools/check.sh -p hst-sim --test shade` (9), `--test world` (5, collision world vs the game's RAM list),
  `--test bodyhit` (2), `-p hst-play court_load` pass.
- Native `--play --stage 6` shot (`context/B647/shots/`): court, props, shade map and players draw as before.
  A pixel diff against HEAD isn't possible: two runs of the same binary already differ (live match timing).
- The browser runs reached the match on all three courts, no exceptions.

## Not verified / not 1:1

- No pixel-exact before/after comparison of the court's draw (shots of a live match differ run to run); the court
  models' meshes, materials and textures are built by the same code in the same order, only added sooner.
- Rendering-side load work only; the collision world is the same (world test against the game's RAM), no RNG or
  match timing touched.
- Browser numbers are headless Chromium + SwiftShader on Linux, not Safari/iPhone (no device here).
