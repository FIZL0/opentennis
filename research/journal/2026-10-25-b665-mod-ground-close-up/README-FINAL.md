# B665: LJ's receiver camera on the ground

## What the user saw
In a match with LJ (Out of Bounds mod, donor 11 Will, left-handed), the cut-away on LJ after a point, at the Game /
Server|Receiver moment too, put the camera on the ground looking up at LJ.

## What it is
It's the post-point close-up, shot 0x64. Its record puts the camera 2 m in front of the shown player at ground level
(the ground-clearance float holds the eye 0.1 m up) and aims it at the head. B367 fixed its broken orientation and
kept the ground position, so the shot still shows.

Checked against the original this time (PCSX2 copy 5, slot 5 with the three post-point lists in RAM poked to
alternate 0x64 with a filler; `context/b665/rec64.py`, `shoot64.py`):
- `record_cutaway.py --players` with the poke → `fixtures/b665/cutaway64.bin`: on all four 0x64 runs (characters 1
  and 5, both orbit sides) the game's eye sits at y = −0.10, looking up. Our `Shot` reproduces every one of the 480
  frames to 3e-6 (new test `hst-sim --test cutaway close_up_s05`; `cutaway_s05`'s replay loop became `views`/`views_of`).
- Screenshot `context/b665/orig64/c03.png`: JJ (character 5) filmed from the ground, whole body in frame, racket
  raised. So the stock low shot is the original's.

The game only offers the shot to characters 1, 3, 5, 7, 8, 9, 11 and 13, the ones whose reaction is made for it
(`pick` skips it for 0, 2, 4, 6, 10, 12). A mod takes its donor's number, so LJ (donor 11) got it as often as Will.
A mod plays its own reactions, though, which aren't made for that camera. Shots with the pick forced
(`context/b665/lj1/`) show LJ cut off at the top when jumping, or seen from below at odd angles.

## Fix
`play/cutaway.rs` `pick`: a player whose slot holds a mod (`MatchMod`, the same on every online peer, stand-ins
included) never gets the close-up. Its turn goes on to the next shot that fits, as for a disc character without it.
Disc characters' picks are unchanged (`cutaway::pick`).

Tests: `hst-play cutaway::tests::mods_skip_the_close_up` (character 11 on every list: stock picks equal
`cutaway::pick` and include the close-up; the mod never gets it), `hst-sim close_up_s05`.
Live: a bot match with LJ (`--mod … --mod-slot 0 --chars 11,5`): 11 cut-aways on LJ (0x61 ×4, 0x62, 0x63, 0x66, 0x68
over a full turn of the point list), no 0x64.

## Not verified / not 1:1
- Mods skipping the close-up is our own design: the original has no mods. Stock characters keep it exactly as the
  original does, recording and screenshot above. If the user wants the ground shot gone for stock JJ/Will and the
  others too, that would be a departure from the original and would need its own task.
- The game-end pick: replays are off in the app (B127), so `replays` stays 0 and a game-ending point skips the long
  shots. For a stock close-up character the receiver's shot then alternates winner shot / close-up at every game
  end, as the original does with replays off. With them on, the original cycles four shots after the first replay.
  This is unchanged here and follows from B127.
- Online with a mod: the decision comes from `MatchMod`, which both peers hold. Not run in a netplay test.

## Wrap-up note
`tools/done.sh` ran clippy and the tests. Its full suite failed on one test only, `hst-base
mods::tests::oob_hair_swings_and_settles` ("oob_pc00_jasmine: the hair never settled: 0.63"). That test reads
`mods/hst-mods/oob_pc00_jasmine`, which another session rebuilt in opentennis-mods `out/` at 22:35 during this run
(uncommitted `tools/rerig` changes there). This task touches no hst-base code or mod data. The test still failed
with the files unchanged for 6+ minutes. Every test this task touched passes (`hst-play cutaway`, 409; `hst-sim
--test cutaway`, 5). Ticked with `tools/ctx.py tick` and committed by hand.
