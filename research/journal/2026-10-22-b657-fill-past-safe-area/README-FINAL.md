# B657 — black and score panel fill past the safe area

## What was wrong (baseline: `context/b657/before/`)
- **Leaving a match**: the pause dimmer and the "Return to the Menu" fade were drawn twice inside the view (the node
  itself plus its B630 `Bleed` copy, which covered the view too) but once over the safe area's strips and the bars,
  so the strips lagged visibly behind the rest while the screen went black. The key legend's strip (z 30) also stayed
  lit over the black until the mode switch. The result screen's fade (after Continue) had the same two problems.
- **Score panel**: the Sets/Games white strips and the doubles A Team / B Team pills stopped at the safe area's edge.
- Found on the way: whenever the HUD's view is inset (phone safe area, a fixed Aspect Ratio, a Safe Area margin) the
  bleed camera draws first into the HUD image, so Bevy 0.19 gives the HUD camera, now second on that target,
  straight alpha blending for its output, but its picture is premultiplied. Every see-through HUD piece came out at
  about half strength: the panel's 35 % white strip read 0x5E over a 0x40 court instead of 0x80, as on a plain desktop
  window. The first extended strips looked twice as bright as their inside part because of this.

## What changed
- `widescreen.rs`: `BleedOutside` (marker next to a `Bleed`): its copy is only the part of the bled rect outside the
  view, as up to four pieces (`outside_pieces`; left/right full height, top/bottom between), each showing its share
  of the image. Test `outside_pieces_frame_the_view`.
- `hud_gamma.rs`: `bleed_outside` draws those pieces (`bleed` skips `BleedOutside` nodes). The pieces are snapped to the
  HUD camera's actual viewport (whole pixels), because a first version left a 1 px undimmed seam at the view's edge.
  The HUD camera blends with `PREMULTIPLIED_ALPHA_BLENDING` while the bleed camera is on (alone: unchanged, replaces).
  Test `bleed_outside_copies_only_the_outside`.
- `menu.rs` (pause): the quad slots are `BleedOutside`; while fading out (`quitting`) the menu's root goes to z 31,
  over the key legend.
- `match_stats.rs`: the fade after Continue is `BleedOutside` and z 31.
- `panel.rs`: slots carry `Bleed` + `BleedOutside`, on for a Strip or TeamPill quad that crosses the 640×448 screen's
  left or right edge (`runs_off`): the flat part of each strip and the pill bodies (rim, ring, colour), not the
  rounded ends or any text. It also works while the panel slides in. Test `edge_pieces_run_off`.
- `mode.rs`: no change needed. The loading cover is already a full-window `Bleed`. When leaving a match the last frame
  it zooms is already all black.

## Checked
`context/b657/shots.sh RUN BIN`: a doubles `--play` (all CPU) for the panel, and a title → match → pause → Return to the
Menu run for the fade, at 4:3 1024×768 (`HST_SAFE=40,30,40,30`), 16:9 1280×720 and 21:9 1680×720 (`60,24,60,24`),
fixed 4:3 on 1280×720, phone 852×393 (`59,0,59,21`) and portrait 393×852 (`0,59,0,34`). Before = main's binary,
after = this branch: `context/b657/before|after/`. **`context/b657/side-by-side.png`** has each shape's panel and
mid-fade before | after, and a zoomed row of the phone's two panels.
- The panel's strips and pills reach the window's edge at every inset shape, with no seam and the same strength
  inside and out (phone: 0x81 outside, 0x80 inside). Faces, words and digits stay inside.
- Fade: the strips darken with the inside (phone column means at the view edge, inside vs strip: 0x11 vs 0x13 mid-fade).
  Every shape is fully black (mean 0.000) 1.1 s in; before, the legend was still lit (0.002–0.003).
- The pause screen's dimmer is now as dark over the view as on an uninset window (before: alpha ~0.75 inside from
  the double draw, 0.5 at the strips).
Tests: `-p hst-base bleed`, `-p hst-base outside_pieces`, `-p hst-play panel::`, `-p hst menu::`.

## Not verified / not 1:1
- The original is 4:3 only with no safe area, so what fills past the safe area is our design; nothing to compare with.
- At a fixed Aspect Ratio the panel's strips and pills stop at the picture's edge, not in the bars beside it (B629's
  rule: only what spans the picture runs into the bars). With no safe-area inset there's nothing new at those shapes.
  The fades and dimmers do cover the bars.
- The result screen's fade (after Continue) wasn't shot: it needs a finished match. It uses the same mechanism as
  the pause fade, which was shot.
- Shot runs pick a random court each time, so the before/after brightness numbers compare different courts. The
  seam and strength checks compare pixels within one shot.
- The premultiplied blend fix changes how every see-through HUD piece looks whenever the view is inset. It now
  matches the uninset look, but I only looked at the panel, pause and fade, not at every HUD screen.
- No real phone. Phone shapes come only from window sizes and `HST_SAFE`.
