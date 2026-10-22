# B648 — Result board: the left plate's "COM" spills out of its slot

## Cause
The point boards and the game/set result board share one pool of slot labels (`name_tags::BOARDS`), and the labels
were taken with one room: 32 disc px past a left-half label's cell. On a point board that room is real (the rank
row's width is clear on the pill). On the result board, though, the left plate's 40-px label slot ends at its
stripe's bar (label 208..248 on the game board, 144..184 on the set board). So "COM", 45 px set in our plate tag,
was drawn whole from 212 to ~262, past the bar. 1P..4P are narrower and stayed inside. The right plate was always
fine: its label is right-aligned with no extra room.

This is all in 640×448 units under the 4:3 root, so the window size and aspect don't matter: the spill is the
same at every size. Menu Size doesn't touch it either (the match HUD doesn't use it).

## Fix
`popups.rs` `label_room`: no extra room on the game/set board, so a label there is condensed (down to 0.7) and then
shrunk to fit its 40-px slot, like the right plate's. The point boards keep their 32 px.

## Verified
- `name_tags::tests::board_labels_stay_in_their_slots`: every slot's label (1P..4P, COM) at all four result board
  slots (left/right on the game and set boards) stays inside its 40-px slot. With the old 32 px it fails with
  "COM at 208: 212..261.9", which matches the user's photo.
- `--shot` at 1280×800 with the Steam Deck preset's settings (`--vsync --shadows high --msaa 4 --scale 100`),
  doubles with 1P (keyboard, K pressed every second so it serves) and COM on team A, CPUs on team B, shots every
  0.5 s around the first game's end: on the game board ("Game Receiver"), 1P, COM | COM, COM all sit inside their
  stripes. The left COM is condensed and ends before the centre bar (frame `context/b648/run4/s0925.png`).

## Not verified / not 1:1
- The labels are our own tag (B541), not the disc's art, so there is nothing to match 1:1. The original's COM
  art fits the 40-px cell, and ours now does too.
- I didn't take a 20:9 phone shot of the board: the 1600×720 all-CPU run's first game was still going at deuce
  when the shots stopped. The placement is in 640×448 units under the 4:3 root, so it's the same at every aspect,
  and the test covers it.
- I didn't try a non-default Menu Size (the user wondered whether the Deck's setting was the cause). The match HUD
  doesn't use it (`widescreen::MENU_SCALE`), and the fault was in the label room at any size.
- The set board (144/456 slots) is covered only by the test, not by a shot.
