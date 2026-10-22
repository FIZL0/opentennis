# B644 — the menus' letter sheet at half scale where memory is short

The glyph sheet (`hst-play/src/play/glyph_sheet.rs`) was always built at `menu_art::K` = 4: 4096 × 2304 RGBA with
levels, ~50 MB on the GPU. Now `glyph_sheet::scale()` builds it at `K/2` (2048 × 1152, ~12.5 MB with levels) when
`hst_base::textures::low_memory()` says so:

- `HST_LOW_MEMORY=1`/`0` overrides anywhere;
- the iOS/Android apps: always;
- the browser: `web::touched()` (a touch screen, or touched once) or Chromium's `deviceMemory` ≤ 4 GB.

The layout is in sheet units, independent of the scale; `textures::ours` is told the scale actually built, so the
UI's pixel rects follow. It's decided on each build (the sheet is rebuilt on each menu↔match switch), so a browser
touched after load gets the half sheet from the next switch on.

Chose half scale over one channel: the outlined half carries two colours (white fill, near-black ring), so a
single channel would need a shader change on every quad that draws it; half scale is a 4× saving with no draw change.

## A/B (context/b644/, `HST_LOW_MEMORY=0` vs `1`, release build)
- `main`, `chars`, `settings` at 1280×720 (2048×1152 PNG) and `phone-main` at 844×390: identical by eye, the large
  titles a hair softer at desktop size only.
- In-match text over the court (`notice-*`; the `HST_NOTICE` run showed the "Press Enter / or plug in a controller"
  prompt, drawn from the same sheet): identical by eye.

Test: `glyph_sheet::the_half_scale_sheet_keeps_the_layout` (size halves, `H` drawn where the layout puts it, pad clear).

## Not verified / not 1:1
- Not run in a real phone browser or the apps; the memory figure is computed from the sheet size (2048×1152×4 ×4/3),
  not read back from a browser's GPU memory counters.
- The sheet is still rebuilt (CPU) on every menu↔match switch; only its size changed. Caching it was not in scope.
- Front-end text, so no match against the original applies (PLAN: menus are ours).
