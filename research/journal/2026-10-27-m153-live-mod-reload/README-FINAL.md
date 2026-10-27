# M153 — reload mods live instead of restarting

## What changed
- `mods_screen/live.rs` (new): Reload on the Mods page (pill, footer button, "Reload now" prompt) starts a live reload
  instead of the restart / Close-game popup. It lists the mods folder, grades one mod per frame (tparam, power gain,
  style, grades: the same steps as the startup read), and an overlay ("Reloading mods", "Reading mods n of N", bar)
  takes every press meanwhile. Then it waits until no mod picture is loading or drawing and swaps the roster in.
- Swap (`Menu::swap_roster`): old own mods are mapped to new ones by folder, else by id. Player picks of a mod follow
  it (costume reset if out of range); a gone mod's pick falls back to the Random card (keeps swap/handicap). Umpire and
  court picks follow their mod or fall back to Random. `last` remapped; umpire/court picture and voice caches are
  cleared so they re-read. Favourites and last picks are already keyed by folder, so they need nothing.
- Pictures (`previews.rs`): `reroster` drops every mod key from the loader, icons and baker, re-checks costumes,
  re-stamps the cache, and queues the mods' pictures with the uncached costume-0 ones first. Those are on
  `baker.reload`, which lets the baker draw them on any screen, **in the background** (user, mid-task: "for drawing
  pictures it should be able to be done in the background"). Each comes off the list when drawn, failed or started.
  While the reload is being read, `baker.hold` stops new drawing so the indices can't shift under a job.
- `preload::forget_mods()` clears the match's early-load cache of mod models.
- Umpire voice-preview cache is keyed by (umpire, mod folder), so renumbering doesn't play the wrong bank.
- Only offered off the select screens, lobbies, matches and pause (same as the transfer rule); otherwise it buzzes.
- `HST_MENU=reloading` shows the overlay on sample data.

## Verified
- Tests: `a_reload_keeps_picks_on_their_mods`, `a_removed_mods_picks_fall_back`, `the_overlay_counts_up`,
  `reload_is_in_place`; `tools/check.sh -p hst main_menu`: 284 passed.
- Live (release build, `HST_MENU=reloadmods`, Enter at 5 s, 81 mods in the user's folder): the overlay showed
  "Reading mods 20 of 81" with its bar; by 12 s the overlay was gone and the menu was usable, with pictures drawing
  in the background (screenshots in `context/m153/`).

## Not verified / not 1:1
- No end-to-end test that installs a real mod folder and reads it from disc; the pick re-mapping is tested on built
  rosters.
- Texture packs aren't reloaded live (outside the roster); they still apply from the next start.
- Phone and web builds not run; the code path is the same (the restart popup is gone there too).
- The menu is ours, so there's nothing to match 1:1.
