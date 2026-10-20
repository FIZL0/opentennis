# M136: mod downloads/uploads on every menu screen

Front-end feature (ours to design, PLAN.md): no PINE / oracle proof needed.

## What it does
- `main_menu/mods_screen/transfers.rs`: a slim pill centred on the title band (640-space x 175..465, y 0..22) on
  every menu screen but the match / pause. Each direction under way gets half: "Downloading 3/5: Ace Pilot"
  ("+N" when several download side by side) / "Uploading 2/3: <mod>", with a gold bar for the batch (a download's
  own "(45%)" progress counted into it). Data: the existing install queue (`queue`, `doing`, `batch`, `jobs`) and
  a new upload batch count `up_batch` (reset when a new batch starts from idle, ended ones counted in `uploaded`).
- New `widescreen::Anchor::Top`: centred across like `Centre`, kept to the window's top like `Sides`, so the pill
  stays at the top of a portrait window (Centre put it mid-screen there).
- Restart prompt: when the last queued download ends and the mods changed (`fresh`), `due` is set; the existing
  B205 Reload / Later popup comes up once no upload is going and the screen is calm (main, mode, settings pages,
  mods, extras, online menus, no other popup / dropdown / child / lobby / pickers). The popup is now drawn in
  `drawn` (all screens) instead of `home::layout`, and its press / pointer / legend arms no longer require
  `Screen::Main`.
- Shots: `HST_TRANSFERS=1` fakes a batch of each on any `HST_MENU` screen, `HST_TRANSFERS=ask` puts the popup up.
  Shots in `context/M136/run2,run3` (main, browsemods, graphics, online, 700x1200 portrait, 1000x750 4:3, popup
  over Browse).

## Tests
`mods_screen::transfers::tests`: percent parsing, the download batch counting down, the upload batch, the popup
waiting for a calm screen and for uploads to end.

## Not verified / not 1:1
- Not tested with a real store download/upload end to end (the network store); driven by the same events the Mods
  page already uses, checked with the unit tests and `HST_TRANSFERS` shots.
- The popup is held back on the character / court / umpire selects, the confirm screen, online lobbies and during
  matches (a restart there would drop the players' picks or the lobby); it comes up on the next calm screen.
- Not shown on the in-match pause menu (that's `hst-play`'s own screen, not the front-end's).
- In a narrow portrait window the pill touches the right end of a long title tab ("Settings"); readable.
- The popup's description goes to the bottom ticker in its mono font on every screen, as it already did on the
  main menu (pre-existing look, not changed here).
