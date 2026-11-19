# M196 — Match end: what next beside the stats

Ours (concept tab "Match end"; front-end/presentation, not 1:1 with the original). The concept file
(`concept.html`) wasn't in this worktree or the main checkout (the folder there is empty), so this follows the task
line's words.

## What it does

- `match_stats/next.rs` (new): once the stats page has been seen and stands still, it moves aside over 20 ticks
  (eased; shrunk to 0.66, right edge 20 px in from the 4:3 screen's) and a box of pills slides/fades in on the left in
  the pause dialog's style (our box and pill layers, the same rim/fill/chosen colours, words at the dialog's 20 px):
  Rematch / New Match / Return to the Menu, online Rematch / Back to Lobby, and a one-line note under them for the
  chosen pill. ↑↓ (W/S, d-pad, stick) choose (sound 0), ✕/Enter picks from tick 120 as Continue did (sound 2, the same
  60-tick fade and `AppExit`), a click or tap on a pill picks it. ←→ still slide to the board; the pills go out and
  come back with the stats page. Legend: Page, Choose, OK.
- The pick goes to the front-end through `next::take()`: `home.rs` `Menu::after_result` (called once a frame from the
  menu's `step`) — Rematch: `Launch` with the kept setup (same roll, so a Random court/umpire stays what was drawn);
  New Match: the select with every pick kept, the humans' cards un-readied (computer cards stay ready), court and rules
  on the confirm after it; Return to the Menu: the main screen on Local as before. Online (`online.rs`): a guest's
  Rematch says Ready in the lobby; the host's waits in the lobby until every seat is ready (no open seat, no countdown
  or wait running), then starts as Start does; leaving the lobby drops it. Back to Lobby is where an online match came
  back already.
- `word_label::label_w`: `label` on a wider sheet (the notes are 448 px sheets).
- Presentation only: nothing reads or writes the sim, its RNG or the match clock (the screen already holds the fixed
  clock).

## Checked

- `tools/shot.sh` runs (context/m196/run2–6): the pills and note at 16:9 (doubles), 4:3 and 21:9; the stats page fits
  beside them in each. `HST_MENU=rematch` round trips: New Match → the select with 1P's pick kept (run3); Rematch →
  a fresh match on the same court with the same umpire (run5).
- Tests: `next::tests::panel_fits_left_of_the_page`, `page_moves_aside` (hst-play),
  `home::tests::result_screen_choice` (hst).

## Not verified / not 1:1

- The concept mockup wasn't available (missing file), so layout, note wording and animation are from the task text,
  not the mockup.
- Online Rematch / Back to Lobby were not run with two machines (no second peer in an unattended run); the host's
  auto-start and the guest's Ready are covered only by reading the lobby code paths.
- No hand cursor beside the chosen pill (the pause dialog has one); the bright pill marks it.
- While the pills are in, a click or tap off them does nothing (the left quarter is under the box, so it no longer
  slides back to the board by tap; ←/A or the d-pad still do).
