# M151d — a mod with generated tables plays online

## What changed
- `mods_online::Wire` carries the rest of a mod's §6 build: `power`/`arc` per shot class, `swing` (swing_speed),
  `run` (run_speed), `disc` (disc_body) and `scale`. Old JSON without them reads as stock (serde defaults 1.0/false).
  `Wire::of` no longer refuses any §6 field: everything in `mods::Build` now goes on the wire.
- `Wire::check` takes `mods::read`'s ranges for the new fields (power 0.5–2, arc 0.25–2.5, swing 1–4, run 0.1–4,
  scale 0.25–4 and only with `disc`; NaN fails the range test). A disc body sends no skeleton (and must not).
- `for_match` sets the whole `Build` from the wire. For a disc body it fills the ten `disc/cNN` costumes (so a peer
  without the folder still loads the donor's disc body, scaled, through `load_disc_scaled`, not the tinted stand-in)
  and leaves `skeleton` None: every peer builds the scaled skeleton, motions, pelvis keys and arm table from its own
  disc, the same way the offline load does. `kept` gives a disc body an empty skeleton so `find` matches it by id.
- Generated tables: every peer builds them in `mod_match::power` from the wire's power/arc and its own disc, as offline.
  To be sure they come out the same on every peer, `hst_sim::shot::probe()` fingerprints the generator: 27 serve cells
  (the arc sample positions) solved to their targets, then `shaped` at power 1.4 with the spin moved 30° (exercises
  `solve`, `cell`, the ball step, the net rule); ~20 ms debug. `Handshake` gains `tables` (sent in the lobby
  `Hello`, cached `handshake::tables()`), and the host refuses a joiner whose fingerprint differs ("your device computes
  the game differently…"). A peer that sends none is taken on its build (an older build is refused by the build check
  anyway). The fingerprint is not per mod: building a mod's actual tables (1–1.5 s per table) at the door was too slow,
  and with build and disc already equal, the only way two peers' tables differ is the platform's float results, which
  the probe covers. It also covers the ball step itself, so a mismatch would desync stock matches as well.
- `state_hash` prints the probe; `tests/platforms.rs` confirms x86_64, aarch64 (qemu) and wasm32 agree (6021 lines).

## Build fields that refused online (task's second half)
All lifted: `disc_body`, `scale`, `swing_speed`, `run_speed` (`stats` and `shots.from` already went on the wire).
swing/run are plain numbers the sim reads (`swing_pace`, `stats.paced`); scale changes TParam's size columns (built
from the wire's base row on every peer) and the disc body (every peer's own disc).

## Tests
- `hst-base` `mods_online` wire tests: round trip, ranges refused, old JSON defaults, the stand-in carries the build,
  a disc wire gives disc costumes, donor voice, no skeleton.
- `hst-base` handshake: `tables` mismatch refused, missing accepted; `gameplay_hash` now includes the build.
- `hst-sim` `shot::powered::tests::probe_is_quick_and_pinned` (pinned value), `tests/platforms.rs`.
- `hst-play` `netplay::…::a_mod_with_generated_tables_plays_alike_on_every_peer` (HST_NETPLAY=1, 8 s): a disc-body
  mod (scale 1.25, swing 1.5, run 0.8, stats S, serve power 1.4 arc 1.5, stroke from 9 as "cannon") in a doubles
  match, owner (folder) vs a peer with no folder: serve tables > 1.3× stock Big Chief's on both; state identical for
  2500 mashed ticks. The B556 Anya test still passes.

## Not verified / not 1:1
- Nothing to compare with in the original: online mods are an addition. Stock play is untouched.
- Not played over a real network between two machines; covered by the two-app lockstep test and the platforms test.
- The lobby still labels a disc-body mod whose folder a peer declined as a stand-in and may offer its (tiny) folder
  for download, though that peer plays the real disc body anyway (cosmetic; online.rs not touched).
- The handshake fingerprint checks the generator on a synthetic table, not each mod's own tables.
