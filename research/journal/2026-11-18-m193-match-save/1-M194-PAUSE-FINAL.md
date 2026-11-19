# M194 Pause menu: Save and Quit

Concept: CONCEPT.md *Pause (M194)*.

## What it does

- `crates/hst/src/play/menu.rs`: a "Save and Quit" row (`SAVE`, label sheet `SAVE_ROW`) before "Return to the Menu",
  in a local match and the online host's (`can_save`: `Netplay.me == "host"`); a guest's pause is as before. Not in
  the browser (`Saves` there has no storage until M198). The dialog's look is untouched: same box, pills, hand, bar,
  venue, board.
- Five rows over a three-set board don't fit between the top bar's rim and the score frame: `rules_tab::Fit::new` now
  does the no-tab case too (a box that already fits is unchanged, one that starts above the rim moves down to it and
  shrinks to clear the frame). With a rules tab the dialog behaves as before for every existing row count (the tab
  test runs over all four row sets now).
- ✕ on it: `Menu.saving` → the exclusive `save_match` system (`save_game::capture(.., Local|Online, no flags)` +
  `Saves::user().write`) → `Menu.saved = Some(ok)` and the box's lines in `pause.rs`'s `SavedBox`. Nothing moves
  while it saves (one frame).
- `main_menu/pause.rs`: the "Match saved" box (`saved_layout`, root at z 11 over the pause screen, kept out of the
  pause's HUD hiding), in the notice's / dialog's style: brown box, white title (22 px), the score line and the two
  text lines, one peach OK pill with the 3D hand. The pause screen draws only its background dimmed under it
  (as for the confirm). OK (✕, click/tap on it, or ○/Start) fades out over 60 ticks and leaves the match the way
  "Return to the Menu" does (back to the main menu). If the save fails the box says "Not saved" and why; OK goes back
  to the dialog.
- Score line (`score_line`): the left team by character (a mod by its own name; doubles "Carol & Jun"), each played
  set's games then the set in play from the save's `Summary`, and the point (0/15/30/40, 40-40 at deuce, Ad-40 /
  40-Ad, tiebreak counts): "Carol 6-4 4-2, 30-15."
- "Leave the match?" gets the note "It won't be saved. Save and Quit keeps it." under the question when the dialog
  has Save and Quit (the box grows 22 px upward; Yes/No pills stay where they were).
- Peers can't close the pause from under a save in progress or the box (`set_paused`).
- `HST_PAUSE_SAVE=1` (with `HST_PAUSE`) saves at once, for `--shot`.
- `save_game::Saves::user()` now uses `store::file("saves")` (the folder settings use, honouring `HST_DATA`) instead
  of `disc::data_dir()` directly, which ignored `HST_DATA`: the first `--shot` here wrote a save into the real
  `~/.local/share/hst/saves` (removed again). Same folder as before for a normal desktop start.

## Checked

Tests (`hst`): `menu::save_and_quit_row` (row order with and without Controllers, dialog between rim and frame for
1-3 sets, ✕ saves, OK fades / goes back when not saved, host-only), `menu::confirm_note`, `menu::saved_score_line`,
plus the existing pause tests (`art_order`, `rules_tab_clears_the_dialog` over the save rows too, `controllers_row`,
`confirm_return`, `input`).

Shots (`tools/shot.sh`, `HST_DATA` in the scratchpad, `context/m194/run2/`): `dialog.png` (five rows, 16:9),
`ask.png` (confirm with its note), `saved.png` (the box; the save file landed in `HST_DATA/saves`),
`sets3-portrait.png` / `saved-portrait.png` (three sets to win, 720×1100 window: rows squeezed between the bar and
the frame, box centred, scaled uniformly).

## Not verified / not 1:1

- Not against the original: the original has no mid-match save; the menu is ours (PLAN: front-end/menus are ours).
- OK → main menu not driven live in a shot run (it's the existing "Return to the Menu" fade-and-exit path; the
  state change is unit-tested).
- Online host's Save and Quit not tried in a live online match (no second peer in this run); it saves `Mode::Online`
  and quits like the host's Return to the Menu. Resuming either kind is M195/M197.
- `Setup::flags` saved empty: the hst app doesn't keep the menu's match flags in the world yet; resume (M195) should
  record them at match start (no harm meanwhile: saves from another build are refused anyway).
- The box tells players to continue from the main menu, which only works once M195 (Continue) lands.
- The confirm's note is a word sheet shrunk to fit its 256-px row, drawn at 288×36: smaller than the title, readable
  at 1280×720.
