# M180 — Base Disc setting

Settings → Gameplay → **Base Disc** (first row, before Rules): which release's disc the game runs from.

## What it does
- Shown only on desktop builds (`disc::CAN_SWITCH`: they restart themselves) with 2+ releases registered by M166
  (`releases::have`): `disc::bases()` lists them US, Japanese, European. Two releases: ✕ asks for the other one
  (no arrows, like Rendering). Three: ✕ opens the dropdown and picking one other than the running disc asks.
  ◀ ▶ do nothing. The popup ("Run from the Japanese disc?" / "The game restarts on it.", Switch / Cancel) is the
  shared `confirm` one, centred, drawn at 16:9, 4:3, 9:16, 21:9 in the test.
- Switch → `disc::switch_base(edition)` on a worker thread:
  - running from the imported pack (`disc.pack`): the target takes its place, the old pack is kept beside it as
    `disc-<serial>.pack` (e.g. `disc-SCUS-97610.pack`), so `discs::beside` still finds it and Rules / Voice Language /
    M174's EU-borrows-US keep working on the new base. A target that is a pack is renamed in; an image is imported
    first (`disc-next.pack`, via `.part`), so a failed import changes nothing. Switching back is a pair of renames.
  - running from a disc named on the command line: nothing moves; the restart names the other disc instead.
  - restart: same arguments (an `--import [iso]` dropped, the named disc replaced); exec on Unix, spawn + exit on
    Windows.
  - Row value shows `Switching…` while copying, `Couldn't switch` on failure with the reason in the side panel note.
- Rules setting untouched: `rules = disc` follows the new base's own release; a saved other release is laid as before.
- Release note added.

## Tests
- `hst-base` `disc::tests::base_disc_swaps_the_packs` (renames both ways, command-line disc, failed import leaves the
  pack), `base_disc_restart_args`, `base_disc_imports_an_image` (the real Japanese image imported into the pack's
  place reads as the Japanese release; the US pack kept beside).
- `hst` `options::tests::base_disc_asks_first` (popup drawn at four aspects, cancel, confirm hands off without a
  settings change); `rules_row` now allows Base Disc before Rules.

## Not verified / not 1:1
- Not tried end to end in a running game (no interactive session in an unattended run): the restart into the new
  pack was checked by its parts (file swap test, restart-argument test), not by watching the game come back up on
  the Japanese disc. Front-end only, so no PINE comparison applies.
- Phones and the browser don't offer it (they can't restart themselves / the browser has no folder of discs); a
  phone could swap the pack and ask the player to reopen the app — left out.
- The Settings legend (`main_menu/legend.rs`, not on this task's files) shows its generic hint on the Base Disc row,
  not a dedicated "Switch" word.
