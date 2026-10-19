# B570 — menu text on low-res screens and phones (FINAL)

## What was wrong
- The menu art (word.tm2 glyphs, pills, labels, key glyphs) is built at 4× the disc's size and had a single level.
  In a small window the UI sampler bilinear-picked every 4th–12th texel, so the letters came out aliased and broken.
  At 640×360 a narrow glyph (an "i") also vanished: Bevy UI rounds node rects to whole pixels, so a quad less than
  1 px wide rounds to nothing.
- Menu Size Auto went by physical pixels, so a HiDPI laptop or phone shrank the menu to 80% on top of the DPI
  scaling.

## Fix
- `hst-base/src/mips.rs`: builds smaller levels for UI images. Each level halves the last with a separable 8-tap
  Lanczos-2 filter, in premultiplied linear light. Glyph sheets are filtered cell by cell (32-disc-px cells), so no
  glyph bleeds into its neighbour, and their small levels get 1.5× coverage. A plain filter leaves dark text on
  light pills pale, because the UI blends in linear light.
- wgpu has no sampler LOD bias, so the loaded `ui.wgsl` is patched to `textureSampleBias(…, -0.75)`, which picks
  the sharper level.
- `MipSteps` builds the levels a band at a time (about 32k texels, ~1.5 ms native). The menu-art loader's `Steps`
  hands off to it, so the web build's step loader (B409) doesn't stall. A 2048² sheet takes about 50 ms native in
  total.
- Levels go down to 1× the disc's size for sheets and labels, ½ for the pills (their pieces sit 2 px apart), and
  ¼ for word.tm2 (cell gaps ≥7 disc px across; only 1 px vertically, but cells are filtered on their own anyway).
  Key-legend glyphs go down to 8 px.
- `Art::text` pads each glyph quad with up to 2 px of its own clear cell on each side, so a small "i" still spans
  a pixel.
- Menu Size Auto now uses the logical (DPI-scaled) height of the 4:3 menu screen: 80% only past 1440 logical px.

## Checked
- A/B shots in `context/b570/` (before/after, 640×360, 1280×720, 1920×1080, portrait 450×900, `final-*.png` at 2×
  monitor scale). Small windows show smooth, solid text with every letter present. 1080p and up look the same as
  before.
- Tests: `mips` (colour kept, no dark fringe, cells kept apart, gain), menu_art, legend, main_menu, menu_size.

## Not verified / not 1:1
- Menus aren't required to match the original 1:1 (PLAN), so no comparison against PCSX2.
- Not tried on a real phone or in the wasm/WebGL2 build. `textureSampleBias` is assumed to work there (it is
  core WGSL in fragment shaders), and the stepped mip timing in the browser wasn't measured.
- The ui.wgsl patch looks for Bevy's exact `textureSample(sprite_texture, sprite_sampler, in.uv)` line. If a Bevy
  update changes it, the patch logs a warning and does nothing (mips still apply, just a little softer).
- At 640×360 physical the smallest text (~6 px) and the key legend are still very small. That's a layout limit,
  not a sampling one.
