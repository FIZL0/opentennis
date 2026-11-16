# M163a1 — Japanese release rules: port phase

I ported every rule in `SPEC.md`, working from the spec alone with the decompile locked. A match on the Japanese
disc now plays by the Japanese build. The US paths are unchanged and every switch is off for the US.
`Release::playable` still turns the Japanese disc away; follow-up **M163a1a** records it in PCSX2 and then opens it.

## What landed
- **CSV columns (§1)** are in `hst_data::columns::us_layout`, applied in `xb::Archive::read`.
  - Japanese `TParam.csv` rows have 63 cells. They get the rally timing pair (cells 28/29) copied into the four
    serve cells, so the serve tables are the rally table, as §2 says.
  - Japanese `AIParam.csv` rows have 43 cells. They get 0 for the lob and special-serve reactions.
  - US rows come back byte for byte. Every reader sees one layout.
- **`hst_sim::build::Build`** has two values, `Us` (the default) and `Japan`. It is one switch per rule:
  - Strong-toss miss: `ServeData.miss` is `None` on Japan, so there is no table and no RNG side draw.
  - No special serve.
  - No counter: `Finish.no_counter`, so there is no banner and no counter-winner name either.
  - No dull flag.
  - Smash narrowing over 6.4..11.885 by 0.3.
  - No 90° widest angle.
  - No lob or special-serve reaction draw: `ai_draw_guess`.
  - The smash search has no baseline check: `rally::Body.build`.
  - km/h is ×60×60×60÷1000, for both the shared length and the figures' length.
- **`play/build_rules.rs`** reads SYSTEM.CNF at match setup and applies the build to `Game`, `aim_stats`,
  `serve_data` and `finish`. The new fields are `#[serde(default)]`, so snapshots stay compatible.
- **Red landing marker on the serve (§5):** it is display only, in `Marks.serve_red`. The singles CPU still reads
  `marks.red`, which is US-equivalent.
- **Match art (§8)** is in `play/release_art.rs`.
  - The info sheet is in another folder of `INPANE` on the Japanese disc. It is now found by its file name.
  - The counter banner and the skull pop-up are missing on that disc. They are drawn blank: the counter never
    happens there.

## Checks
- These tests pass under `tools/check.sh`:
  - `columns`, the 3 unit tests
  - `hst-sim` `build`
  - the `aim`, `serve` and `ai_rally` tests
  - `hst-play` `build_rules`, `popups` and `hit_fx`
  - `hst-data` `releases`
- `build_rules::japan::a_bot_match_on_the_japanese_disc` is a headless bot match on `fixtures/discs/SCPS-15113.iso`.
  It checks four things:
  - The build is Japan.
  - The serve miss is gone and the counter is off.
  - Stamina is in its US column and the serve grades equal the rally grades.
  - Every serve shows the red marker while `marks.red` stays empty.
  It skips without that ISO, even under `HST_REQUIRE_DISC`, because it isn't the user's disc.
- `release_art::the_japanese_disc_finds_moved_art` checks the info sheet, the counter banner and the skull lookups on
  the Japanese disc.
- US bit-exactness is covered by the replay tests in the full suite run by `tools/done.sh`.

## Not verified / not 1:1
- **No PCSX2 recording of the Japanese disc.** There is no save state for it. A bot match means driving the
  Japanese menus from boot, which is a session of its own. So the ported rules match the spec and the tests but are
  not proven against the running Japanese game, and `Release::playable` still refuses Japan. M163a1a covers the
  recording, the comparison and opening the disc.
- **Front-end menus on the Japanese disc:** `message0.dat` is missing from `MENU00B`, and other MENU art has moved.
  The pause screen's venue caption and assignment screen, and the main menu, would stop on it. This is not reachable
  while the disc is refused. It is in M163a1a.
- **Cutaway/instant-replay km/h** (`instant::speed_kmh`) and **tornado km/h** still use the US formula. They are
  presentation (§9), and only the last bit could differ.
- **Gallery-sound pick (§9):** left as is, presentation.
- **Practice-only differences in the spec:** not ported, because this remaster has no practice mode that uses them.
- **Skull pop-up and counter banner art** are blank on the Japanese disc rather than taken from elsewhere: the disc
  doesn't have them.
