# F2: the end screen's stats board on the pause menu, one Quit with a confirm

Spec: the concept page's 'Pause menu' tab (`context/endscreen-concept/end-screen-concept.html`).

## What changed

- `hst-play` `match_stats/pause_board.rs` (new) builds the board each frame while the pause menu says it is open
  (`PauseBoard.open`). It puts the end screen's score board on top: every set played, plus the set in play on the
  bright sheet at full strength, even at 0-0 (`board::score_board_of`, split out of `score_board`). Under it go the
  stats board and the Longest Rally / Match Time bar for the match so far (`MatchStats::finish` with the current
  score; the leader is ahead on sets, then on games). There is no banner. It lays the board out as resolved quads
  (image, rect, rect on the 640×448 screen, colour), so the front-end crate can draw them without the end screen's
  private art types. The `Build` SystemSet orders it between the pause menu's `step` and its `draw`.
  `plate::refresh` now also sets the plate words while the pause board is open.
- `hst` `menu.rs` keeps the original's top bar (Paused, rules, court, umpire). The old bottom score board is gone
  from the pause screen. It stays as `score_board`, because the Continue cards (`saved_board.rs`) still draw it.
  - The dialog is a box in its own units (`dialog`), and the board sits to its right with their middles level.
  - `Place` fits the block (the hand's room, the box, the rules tab under the box, the board) at one scale between
    the top bar and the legend strip, never larger than 1:1, centred.
  - Clicks and taps hit-test the drawn pill rects (`Hits`).
  - The court dim is 104 instead of 64, because the concept shows the court darker under the board.
- "Save and Quit" and "Return to the Menu" are now one "Quit" row. Choosing it asks "Quit the match?" in the box's
  place. The board stays up and there is no extra dim. The choices:
  - **Save and Quit**: saves and shows the Match saved box, as before.
  - **Quit without Saving**: fades out to the menu.
  - **Cancel**: back to the pause menu with Quit still selected. ○, Back and Start do the same.
  - Up and down wrap around.
  - Under the pills is a note for the selected choice: "Keep this match to continue later" / "Drop this match and go
    to the menu" / "Back to the pause menu".
  - The legend says Cancel for Back and Start.
- The B68 rule ("ask only once a point was scored") is gone: Quit always asks, as the concept does.
- The Offbeat rules tab now sits under the dialog, with the full rim on all four sides, instead of on the old score
  frame. `rules_tab.rs` lost its frame placement and fit code.
- `HST_PAUSE_ASK=1` now opens the confirm as if chosen from Quit, so Cancel lands on Quit.

## Checks

- Tests:
  - `tools/check.sh -p hst menu`: 326 passed. New: `fits_any_screen` (16:9, 4:3, 21:9; uniform scale ≤ 1; inside
    the visible area; box GAP left of the board; rules tab clear of the pills) and `quit_asks` (all three choices,
    ○/Start, wrap, no-save variant, the board up during the confirm). `save_and_quit`, `controllers_row` and `input`
    were rewritten for the new layout.
  - `tools/check.sh -p hst-play match_stats`: 306 passed, including the new `pause_board::live_set` and `leaders`.
- Shots (`tools/shot.sh … --play --shot-at 9`, `HST_PAUSE=6`) are in `context/f2/run3/`:
  - `pause-169.png`, `pause-43.png`, `pause-219.png`, `ask-169.png`.
  - Side by side with the concept's menu and quit pictures: same arrangement (box left, score board over the stats
    board and the Rally/Time bar right, middles level), same three pills and note, no extra dim.
  - At 4:3 the whole block shrinks uniformly. At 21:9 it stays centred with room either side.
- Each choice, live (`HST_PAUSE_ASK=1` + `HST_KEYS`) in `context/f2/run4/` and `run5/`:
  - ✕ on Save and Quit: the "Match saved" box.
  - Down then ✕: fades out to the title.
  - Down, down, ✕: back to the pause menu on Quit.
  - KeyK (Back): back to the pause menu on Quit.

## Not verified / not 1:1

- Backspace isn't among the keys `HST_KEYS` can press. Back was checked with K, its other binding. Start's Cancel is
  only covered by the unit test.
- An online guest can't save (M194), so its confirm shows only Quit without Saving and Cancel. That is covered by the
  unit test, not checked live.
- The "Match saved" box (`main_menu/pause.rs`) still sits where it always did, over the left of the board, which is
  dimmed under it. The concept doesn't show this step.
- The match rating reads 51/49 before anything happens. That is the end screen's own formula on empty counts.
- The pause screen no longer shows the point score (15/30/40), since the concept's score board has none. The games
  in the set in play are shown.
- The in-world "COM" player markers show faintly through the court dim, near the box's top and in the top bar. They
  are the match HUD's, drawn under the dim. Not changed here.
- The court dim (104) and the box/board gap (24 px) were set by eye against the concept, not measured.
