# F1 — End screen in the board style, one screen

Spec: the concept page's "End screen" tab (`context/endscreen-concept/end-screen-concept.html`).

## What changed

- `match_stats/board.rs` (new): the stats board as one widget, `board::widget(&View)`. It draws the final set-score
  board on top, in the stats board's own frame (a 9-slice of the frame sheet). Each side's players sit on their
  stripes with their faces and slot labels (one per side in singles, one per row in doubles), with a column per set
  played. The set's winner is drawn in gold and bigger, the loser in grey. Under that it draws the stats board
  (`layout`) and the Longest Rally / Match Time bar. Everything is in the stats board's 640×448 coordinates, with the
  score board above y 72. The pause menu can draw the same widget and fit it however it likes.
- `match_stats.rs`:
  - One screen, no pages: the two-page slide, the "seen" state, the move-aside shrink, the scale pulse and the
    separate final-board page are gone.
  - A flat dim node (#3d4856 at 85 %) covers the whole window, bars included, while the screen is open.
  - The 3D "Game, Set, Match!" banner shows only during the ceremony. The end screen has its own gold word label.
  - "Stats" is one solid quad in the board's yellow instead of the original's fade to grey.
  - The cells' dark backing is at alpha 112 instead of 64, because they sit over the dim.
  - Input: ←→ (↑↓ and the stick too) choose, ✕ picks from tick 120, a tap on a pill picks it. The legend shows
    Choose / OK.
- `next.rs`:
  - The pills are a centred row under the bar in the board's colours: dark rim, olive fill, and the chosen one yellow
    in a white ring.
  - The chosen pill's note sits under the row.
  - `fit` scales the whole screen uniformly (k ≈ 0.85, from 16 to 452 in board coordinates) into the 4:3 view and
    centres it. `pill_at` inverts the fit for taps.
- `popups.rs`: `final_board` was used only by the old score page, so it is deleted. `Final` stays (its test still
  builds one).

## Checked

- Shots at 16:9, 4:3 and 21:9 (`HST_SIZE`, with `HST_STATS=14 HST_STATS_PAGE=1 --stage 10 --play --pads -,-,-,-`) in
  `context/f1/run1/`. They match the concept's layout: the title, the score board, the full-size board, the bar, the
  pill row and the note. They keep their shape at every aspect, and the dim covers the side bars.
- Pill action (`context/f1/run2/`): → moves the choice to New Match and its note changes to "Change picks, court or
  rules". ✕ then starts the fade to black.
- `tools/check.sh -p hst-play match_stats` and `popups` pass. New tests:
  - `board::score_board_sets`: the set digits, gold for the winner, the board centred and within its extent.
  - `next::row_fits_under_the_board`: the row and banner fit inside the screen, and a tap on each pill finds it.
  - `choose`
  - A match ended mid-set also shows the column for the set it was in.
  - `doubles_fits`: the whole screen, five sets and online pills, fits the 400-slot pool.

## Not verified / not 1:1

- **Character names on the score board.** The concept names the characters (Carol, Jun, …). Ours shows the slot
  label (1P / COM); a named player's label becomes their name plate through the name tags. Character names live in
  the `hst` crate, not in `hst-play`.
- **Name plates.** Only four plates exist per HUD. With the score board and the stats board both carrying labels,
  a named player has two labels, and only the first four labels get plates. The widget lists the stats board first,
  so its headers get them. Not checked with named players (F3).
- **"COM Lv5" headers.** The stats headers still show the original's rank art (Hopeful / Lv 5) and slot labels, not
  the concept's "COM Lv5" text.
- **The full return trip.** After the fade, I did not see the app go back to the menu or what the menu does next.
  That path (`next::take`, AppExit) is unchanged code, and the shot run with `--shot` kept running until I stopped it.
- **Pause-menu reuse.** The widget exists (`board::widget`), but the pause menu doesn't draw it yet, and the widget
  is still private to `match_stats` (F5).
- **Online (Back to Lobby) row and real finished matches.** Only checked in tests: the online row, and a match that
  actually finished with several sets. The shots force the end with `HST_STATS`, so they show 0–0 in one column.
- **Taps.** Mouse and touch picks are tested through `pill_at` only, not by hand.
- **Matching the concept.** The colours (dim, olive, gold, grey digits), the pill widths and the face size in the
  doubles score rows are by eye.
