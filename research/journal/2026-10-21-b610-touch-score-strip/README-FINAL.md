# B610 — touch pad shoulder row over the score strip

With the touch pad on, the L1/R1/Select/Start row (and the chat button) covered the score panel's top Sets/Games
strip: R1 over "Games" on the right, and L1 over "Sets" when the top team's strip is on the left.

## Fix
The buttons stay where `web/touch.js` puts them (top row, where a pad's shoulders are). Instead the score panel's
top half (strip, pills, faces, points, ring, doubles banners, the name plates that replace slot labels) drops
just below the row when the pad is on:
- `touch_pad::top_row(size)`: how far down the view the row reaches (percent of the view; 0 without the pad,
  safe-area insets and a letterboxed view taken into account).
- `panel::touch_drop` / `drop_below`: drop = row + 1 % − the strip's own top (y 14 of 448), never negative.
  `panel::vplace_clear` adds it to the top half's quads; `name_tags` uses it for the panel's plates too.
  Without the pad nothing moves.

## Checks
- Unit test `touch_pad::tests::score_strip_clears_the_shoulders`: both corners' strips (background and text)
  clear every shoulder/chat button and stay above the stick zone at 1280×720, 852×393 with iPhone insets,
  1024×768, 450×900 and 393×852 with portrait insets.
- Shots (context/b610/run1/): `land.png` (1280×720, `HST_TOUCH=1`), `port.png` (450×900) — the strip sits below
  R1 in both; `land-off.png` without the pad is the original layout.

## Not verified / not 1:1
- With the touch pad on the top half of the score panel sits lower than the original's (it moves away from the
  screen's top edge by about one button height). That is an addition for touch play: with the pad off it is
  unchanged.
- Not checked on a phone; desktop shots with `HST_TOUCH=1` only.
- Clippy gate: done.sh failed on a pre-existing lint from main (reaction_voices.rs, needless borrow of the game); fixed in place (one character) so the gate passes.
