# M175: Settings → Gameplay → Rules

## What landed
- `hst-base/src/rules_choice.rs`: the setting. Releases (`us`, `jp`, `eu`); the choices are the main disc's own
  release plus any sibling disc registered with `rules_choice::add_sibling(path)` (M166 calls it for each disc it
  finds; until then only the main disc's release shows, and the row is fixed). Default (`rules = disc` in
  settings.txt, index 0) = the main disc's own; a saved choice whose disc is gone is kept and falls back to the own.
- Choosing another release lays that disc's gameplay data over the main disc with M171's crossplay overlay
  (`crossplay::patch_for` from the sibling against the main disc's raw manifest, `crossplay::verify` against the
  sibling's digest), so every `Iso::open` of the main disc reads the other release's game with its own textures,
  voices and menu text. `build_rules::of` reads SYSTEM.CNF through the overlay, so `Build` follows by itself; no
  hst-play logic changed. An EU disc + US sibling reads as the US digest exactly, with the 60 Hz regeneration off
  (SYSTEM.CNF is the US one under the overlay).
- Online: `net::host` holds `Online::host()` (the choice stays laid; `rules_choice::lobby()` gives the room's
  rules + digest from it; `handshake::patch_for` reads the disc through the overlay when laid, so guests get the
  chosen release's data). `net::join` holds `Online::join()` (our overlay comes off; the guest's manifest/digest
  were raw already, so it plays the host's rules). Setting changes during a lobby wait until it ends.
- Menu: Settings → Gameplay → Rules (first row; the Gameplay tab now shows on every device). Settings round-trip.

## Tests
- `hst-base rules_choice::another_release_beside_the_main_disc`: US + JP beside (reads as JP digest/release),
  EU + US beside (US digest, TRAJ read == raw, i.e. no regen), menu archive stays the main disc's, a guest on the
  main disc alone verifies against the overlaid host's patch; removed → own digest again. Skips without the
  JP/EU fixtures.
- `hst options::rules_row`, `main_menu settings_round_trip` (Rules line).

## Not verified / not 1:1
- No full online run with a host on a sibling's rules: the net.rs path is exercised only through its pieces
  (lobby(), patch_for over the overlay, verify on a guest path) because siblings can't be registered from the
  disc-load flow until M166.
- The main menu's "Disc:" header and anything else reading SYSTEM.CNF through `Iso::open` shows the chosen
  release's serial while another release is laid (menu text itself stays the main disc's).
- Laying a choice reads both discs' digests (a few hundred ms) at startup / on change; on wasm that is on the main
  thread. Only happens with a sibling registered.
- Gameplay exactness is inherited from M171's crossplay (digest-checked data) and M163a1's Japanese build; no new
  PINE verification was needed (no sim code changed).
