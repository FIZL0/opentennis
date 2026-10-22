# B645: web match load's wasm heap peak

B643 run6: the menus' start peaked 355 MB live (heap 423 MB), an online match's load 444 MB live (heap 560 MB)
against ~300 MB once playing. The wasm heap never shrinks, so the peak is what the tab keeps.

## How it was measured

- New: the start's sliced executor (`hst-play/src/mode.rs`) logs, in the browser, each start system that peaked
  4 MB or more over the bytes live before it: `start: NAME peak +P MB, kept +K MB (live L MB)` (`web::span` /
  `web::span_peak`, a second peak counter beside B468's, so `memory_log` is untouched).
- Build: `target-wasm` release `--features ship`, wasm-bindgen into `web/pkg` (no wasm-opt; it doesn't change the
  heap), `python3 -m http.server 8843 -d web`, headless Chromium (SwiftShader WebGL2), disc imported into a fresh
  profile with B643's `probe.mjs`. Runs in `context/B645/` (`r1` before, `r2`/`r3` after):
  `?play&pads=-,-,-,-&log` (match straight away), `?log` (menus), and menus → match driven by Enter presses (`r3`,
  the same path as an online match: the menus loaded, then the match on top).

## What was found

r1 (before): `pause::setup peak +153.2 MB, kept +75.8 MB`; every other start system ≤ 16 MB. The pause screen (and
the main menu's `read_sheets`) draws the menus' letter sheet (`glyph_sheet`, 4096×2304 with 3 levels, ~50 MB), and it
was drawn again on every menu ↔ match switch: the menus' copy is freed with the menus, so the match drew its own,
and back. Drawing it held several copies at once:
- `unpremultiply` collected the pixmap's pixels into a second buffer through an iterator with no size hint, so it
  grew by doubling (up to 64 MB of capacity plus the copy) while the 38 MB pixmap was still alive;
- `MipSteps` pushed each level's f32 texels into a Vec that grew by doubling (level 1: 38 MB, capacity 67 MB, plus
  the copy when it grew), and appended each finished level to the image's bytes, which grew (and copied) again.

## Changes

- `glyph_sheet::image` keeps the sheet's handle (a static) and hands it out again while the store still holds it
  (its size checked, so another app's handle in a test isn't taken for it). Built once per session; menu ↔ match no
  longer rebuilds it. GPU memory is unchanged (both sides held one anyway). Also a little faster to switch.
- `menu_art::unpremultiply` demultiplies in the pixmap's own buffer (`Pixmap::take`): same bytes (test).
- `mips::MipSteps` reserves the image's bytes for every level up front and each level's f32 buffer at its exact
  size: same output, no doubling.

## Results (wasm heap / peak live)

| run | before | after |
| --- | --- | --- |
| menus' start | heap 423 MB, peak 355 MB (B643) | heap 341 MB, peak 297 MB; steady 235–243 |
| `?play` match load | heap 446 MB, peak 378 MB (r1) | heap 367 MB, peak 326 MB; steady 243–255 |
| menus → match (online's path) | heap 560 MB, peak 444 MB (B643 run6, online) | heap 357–381 MB, peak 332 MB; steady ~295–300 (r3, offline) |

In r3 the match's `pause::setup` is now `peak +4.9 MB` (was +153). What's left at a match load from the menus:
`hst::load` (court archives read whole) +44 MB peak / +15 kept, `play::setup` +16, `menu::setup` +11; the load's peak
is ~10% over the steady size. The one sheet build still peaks +94 MB at whichever start comes first (menus, or
`?play`): the pixmap/byte buffer (50 MB with the levels' room) plus level 1 kept as f32 (38 MB) for level 2. Follow-up
B646 (halve that, stream court archives); B644 (smaller sheet on phones) noted.

## Verification

- `tools/check.sh -p hst-play glyph_sheet` (new `the_sheet_is_drawn_once`, `unpremultiplied_in_place`), `-p hst-base
  mips`, `-p hst-play menu_art`, `-p hst-play legend` pass.
- Browser runs above (`context/B645/r1`, `r2`, `r3`); the menus and match came up and played in each (r3 reached the
  match through the menus).

## Not verified / not 1:1

- Not run on an iPhone or in Safari (no device here): all numbers are headless Chromium + SwiftShader WebGL2 on
  Linux, the same setup as B643. WebKit's allocator and per-tab accounting differ.
- The menus → match run was an offline match through the menus, not an online one against a native host (that needs
  a native build at the same build id); the load path after the menus is the same `Mode` start, and the online
  match's extra start (netplay) isn't a heap item in B643's log.
- Front-end/rendering only (menus' lettering, mips): no sim, RNG or match timing touched; the sheet's pixels are
  unchanged (the in-place unpremultiply is tested byte-for-byte against the old one; the mip change only reserves).
