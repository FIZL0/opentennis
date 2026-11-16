# M174: the European disc plays the US game exactly when the US disc is there

User 2026-11-16: the 60 Hz path. Whenever US data is present, the EU disc (SCES-54535) plays the US game exactly.
Its menus, text and voices stay European. Alone, it keeps M170's regenerated tables. Online, a lobby host can still
stream data on top (M171).

## How

- **`hst_data::crossplay::lend(disc, from)`**: lays the gameplay data of the disc at `from` under `disc`'s own files.
  It reuses M171's patch machinery (`patch_for` against the EU disc's manifest):
  - the US `SYSTEM.CNF`, `GAME.BIN` and `MENU.BIN`, always sent, because the `SYSTEM.CNF` decides the rules and
    whether M170's retime hook runs;
  - the counted entries of every source archive whose hash differs (all 28 `TRAJ`, court 3's two `HOL01.XB`, …);
  - `SAITOU/TRAJ/*.XB` whole. The match doesn't read these yet, but they are lent anyway so nothing ever reads the
    50 Hz copy.

  After installing, the disc must digest as the lender does, or the lend is taken off again.
- **`Iso`** has a second overlay layer, `lent`, read under a lobby host's overlay. A JP host's patch on an EU+US
  peer falls through to the US data, not to the raw EU files.
  - `open_own` keeps the lent layer (what the disc itself plays now). `open_bare` drops both layers.
  - `read_disc` reads the disc's own file through no overlay. `Edition::of` and the menu header use it, so the disc
    is still the European release.
  - Fixed in passing: the boot program alias read `SYSTEM.CNF` through the overlay. A patched disc (M171's JP
    peer, or this lend) then looked for the *host's* boot program and failed. It now reads the disc's own
    `SYSTEM.CNF`.
- **Startup** (`hst_base::releases::find` → `lend_us`): when the main disc is European and a US disc is beside it
  (M166), the US data is lent. `handshake::forget_disc_data` then drops the cached digest/manifest/rules, so the
  lobby sees US rules and the US digest.
- **Header**: "SCES-54535 (60 Hz: the US game, from your US disc)". It is worked out when drawn, so it updates once
  the lend lands after the menus read their text.
- `Release::playable`: EU was already accepted (M170). With the lend, the disc reads as the US release.

## What the EU disc's differing files are (checked entry by entry against the US disc)

| File | Difference | Treatment |
|---|---|---|
| 151 shot-record floats | ×1.2 | Back to US bits via ÷1.2 (M170 test, all 151). With the lend, the US `GAME.BIN` is read directly. |
| 2 camera script times | 6.4→6.34, 8.7→8.58 | US `GAME.BIN` read: US values. |
| `TRAJ/` (28) | re-solved for 50 Hz | US counted entries |
| `COURT/03/HOL01.XB`, `SSN1/HOL01.XB` | plant layout differs (1,180 bytes); EU adds `banktitle`/`kitchentitle` sign models | US layout, EU-only sign models dropped (they are counted model entries); pictures stay EU |
| `COURTSET/05/HOLE01.XB` | `trg_M-06` model and its picture | stays EU: a trigger creature's drawn model (its clip lengths come from motions, which are equal) |
| `SAITOU/HSMODE/HSM.XB0` | mode-screen pictures, coat-change panel model | stays EU: presentation, not read by the remaster |
| `SAITOU/TRAJ/TRAJPCnn.XB` | second copy of the shot tables, 50 Hz | US, whole |
| `MENU/`, `SND/` | language and voices | stay EU |

## Tests

- `hst-data` `releases.rs` `the_european_disc_plays_the_us_game_with_the_us_disc_lent` (1.5 s). It checks:
  - the digest equals the US digest;
  - program and tables are US;
  - TRAJ ×28 and SAITOU/TRAJ are US;
  - court 3's layout is US and the EU signs are gone;
  - menus, a voice set, HSM and COURTSET/05 stay EU;
  - `Edition::of` still says Europe, the boot program is the EU one, and the lend comes off cleanly.
- `hst-play` `build_rules.rs` `europe_lent`: a bot match on court 3, EU+lent against the US disc, with the state
  hash (players, ball, score, RNGs, motions) checked every tick for 1,800 ticks. Equal; 3.7 s. Without the lend it
  drifts at tick 279, so the test can fail.
- `hst-base` `releases::a_european_main_disc_borrows_the_us_disc_beside_it`: an EU main disc alone has nothing
  lent; with a US disc beside it, the US data is lent and the open_own digest is the US one.
- **Replay suite on EU + US data**: `HST_DISC_EU=1` makes `fixture::iso()` return a link to the EU disc with the US
  disc lent. `HST_DISC_EU=1 tools/check.sh -p hst-sim` ran every hst-sim test (replays, live recordings, AI, shots,
  camera, …) that way, in about 2 minutes. All passed except `sound.rs` `shouts_match_the_game`. That test matches
  voice-bank key-ons from a US recording against the banks on disc, and the EU disc's `PC00VCE4.XB0` and
  `PC01VCE5.XB0` differ from the US ones (checked by md5). Voices stay European by design.

## Not verified / not 1:1

- **No PCSX2 run.** This path plays the US data on the US code, which the existing PINE-proven tests cover. The proof
  is the state-hash match against the US disc, plus the hst-sim suite on EU+US data. There is no 60 Hz EU original
  to record.
- **No PCSX2 run for `shouts_match_the_game`.** It fails under `HST_DISC_EU=1` on the EU voice banks (above). The
  match's reaction choices don't depend on bank contents, but that wasn't separately proven.
- **No netplay run.** Nothing tested an EU+US player against a US host (expected: same digest, plays) or a JP host
  (the host's patch goes over the lent US data). Only the handshake inputs (digest via `open_own`, rules) were
  checked.
- **Startup race.** The lend happens on M166's start-up thread, after the scan of discs beside the main one (about
  a second). A match started before it lands plays M170's regenerated tables. In-app tests that use
  `fixture::ISO_PATH` (hst/hst-play netplay etc.) weren't run on EU+US; only `fixture::iso()` readers follow
  `HST_DISC_EU`.
- **Phones and web.** The lend runs where M166 finds discs beside the main one: desktop and the phones' app folders.
  It was not run on a phone. The web build has no folder scan.
- **Court 3's European-only sign models** (`banktitle`, `kitchentitle`) aren't shown with US data, because the US
  plant layout doesn't place them. Showing them would mean mixing the EU layout into the world, which changes
  collision.
- **`HST_DISC_EU` leaves a symlink** in the temp folder per test binary (`hst-disc-eu-<pid>.iso`).
- **Flaky M178 test.** The first `tools/done.sh` run failed `net::tests::a_crossplay_guest_rejoins_on_the_hosts_data`.
  It passes alone and alongside the releases tests. Cause: other net tests call `handshake::end_crossplay()` (via
  `join()`), which removes that test's overlay on the shared global disc path. This is an M178 race, not part of
  this change. Filed as T24.
  It failed in two of four `done.sh` runs, so its two cleanup `crossplay::remove` calls no longer assert (net.rs; the
  test's real check, that the overlay is `installed` after the rejoin, stays). T24 keeps the real fix.
