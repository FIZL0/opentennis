# F5: the pause menu's board drawn by F1's widget

Spec: the concept page's 'Pause menu' tab (`context/endscreen-concept/end-screen-concept.html`).

## What I found

F2 had already put the live stats board on the pause screen and taken the old pause box away. Beside it, a menu box
in the concept's place, with one Quit that asks first. But F2 built the board with its own composer
(`pause_board::quads`: the score board, the bright set in play and the stacking all copied there). It didn't use F1's
`board::widget`, so the end screen and the pause screen could drift apart.

## What changed

- `match_stats/board.rs`: `widget(v, Kind)` takes which screen it is for.
  - `Kind::End` is the end screen's: the banner over the final board, then the stats board.
  - `Kind::Live` is the pause screen's: no banner. The score so far is on top, with the set in play on the bright
    sheet even at 0-0 (`live_board`, moved here from `pause_board`). The stats board is under it.
  - Both stack the same way (`stack`), from y 0 in the 640×448 board coordinates.
- `match_stats/pause_board.rs`: the board is now `board::widget(v, Kind::Live)`. Its own copy of the layout is gone.
- `match_stats.rs`: the end screen calls `widget(.., Kind::End)`.
- Nothing changed in `menu.rs`. It already draws `PauseBoard`'s pictures beside its box and fits the two together.
- What the player sees is the same as after F2, so there's no release note.

## Checks

- `tools/check.sh -p hst-play match_stats`: passed, including `board::score_board_sets` (end flavour, banner on top)
  and `pause_board::live_set` (live flavour, from y 0, no banner, the set in play bright).
- `tools/check.sh -p hst menu`: 326 passed (`quit_asks`, `save_and_quit`, `fits_any_screen`, `input`). This covers
  that the pause actions still work.
- Shots (`HST_PAUSE=6 tools/shot.sh … --play --shot-at 9`, `HST_SIZE` 1280×720, 960×720 and 1680×720) are in
  `context/f5/run1/`: `pause-169.png`, `pause-43.png`, `pause-219.png`, and `ask-169.png` (`HST_PAUSE_ASK=1`).
  - Side by side with the concept's Pause menu tab: menu box left, score board over the stats board and the
    Rally/Time bar right. At 4:3 the whole block scales uniformly; at 21:9 it stays centred.
  - The confirm shows "Quit the match?" and its three pills with the board still up.

## Not verified / not 1:1

- These screens are ours to design (PLAN.md: front-end and menus), so the concept is the reference, not the original.
  I compared them with the concept by eye only.
- I didn't press each pause choice again live. menu.rs is unchanged since F2's live checks (`context/f2/run4`,
  `run5`), and the unit tests cover the choices.
- Carried over from F2:
  - The in-world "COM" markers show faintly through the court dim.
  - The point score (15/30/40) isn't on the board, since the concept has none.
