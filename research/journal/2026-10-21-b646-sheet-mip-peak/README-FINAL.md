# B646: the letter sheet's start peak in the browser

B645 left the menus' start peaking +94 MB of wasm heap in `read_sheets`, building the letter sheet (4096×2304, 5
levels): the sheet's bytes (grown from 38 to 50 MB for its levels, a realloc that can copy) plus level 1 held whole as
f32 (38 MB) while level 2 was made from it.

## Changes

- `mips::MipSteps` now makes every level at once, row by row: each output row of a level is filtered from the
  few rows of the level above that its 8 taps reach (kept filtered across, so half as wide), and handed down to the
  next level as soon as it's made. No level is ever held whole in floats; at most ~9 rows per level (test). Same
  bytes as before: same taps, same order of adds, same clamps (`row_by_row_is_level_by_level` checks it against a
  whole-level float reference; before the change, a throwaway test hashed 8 images, cells and gains included, with the
  old code and the new code gave the same hashes). Its byte buffer is resized for every level up front and each row is
  written in place. `step()` still does ~32k output texels a call (B409's frames in between).
- `mips::room(w, h, levels)`: the bytes a sheet takes with its levels. `glyph_sheet::sheet` makes its pixmap's
  buffer with that capacity, so adding the levels never reallocates it.
- Court archives: not streamed. They're ~5 MB decoded per court (CMN + GRD01 + HOL01), so they can't be most of
  `hst::load`'s +44 MB peak; that's the models/textures built from them. Follow-up B647.

## Results (headless Chromium, SwiftShader WebGL2, same setup as B645; runs in `context/B646/`)

| | HEAD (r0) | B646 (r1, r2) |
| --- | --- | --- |
| `read_sheets` peak | +94.1 MB (kept +50.1) | +54.7 MB (kept +50.1) |
| `read_text` peak | +5.3 MB | under the 4 MB log line |
| menus' heap | 341 MB | 310 MB |
| menus → match heap (r2) | 357–381 MB (B645 r3) | 337 MB |
| `hst::load` peak | +43.8 MB (B645 r3) | +38.5 MB (run noise / the smaller heap; untouched) |

r2 went menus → match (Enter at 0 s, then every ~2 s, as B645 r3); the screen showed the welcome/loading card with
its lettering drawn right. Note: without an Enter at 0 s the fresh profile sat on a black screen on HEAD's build too
(r0), so that isn't this change.

## Verification

- `tools/check.sh -p hst-base mips` (new `row_by_row_is_level_by_level`), `-p hst-play glyph_sheet`, `menu_art`,
  `legend` pass.

## Not verified / not 1:1

- Front-end rendering only (mip levels of menu art): no sim, RNG or match timing touched; output bytes are identical
  to before (tested), so nothing to compare against the original.
- Browser numbers are headless Chromium + SwiftShader on Linux, not Safari/iPhone (no device here).
- The court-archive half of the task wasn't done: measured as not where the load's peak is; filed as B647.
