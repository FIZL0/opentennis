# M193 Match saves, core

`crates/hst-play/src/play/save_game.rs`, registered in `play::plugin`. Mockups (concept round 3) are in the main
checkout's `research/journal/2026-11-18-m193-match-save/concept.html` (untracked there; not copied in).

## What a save is

- `Setup`: singles/doubles, characters and costumes by match player (a costume is also a CPU's strength), which seats
  1P..4P are people, hand toggles, handicaps, court/stage, umpire, sets/games, character mods by slot plus the court's
  and umpire's mod (`ModPin`: folder, dir, `mod.json` version), the rules release (`rules_choice::chosen`) and match
  rate (`MatchHz`), and optionally the flags the menus started the match with (`Setup::flags`, for the UI task to
  restart through the usual match start).
- `Head`: build id (`handshake::build`), mode (Local/Online), saved time, `Setup`, a score `Summary` for the card.
- The state: `snapshot::encode` of the match as the current point started.
- File: `HSTSAVE1` + u32 length + head as JSON + state bytes, so any build can read the head and show "Saved in vX".
  `Saves::user()` = data dir `saves/`, file name = hash of mode + line-up (seats, characters, costumes, mod folders),
  so the same line-up and mode replaces its save; after each write only the newest `KEEP` = 5 stay.

## Point start

The instant replay already keeps a full `snapshot::save` at each serve set-up (`Recorder.serve`), so no extra copy is
taken per point. `PointStart` mirrors it after each tick while no replay plays (a replay puts an older point's start
back); `capture` uses the recorder's (rollback-consistent) one, or `PointStart` during a replay. Capturing restores
that state, encodes it and restores the current one (as `snapshot::encode` already does for the replay's serve).
Keeping is read-only: a match that's never saved plays exactly as before.

`resume` decodes the state into a match freshly set up from the save's `Setup` (`Setup::insert` for the headless
path), then sets the recorder's serve (and `PointStart`) to the resumed state: without it the first tick would take the
"match's first tick" path, dropping a pad from the replay log.

## Refusals

`Head::check`: another build → "Saved in v1.7.4" (the whole id when only the commit differs); another rules release or
match rate → "Rules changed"; a mod missing or at another version → "Mods changed".

## Test

`save_game` (hst-play): the app's singles match driven by `fixtures/round1.bin`'s 1P pads, runs to a later point and
200 ticks into it, `capture` (the hash afterwards is unchanged), writes/reads the file, builds a fresh app from the
save's `Setup`, `resume`s: the hash equals the uninterrupted run's at the point start, then 2400 ticks on the same pads
(several more points) match tick for tick. Plus `a_line_up_keeps_one_save_and_the_newest_five_stay` and
`refusals_say_why` (no disc needed). 3.4 s together.

## Follow-ups

M198 local menus (pause Save and Quit, Continue card and list, browser storage), M199 online lobby resume.

## Not verified / not 1:1

- No PCSX2 comparison: saving is ours (the original has no mid-match save); the proof is the hash test above.
- The test app leaves out `play_sounds` and the audio (like the other headless match tests); a full app's sound side
  isn't covered by the hash test.
- Entities outside `Figure` (umpire, walkers, creatures) keep the fresh match's motions, as a netplay resync does;
  they don't feed `state_hash`.
- Visual setup state drawn at match start from the seed (sky/clouds, `main.rs`'s `match_weather`) isn't in the save:
  a resumed match gets the fresh setup's; only looks.
- Saving during an instant replay saves the start of the point being replayed (the next point's start doesn't exist
  yet), so that point is played again on resume.
- `Saves` writes with `std::fs`: browser builds can't keep saves yet (M198).
- No UI yet: nothing calls `capture`/`resume` outside the tests (M198, M199), so no release note.
