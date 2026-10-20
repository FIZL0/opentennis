# M135 — Update mods (FINAL)

- An installed store mod is tagged **Update** only when the store's copy has another zip *and* a newer `mod.json`
  version than the installed copy (`View::vers`, read beside `have`); a re-zip at the same version stays Installed.
  This matches what Download all already skipped (B255). A folder with no version read falls back to the zip alone.
- Per-mod Update: the existing hovered-card button ("Update - N MB") and ✕ on the tile now follow that rule.
- **Update all**: a green pill in the Mods footer (Installed and Browse) while any update waits; queues every update
  of every kind not already queued/downloading (`Menu::update_all`, Downloads at Once respected).
- Home screen: a card under Extras ("N MOD UPDATES / Update all", "Updating..." once all are queued) while updates
  wait; ✕ runs Update all; legend says Update all. The pills' pitch is computed from the cards shown (unchanged for
  existing combos; with Rematch + new version + mod updates all up the pills shorten so the cards clear Quit).
- `prime` now reads every kind's installs at startup, and the main menu reads the store listing whenever any store
  mod is installed (not only on wide windows / the Mods row), so the card can appear.
- Shot sample: `HST_MENU=main-modupdates`. Shots in `context/m135/` (16:9, 4:3 home; Browse footer).
- Tests: `update_all_queues_newer_versions`, `main_screen_update_all_card`, `home::cards_clear_quit`.

## Not verified / not 1:1
- Front-end only (ours to design); no original to compare.
- The triple-card case (Rematch + new game version + mod updates) was checked by the layout test, not by a shot.
- No live store update was installed end to end (needs a store mod with a bumped version); the queue is the
  existing install path, covered by its tests.
