# B629 — space outside the safe area

## What looked bad
Baseline shots (`context/b629/before/`): with Aspect Ratio on Auto the menus already filled the window (B623's
`Bleed` copies). The flat black bars came from a **fixed** Aspect Ratio (4:3, 16:9, 21:9 on a window of another shape):
menus, title splash, loading screen and the match all sat in a centred rect with black bars, and on a phone the
fixed-ratio picture also had black around the notch side.

## What changed
- `widescreen.rs`: `Bleed` copies now extend to the whole window, not just the picture. `bleed_bars`: past the picture a
  node only grows along an axis it already spans (the menu wall into every bar; a title bar or legend strip sideways
  only; side cards and text never). `bleed_in` keeps edges that are already past the view. New `cover_window`
  (percent-of-view rect of a picture covering the window) and `bleed_aspect` (window aspect).
- `hud_gamma`: the HUD image is window-sized (UI camera viewport = the view inside it), so bleeds reach the bars; the
  composite draws the whole window, the 3D picture in its rect and, outside it, a soft dimmed glow of the picture's edges
  (48×48 downsample pass, fading out within half a picture height; strength 0.45). Original look keeps black bars.
- `loading.rs`: the court picture is a `Backdrop` covering the whole window (cropped, never stretched), as a Bleed.
- `splash.rs`: the title court frames to the window's aspect.
- `pipelines.rs`: record VERSION 2 (the composite's bindings changed; a stale record failed validation and quit).

## Checked
`--shot` at 4:3 1024×768, 16:9 1280×720, 16:10 1280×800, 21:9 1680×720, phone 852×393 / 393×852 with `HST_SAFE`,
each with Auto and with fixed 4:3 / 16:9 / 21:9 settings (scratch `HST_DATA` dirs with `aspect = …`):
`context/b629/r2`…`r5`, sheets `sheet-r3-match.png`, `sheet-r4-load.png`, `sheet-r5.png`. Auto shots vs `before`:
same layout, <1.5 % pixel difference (capture resampling). HUD/score, cards and text stay inside the view in all.
Tests: `walls_bleed_to_the_picture` (past-edge case), `walls_bleed_into_fixed_ratio_bars`.

## Not verified / not 1:1
- The original is 4:3 only, so what fills the outside area is our design; no original to compare.
- Glow strength/reach picked by eye on a desktop monitor.
- No real phone, tablet or Steam Deck run: phone shapes only through window sizes + `HST_SAFE` (B623 / B620 cover devices).
- The loading cover was shot through a temporary hook that showed the loading screen directly (removed), not through
  a real menu → match load.
- The pipelines VERSION bump drops everyone's existing pre-warm record once (first launch re-records). Follow-up P136:
  make the record survive shader binding changes on its own.
