# B631 — touch buttons too small in portrait

The phone apps' pad (`touch_pad::Layout`) only had the browser's landscape sizing: `u = min(7 % w, 12.5 % h)`,
which on an upright iPhone (393×852) is 27.5 px against 49 px on its side, with everything along the top edge.
The browser already had portrait rules (B112, B313, B531 in `web/touch.js`); the native layout now follows them:

- `Layout::upright` (any window taller than wide): `u = min(12 % w, 7 % h)` (47 px on that iPhone), so it's thumb-sized
  from the short side; the face diamond as in landscape; L1, Select, Start, R1 in one row 4.1 u above the safe area's
  bottom (over the face buttons); chat centred under Select/Start; the match's stick zone is the lower half's left 45 %;
  the menus' stick circle in the bottom-left corner as before.
- `row_bottom` (B610's score-panel drop) is 0 upright: the row isn't at the top any more.
- `web/touch.js` unchanged: its portrait rules already size from the short side (min(12vw, 7vh)).

Test: `upright_pad_is_thumb_sized_and_low` (size, row, chat, everything in the lower half, no top-row drop); B610's
`score_strip_clears_the_shoulders` still passes for the portrait sizes.

Shots (context/b631/run1/): `port.png` (450×900, `HST_TOUCH=1`, `HST_SAFE=0,40,0,30`) — stick, diamond and the
pill row are full size in the bottom part of the screen; `land.png` (1280×720) unchanged.

## Not verified / not 1:1
- Not tried on a phone: desktop `--shot`s with `HST_TOUCH=1` only (no device in an unattended run).
- In-match portrait framing (whether the court picture leaves the bottom free) is B632's (split view per team); this
  task only moves the pad there. Front-end/touch layout isn't part of the original, so no 1:1 check applies.
