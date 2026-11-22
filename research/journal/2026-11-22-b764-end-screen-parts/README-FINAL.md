# B764: end screen (F1) redone with the game's own parts

One screen, flat dim, all of it fitted at one scale to the visible part of the screen (any aspect ratio):

- **Left: the pause menu's box** (`next::menu`): its 320-wide brown box, 288-wide pills 40 apart, the pause's colours
  (ink rims, white rings, the chosen pill bright, the rest dim), its words at the same size, and its 3D pointing hand
  at the pause's place. The one-line note sits in the box under the pills. ↑↓ choose (←→ still work), ✕ picks, a
  click or tap on a pill picks it (hit rects from the last draw, `next::Hits`).
- **Right: the board widget** (`board::widget`, one widget the pause menu can reuse): the disc's "Game, Set, Match!"
  banner (the `i_gameset_00` model's own texture, drawn flat), the original's final score board
  (`popups::final_board`: its frame, stripes, green games digits and the big red set digits), then the stats board.
- **Plates** (`plate.rs`): on both boards, the pause score board's plates: face in the board's white ring, the
  player's name (or 1P/COM) in the plate's team colour where the slot label was, the character's pill under the face.
- **Digits**: every stats value, the rally and the match time use the board's orange outlined digits
  (`result_gameset02`); "Stats" is white (the sheet's own), no olive or new palette left in the end screen's code.
- The pointer: hst's cursor setup now also inserts `next::Pointer` (the pause hand's picture); without it the hand
  is just not drawn.

Checked: `cargo build --release`; `tools/check.sh -p hst-play match_stats` (20) and `popups` (17) pass. Shots in
`context/b764/run2/` (`end-*.png` with `HST_STATS=4 --shot-at 14`, `pause-*.png` with `HST_PAUSE=6 --shot-at 8`,
both `--play --pads -,-,-,- --stage 10`) at 1280×720, 960×720 and 1680×720: box, pills, hand, plates and digits match
the pause screen's. `context/b764/run3/`: S, S, W move the chosen pill with the note and hand, Enter picks and the
screen fades out.

## Not verified / not 1:1

- The character names list is copied into hst-play (`plate.rs`): the select's list lives in the front-end crate,
  which hst-play cannot name.
- One line outside the task's files: `crates/hst/src/play/main_menu/cursor.rs` inserts `next::Pointer`, since the
  pause hand's image is made in the front-end crate.
- The stats header is 16 px taller than the original's (room for ring, name and pill); its stripes and the frame's
  header part are stretched to it.
- Stats digits are the board's 32-px glyphs drawn 20 px in the stats sheet's 16-px cells, so neighbours touch slightly.
- The score board is the original's final board (green games, red sets digits), not the pause board's orange ones:
  it is the game's own end-of-match board.
- The online menu (Rematch / Back to Lobby) is covered by the fit test only, not shot.
