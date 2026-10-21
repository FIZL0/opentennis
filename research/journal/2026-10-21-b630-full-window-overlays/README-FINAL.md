# B630 — overlays and backgrounds fill the whole window

## What looked weird
Baseline through a real menu → match load (title, five Enters; `context/b630/shots.sh`, shots in
`context/b630/before/`): at a fixed Aspect Ratio (4:3 or 21:9 on a 16:9 window, 4:3 on a phone shape) the loading
screen's court filled the window only for the first frame or two. After that the bars showed the new match's edge
glow and the key legend's black strip, with the court and the pane only in the middle. The pause menu's dimmer and the
result screen's fade to black stayed inside the HUD's view, so the bars and the phone's safe-area strips stayed bright.

Cause: the mode switch despawns every entity the new mode didn't start with, the `Bleed` copies on the bleed camera's
root included. The loading cover itself is kept, but `hud_gamma::bleed`'s node → copy map still pointed at the
despawned copy, so it was never made again.

## What changed
- `hud_gamma.rs` `bleed`: a copy that's gone is dropped from the map and made again. A new copy is spawned already
  filled and visible, so it draws on its first frame (before, it was spawned hidden and filled the next frame). Test
  `bleed_copy_made_again_after_a_mode_switch`.
- `play/menu.rs` (pause): every quad slot carries a `Bleed`, on for the dimmer (and the top bar's pieces), ordered over
  the key legend's strip and under the loading screen. It covers the whole window now, safe-area strips and bars
  included. The same goes for the end-of-match confirm popup's dimmer (same `DIM` quad).
- `match_stats.rs`: the fade to black after Continue is a `Bleed` too.
- `widescreen::Bleed` derives `PartialEq` (`set_if_neq`).

## Checked
`--shot` runs through the real title → select → match load and pause (Escape), at 16:9 1280×720, 21:9 1680×720, a
4:3 and a 21:9 Aspect Ratio on 1280×720, phone 852×393 with `HST_SAFE=59,0,59,21` (Auto and 4:3) and portrait 393×852
with `HST_SAFE=0,59,0,34`: `context/b630/before|after/`, sheets `sheet-*.png`. After: the loading court covers the
window for the whole load at every shape. The pause dimmer darkens the bars and the safe-area strips (left-strip mean
grey 0.13→0.06 at 4:3 and 0.28→0.06 at 21:9 settings, 0.29→0.17 at the phone). Auto at 16:9 is unchanged (no strips).
Text, the pane, the pause box and the score stay inside the view.
Tests: `-p hst-base bleed_copy`, `-p hst menu::`, `-p hst-play match_stats`.

## Not verified / not 1:1
- The original is 4:3 only, so the outside area is our design; there's no original to compare against.
- The pause top bar's three pieces each touch only one edge of the picture. Under B629's rule (only what spans the
  picture runs into the bars) they don't extend into a fixed ratio's side bars, only under a notch at Auto. Left as is.
- The result screen's fade to black wasn't shot (it needs a finished match), and neither was the ask popup's dimmer.
  Both use the same mechanism, which the shots and the test above cover.
- No real phone. Phone shapes come only from window sizes and `HST_SAFE`.
- The court is random per run, so the before/after strip brightness numbers compare different courts. The dimming is
  plain to see by eye.
