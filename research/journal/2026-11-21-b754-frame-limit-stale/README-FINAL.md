# B754 — Frame Limit shows 60 while the game runs uncapped

## Which side was wrong

The menu's copy. Both the pause menu's Settings (on opening) and the main menu (coming back from a match) read
settings.txt through `Settings::parse`, which ran P137's `Graphics::upgrade()` on every read. On a phone (iOS/Android,
`PHONE_OS`) that turns any saved `frame_limit = uncapped` with a High/Default picture into 60: it was meant only for
files written before P137, when Uncapped was the phones' default. So:

1. Frame Limit → Uncapped in the main menu: saved as `uncapped`, the match started with `--fps 0` (what the FPS
   overlay showed: no limit).
2. Pause menu → Settings: opened with settings.txt parsed → 60; any change saved 60 to settings.txt.
3. Back in the main menu: parsed again → Settings showed 60 fps.

The match's own flags never pass through `upgrade()`, hence the disagreement. On desktop `upgrade()` never touches
the frame limit, so this was phone-only.

## Fix

`Graphics::upgrade_saved(text)` (graphics.rs): the phone 60 cap applies only to a file without a `match_speed` line.
That line (M176) has been written on every save since it landed, after P137 on the same day, and both first shipped
in v1.8.3: a file that has it comes from a build whose phone default was already 60, so its Uncapped is the player's
own pick. The Steam Deck move-up still runs either way. `Settings::parse` calls it instead of `upgrade()`; everything
that reads settings.txt (pause menu, main menu, Auto's save) goes through there, so menus, the saved file and the
applied limit now agree and the choice survives the round trip.

Tests: `graphics::tests::a_chosen_uncapped_stays`, `main_menu::tests::settings_round_trip` (Uncapped on Default
survives text → parse).

## Not verified / not 1:1

- Not run on a phone: the cap only exists on iOS/Android builds (`PHONE_OS`), and desktop tests check the phone
  branch only through the `PHONE_OS`-dependent expectations. Logic checked by reading; the iPhone round trip
  (Uncapped → pause Settings change → main menu) still wants a look on the device.
- A phone file saved by a build from before M176 with the player's own Uncapped comes back at 60 once (can't be told
  apart from the old default); after that it's kept.
- Remaster-only setting, nothing in the original to compare.
