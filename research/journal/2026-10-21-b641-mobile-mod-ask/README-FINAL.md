# B641: mobile guest, no download ask, played as Ashley

## What the code shows
I couldn't replay the user's lobby, so the cause is worked out from the code. Nothing in the ask or the pick is
specific to Android or iOS: the select, the lobby, `settle_to`, `mods_online::find` and the stand-in are the same
code on every platform. The symptom fits a gap that any platform has:

- A guest's select opens on the **Random card** (B346). Its `Cmd::Join` carries `picks()` as they are then: the
  grid's first character, **Ashley** (cursor 0), flagged random, so the lobby card shows "Random". The guest's seat
  keeps that pick until its player readies (`Cmd::Picks`). While it chooses, others only see its place (`Cmd::Live`).
  For a custom character that place includes the mod's id and name, so the lobby note says "<mod>: you'll be asked
  to download it" even though no wire has been sent.
- The host's Start counts down "ready or not" (B215). When the countdown ends, a player still on the select plays
  the seat's held pick, which is Ashley.
- With no wire in the line-up, `Room::mods()` is empty. The start doesn't wait for anyone's `Ready`, nobody is
  asked, and nothing downloads. A phone user is slower on the select, which explains why this showed up there.

The user saw a lobby that mentioned a mod, no ask, the match starting, and the guest as Ashley. That matches the
above whether the guest was on the mod card or on Random.

A second, smaller hole: the B558 Settings pages over the select or lobby hid the ask. `poll` raised it under the
pages, but neither the pages nor the lobby drew it, so B281's 10 s wait ran out unseen and the stand-in played.

## Fix
- `online.rs` `Menu::rush_picks` (from `poll`): once the host's countdown runs and this machine still has players
  on the select (not ready), their picks go in as they stand. A Random card is drawn once with `roll_random`, the
  same draw readying makes, and the card still shows ?. A custom character goes with its wire, read in first
  (`modfs::ready`, the browser). The picks are sent again if the player moves before the start.
  A picked mod is now in the line-up before `go`, so the start waits for every peer's `Ready` and the B281 wait
  asks the others. A guest's `Picks` is taken until `starting` (the 4 s countdown gives it time).
- `over.rs` `back`: the pages don't come back over the lobby while a mod prompt is up, so the prompt shows.
- Stand-in look (user 2026-10-21, which reverses B489): `mods_online::tint` is back (a 120° hue turn, the colour
  channels rotated in every RGBA8 texture of the parts, racket and GS materials). `play::mod_match::load` applies it
  to the donor it loads for a mod with no looks. Each load makes its own images, so a stock player on the same
  donor keeps its colours. The ask text now says "as its base character, recoloured".

## Checks
- `main_menu::online::tests::a_countdown_takes_the_select_as_it_stands` (on a copy of
  `context/mods/getagrip_pc00_emi`) checks that:
  - nothing is sent before the countdown;
  - nothing is sent while the mod is still being read in;
  - the picks go out with the mod's wire, and only once;
  - moving to a disc character sends that instead;
  - a ready player sends nothing;
  - a Random card is drawn once and sent still flagged random.
- `main_menu::over::tests::the_lobby_runs_under_the_pages`: the pages close when the ask comes up.
- `tools/check.sh -p hst online` (337 passed), `-p hst-play stand_in` (the stand-in path now runs the tint),
  `-p hst-base mods_online`. `cargo check -p hst --target wasm32-unknown-unknown` passes.

## Not verified / not 1:1
- **The user's lobby on a phone.** This unattended run had no Android or iOS device or second peer. The cause above
  is read from the code, and the fix is checked with menu-level tests (`Net::wired`), not over a real connection.
- **The tint has no picture check.** It's the same channel rotation as before B489, now run on the stand-in path
  (`stand_in_simulates_like_the_mod`). No screenshot was taken. A texture pack switched on or off mid-match re-applies
  its images and would drop the tint for the rest of that match. Compressed (non-RGBA8) textures stay untinted.
- If the countdown ends before a browser has read a mod's files in, that player still plays the donor (no wire).
  The 4 s countdown is normally enough.
- Not checked against the original: it has no online play or mods (the menus are ours, PLAN).
