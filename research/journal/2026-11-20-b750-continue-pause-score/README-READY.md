# B750 — Continue cards with the pause menu's score board (concept round 1)

**Branch:** `concept/continue-pause-score` (pushed). Two commits on top of main: the cards, then the spec-review fixes.
**Shots (A/B):** https://claude.ai/artifact/NDdNThsSWAx5GnEbGtbUim (private to the owner). Local copies are in
`context/b750/`: `run1/before-{169,tall}.png` (B747) and `run4/c-{169,tall,43-dead,219}.png` (this branch).

## What's on the branch

- `crates/hst/src/play/menu/saved_board.rs` (new): runs the pause screen's own `layout` with a save's score
  (`save_game::Summary`, `saved: true`, so no dialog or court/umpire over it). It returns the board's pieces
  (frame, plates, the set games' sheets and the points) relative to the board, plus each player's face frame,
  seat-tag cell and character-pill place. The card draws the same disc art, so it reads like the pause menu's board.
- `main_menu.rs`, `read_sheets`: loads the five INPANE sheets the board uses (`saved_board::SHEETS`). This file
  isn't on the task's file list, but the change is additive (one loop) and the board needs it.
- `continue_screen.rs`:
  - Cards are landscape (240 wide). A band at the top holds the mode text and the Ready / Mods changed chip.
  - The title names every player: "Ashley & Momoko vs Kent & Jun". It goes on two lines when one won't hold it.
  - Under the title: the pause board, with faces, 1P..4P/COM tags and navy name pills. Doubles show all four.
  - COURT / SAVED fact boxes run across the card's width. Text shrinks as a whole to fit and is never cut.
  - Card height is set by the tallest board on the screen (doubles at best of 5 is taller). Shorter boards sit
    in the middle of the room.
  - The grid picks the column count that draws the cards biggest (up to ×2). Tall views add the room above and
    below the 4:3 frame. Up to 700 design units wide (4:3, tall), the foot bar stacks over Resume. Its Length cell
    is wider, so "Best of 3, 6 games" isn't cut.
- RELEASE_NOTES line.

## How to try it

    git switch concept/continue-pause-score
    HST_MENU=continue-screen tools/shot.sh out.png            # 16:9
    HST_SIZE='450 900' HST_MENU=continue-screen tools/shot.sh tall.png
    HST_MENU=continue-dead ...                                  # a save that can't resume

Or play a match, Save and Quit, and open Continue.

## Spec review

- Fixed:
  - Seat tags now follow the seat (`humans` is per seat, 1P..4P), not a count of the human seats.
  - An extra Slot quad can no longer overwrite the last tag.
  - The grid test covers doubles' best-of-5 card height and a tall room, and checks the ×2 cap.
- Kept, flagged as outside the strict file list or the cards: the `main_menu.rs` sheet loader (needed), and the
  foot-bar narrow threshold and cell shares (otherwise the task's "any aspect ratio" left court/length clipped
  at 4:3 and tall).

## Not verified / not 1:1

- The menu is ours (PLAN: front-end not 1:1). I made no PCSX2 side-by-side: the pause board only shows mid-match,
  and the card is a scaled re-use of its layout, not a capture match.
- Name pills are 17 px tall, against the pause's 11 px, so they read at card size. Seat tags and pills are our
  text in the pause board's places, not the disc's slot art.
- At 16:9 the pointing hand is hidden when the cards fill the width. The gold ring marks the chosen card.
- Text shrinks to fit with no floor. A very long mod name could get small, but it's never cut.
- I haven't tried it on a phone or the web build, only the desktop `--shot` sizes above.

## What's next

The user picks: land as is, adjust (pill size, card shape, foot bar), or drop. If landing: merge the concept branch
and tick B750.
