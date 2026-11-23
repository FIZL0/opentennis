# B769 — game 2's first-serve fault: the point-over and a missing AI draw

## Task

In slot 5's US bot match (`fixtures/match_s05.bin`), game 2's first serve is a fault in both the game and play. The fault's point-over lasted 80 ticks in the game and 38 in play. Around the serve's press/contact, the game's AI generator draws 1, 69, 2, 1 where play draws 1, 2, 66, 2, 1 (one draw short). Players part at recorded tick 6899.

## Findings

Worked from recordings only; no decompile.

- **Point-over length.** The headless test harness (`played_ticks_rng` in `crates/hst-play/src/play/fifty_hz.rs`) had no mixer, so the umpire's "Fault" voice was never timed. The call board settled as soon as its animation ended (38 ticks). Real play has the voice, and the post-point waits for it. The harness now inserts `Sound::test` from the disc fixture, the `heard` plugin and `play_sounds` after `held_ball`, as a real match does. The fault's point-over is now 80 ticks, phase for phase. The US test now asserts 28 phases (up from 27). All 8 `fifty_hz` tests pass, the EU ones included.
- **The missing AI draw.** At the serve's contact tick (play tick 6814, game vsync 14424), play draws 66 + 2 where the game draws 67 + 2. Per player at that hit message (server p1, receiver p0):
  - p0: timing/picks 16, reaction 1 (the fast-ball reaction took hold, so no guess roll).
  - p1: 15.
  - p2: 16 + reaction/guess 2.
  - p3: heard-hit 1 + 15.
  - A dummy extra draw placed anywhere after p0's whole sequence (before p1, before p2, before p3, or after all four) puts the AI generator on the recording at every recorded draw tick through the fault call (6862). It gives 29 phases (the second serve's set-up, 253 ticks), and players match until tick 7285.
  - A dummy draw before p0, after p0's timing, or after its picks fails earlier (at 6829 and 7218 respectively).

### Ruled out

- A special serve: the serve isn't one (char 2, flat, offset −6, grade 4).
- The fast-ball reaction not applying: forcing it off makes p0's hold 13 instead of 25, and phase 27 parts, so it applies.
- The guess chance still being rolled after a fast-ball reaction: breaks the earlier serves at ticks 1271 and 3983, where that case played without the roll.
- The fast ball not blocking the guess at all: breaks at phase 26.
- The serve history including the player's own team's serves: gives p0 a serve-pace error of 12 and parts at 6829.

### What is different about this serve

- It is the match's first serve by team 1, so receiver p0 has never heard an opponent serve and draws no serve-pace chance. Every earlier receiver did (ticks 1271 and 3983: 17 draws).
- It is the first grade-4 (badly mistimed) serve.
- Team 0 is on the far end after the change of ends.

Draw counts per tick can't tell the after-p0 placements apart. Identifying the draw needs the original's hit-message logic: follow-up B771 (a spec task).

## Not verified / not 1:1

- The extra AI draw at game 2's first serve contact is not identified or ported (B771). Players still part at 6899 in play, so the brief's "past 6899" check is not met.
- Only the US harness's voice timing was compared against the game. The EU tests pass with it on, but their point-overs were not re-checked call by call.
- `Sound::test` is a silent mixer built from the disc; voice lengths come from its sample lengths, which are assumed equal to real playback.

## Scripts

None new (B767's `us_rng_window.py` was used). A capture attempt of the second serve (`context/b769/`) failed because PCSX2 wasn't running; it was not needed in the end.
