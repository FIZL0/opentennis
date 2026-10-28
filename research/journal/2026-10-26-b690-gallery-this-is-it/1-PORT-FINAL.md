# B678 port: the gallery's "This is it!"

Ported from SPEC.md with the decompile locked.

## What changed

- `hst-sim/src/sound.rs`: `Gallery` gains the line. `step` takes a `Watch` (hit count, a hitter, and whether a
  player is within 3 m of an original smash point, `Watch::near`) and runs: the sounding line forgotten once over,
  the cheer, the "This is it!" check (courts 1/4/6, ≥2 players, nothing sounding, nothing pending, once a shot, set
  before the distance test; program 9 at 0x4c, keys 1, 0, 1…, one court-generator draw for the stand, only when it
  plays), the reactions (while a line or the match-end ovation/groan sounds a pending reaction is dropped, but events
  7/8/9 wait, their countdown held), then a new hit (hit count changed with a hitter) clears the flag. `idle` also
  forgets the line. New fields `sounding`, `checked`, `line`, `hits`, `takes` (serde, deterministic).
  `Gallery::lengths` sets the takes' lengths in ticks.
- `hst-sim/src/effect.rs`: `SmashSearch::of` names each candidate's player; `whose` records it per placed point.
- `hst-play/src/play/smash_marker.rs`: `receivers` names a player only for the original's candidates (a human whose
  heights the lob cap left alone; a capped top is one the uncapped ball never reaches, so the original gives up
  there); our B507 points (all-CPU teams, capped) carry `None` and never draw. `watch(g)` builds the `Watch`.
- `hst-play/src/play/npcs.rs`: both manager call sites pass `watch` (computed before the court generator is
  borrowed; the walkers' step moves neither players nor ball). Intro and ceremony galleries pass `Watch::default()`.
- `hst-base/src/audio.rs`: `Sound::take_ticks` renders takes on a silent mixer of the disc's tables (the umpire's
  `centre_frames`), so lengths are the same with or without an audio device. `play.rs` `match_sounds` sets them from
  the match's gallery bank. Court 4: program 9 keys 0/1 = 47/37 ticks (0.78/0.61 s, as the spec measured); the
  ovation/groan 97–107 ticks on courts 1/4/6.

## Tests

- hst-sim `tests/sound.rs`: `this_is_it_courts_and_stand`, `this_is_it_once_a_shot`, `this_is_it_sounding`
  (dropped applause, pending reaction defers the check, line-length wait, ovation holds the cheer);
  `effect::points_remember_their_player`; hst-base `gallery_take_ticks`.
- Unchanged: hst-sim sound/rng/effect/lib, `--test replay`; hst-play smash_marker, npcs, ceremony, intro,
  match_stats, popups, replay, sound; `HST_NETPLAY=1` snapshot.

## Left

- "Within 3 m" is taken as ≤ 3 m (squared distance ≤ 9 in EE arithmetic); the boundary isn't pinned. Presentation only.
- With the ovation's real length, the after-match cheers now come ~100 ticks later than before (they wait for it),
  per the spec. `gallery_match_end` still passes with zero lengths.
- A court mod's replacement gallery bank isn't used for the lengths (the disc bank's are).
- Not checked by ear yet.
