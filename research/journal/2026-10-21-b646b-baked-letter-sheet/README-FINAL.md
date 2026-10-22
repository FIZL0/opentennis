# B646b: the menus' letter sheet baked at build time

## What changed

- The letter sheet (`glyph_sheet`) is no longer drawn at start. Both scales (K = 4096 wide with 5 levels, and the
  low-memory K/2 = 2048 wide with 4 levels) are baked into the game (`include_bytes!`) as
  `crates/hst/assets/fonts/letters-k4.bin` (746 KB) and `letters-k2.bin` (332 KB). They're drawn from our own OFL
  font, so they're ours to ship, beside the font and its OFL.txt. Format: for each level, the PNG's length (u32 LE) and
  then a grey+alpha 8-bit PNG. Grey is the white's coverage and alpha the letter's, so the bytes are B646a's `Rg8Unorm`
  texels exactly.
- `glyph_sheet::baked(k)` reads each level's PNG straight into one buffer sized for all the levels (`mips::room / 2`),
  so nothing besides the sheet is held except the PNG decoder's state. It sets the same trilinear sampler `MipSteps` gives.
- The drawing (`sheet`, `two_channels`) and the new `bake` are now test-only. It's the same code as before, so the baked
  file is exactly what the game drew.
- Staleness check (I picked a check, not a build.rs, because a build script can't run this crate's drawing code, which
  needs menu_art's outline and hst-base's mips without Bevy): `baked_sheets_are_current` draws both scales the way the
  code does now and compares them to the baked files. It allows only the odd texel 1–2 steps off, for float maths that
  might differ on another machine. Any change to the font, the `FONT` list, the layout, the ring/ink or the mip filter
  fails it with the command that rebakes:
  `HST_BAKE_LETTERS=1 tools/check.sh -p hst-play baked_sheets_are_current`.
  Checked: with the ring at 3.5 instead of 4, it fails with 288k of 25.1M bytes differing (max 255).
- mips.rs, menu_art.rs and build.rs are not touched (no build script needed).

Note: B646a's merge (fe1316d1) was still in the runner's gate when this started. It sat directly on this branch's
base, so the branch was moved forward to it (a fast-forward, no merge commit) to build on the two-channel sheet.

## Measured (headless Chromium, SwiftShader WebGL2, B645's setup; `context/B646b/`)

Release `--features ship` wasm, wasm-bindgen, no wasm-opt. Menus started with `?log` on the same imported-disc profile,
three runs each. The before build is this branch's base with the old `glyph_sheet.rs`.

| | before | after |
| --- | --- | --- |
| `read_sheets` peak / kept | +48.5 / +26.2 MB | +30.7 / +26.2 MB |
| `read_sheets` time | 352–367 ms | 55 ms (run 1); under the log's threshold in runs 2–3 |
| menus' mode start (`started in`) | 724–777 ms, 5 frames | 511–598 ms, 6 frames (runs 2–3; run 1, the first visit after the import: 1127 ms) |
| wasm memory / live peak | 277 MB / 261–265 MB | 272–274 MB / 250–252 MB |

The peak's +4.5 MB over the sheet is the PNG decoder (zlib window, a row) plus the level being read. The .wasm grows by
~1 MB (both baked files).

## Verification

- `tools/check.sh -p hst-play glyph_sheet`: 6 pass, including the new `baked_sheets_are_current` (exact today) and
  `the_sheet_is_drawn_once` (now through the baked path).
- The browser after-build's title screen draws its outlined "Press" right (`context/B646b/after-3.png`).

## Not verified / not 1:1

- Front-end only (the menus' lettering): no sim, RNG or match timing touched, and nothing to compare with the original.
- No native A/B screenshots: the test proves the baked bytes, size, format, level count and sampler equal what was
  drawn before, so the GPU gets identical data.
- The low-memory (K/2) sheet was checked only by the test (byte-equal to its drawing), not in a browser run on a
  low-memory device.
- Not run in Safari, on an iPhone or in the WebGPU build: same image data, and the PNG decoding is plain Rust.
- The menu start times come from one machine with a software GPU and vary by about ±50 ms between runs.
