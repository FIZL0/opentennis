# B646a: the menus' letter sheet as two channels

## What changed

- The letter sheet (`glyph_sheet`) is drawn as before (RGBA: white letters, white over a dark ring on the outlined half),
  with its mip levels, then compacted in its own buffer to `Rg8Unorm` (`glyph_sheet::two_channels`): R = the white's
  coverage, G = the whole letter's (white plus ring), both linear and premultiplied. Every texel is white over the ink
  (40/255), so straight colour + alpha give both numbers back. The buffer is truncated and shrunk: half the bytes on
  the CPU side and on the GPU. The ink value lives in `hud_gamma::LETTER_INK` now (glyph_sheet takes it from there).
- Rendering: the menus' letter quads go through Bevy's generic UI `ImageNode` slots, so there's no letter material.
  A node drawing from the sheet says so by its colour's alpha (`hud_gamma::letters`: +16; a hidden one stays 0), and
  `hud_gamma::two_channel_letters` patches Bevy's UI fragment (the `let color = select(…)` line, once, beside
  `mips::sharpen_ui`'s patch of the sample line) to rebuild `tint · (white + ink·(cover − white)) / cover`,
  alpha `node alpha · cover`. Callers: `main_menu::draw` (front-end and the match's pause/settings) and
  `pause::notice_draw`, for `Tex::Font` quads only.
- The GPU now filters premultiplied linear coverage instead of straight sRGB colour, which is if anything cleaner at
  edges; the stored levels are the same Lanczos levels as before (converted from them).
- mips.rs not touched: B646's rewrite of `MipSteps` isn't on this task's branch (it branched before B646 merged), so
  the conversion runs after it, on whatever it makes; nothing to merge there.

## Measured (headless Chromium, SwiftShader WebGL2, B645's setup; `context/B646a/after/menu.txt`)

| | B645 (before, `?log`) | this branch |
| --- | --- | --- |
| `read_sheets` kept | +50.1 MB | +26.2 MB |
| `read_sheets` peak | +94.1 MB | +94.1 MB (this branch lacks B646, whose row-by-row mips take it to ~+55) |
| menus' heap / live after start | 341 MB / 249 MB | 304 MB / 225 MB |

Sizes: desktop sheet 4096×2304 + 2 levels 49.5 → 24.8 MB; low-memory (2048 wide) 12.4 → 6.2 MB.

## A/B (desktop, release, `context/B646a/`)

`HST_MENU=main` and `confirm` at 1280×720 and `main` at 480×270 (the smaller mip levels), before/after binaries:
identical by eye (crops `crop-*.png`); pixel differences only on letter edges (≤ ~60/255 at the AA fringe, from the
premultiplied filtering) and the animated 3D preview. The browser build's title ("Press Enter", outlined) draws right
under WebGL2 (`after/menu.png`).

## Verification

- `tools/check.sh -p hst-play glyph_sheet` (new `two_channels_halve_the_sheet`), `-p hst-base two_channel`
  (`two_channel_letters_flagged`) pass; `hst` builds.

## Not verified / not 1:1

- Front-end only (menus' lettering): no sim, RNG or match timing touched; nothing to compare with the original.
- Not bit-identical to the RGBA sheet: the GPU filters premultiplied coverage now, and the colour is rebuilt from
  8-bit linear coverage (≤ ~1/255 off); edges differ slightly, the same by eye.
- Peak after merging with B646 not measured here (this branch lacks it); the conversion is in place, so it adds
  nothing to B646's peak unless the allocator copies on `shrink_to_fit` (then +25 MB briefly, under B646's +55).
- Not run on Safari/iPhone (no device); WebGPU build not run (same WGSL patch, `Rg8Unorm` is core in both).
- If a Bevy update changes the UI shader's colour line, the patch logs an error and the letters draw wrong (no test
  can see Bevy's embedded shader source here).
