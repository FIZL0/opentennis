# R14: EU point 2's set-up "5 ticks late" is five stalled vsyncs, not a wait in the set-up (FINAL)

## Result
Nothing in the game's set-up waits. The new-point AI reset runs in the tick the match changes to phase 2 (serve
set-up), as in play. The recording's "recorded" numbers are vsyncs, and the game stalls vsyncs with no tick in them, so
by point 2 the vsync numbers run 5 ahead of the tick numbers. Play's tick 572 is the game's vsync 577.

## How it was measured
- `research/eu_draw_calls.py <slot> <vsyncs> <out.bin> [frames.bin]`: patches the EU
  generator's draw routine (a stub on the AI generator only: vsync, caller and 16 stack words per call) and, with a
  4th argument, also saves the match object's first 0x100 bytes every vsync. Run from `eu-bot-start.p2s` loaded as slot 8.
- The EU start state is a v2.9.179 save: the v2.9.52 slot copy refuses it ("no longer compatible"). Start the copy with
  `HST_PCSX2_BIN=/usr/bin/pcsx2-qt` (the system v2.9.179). A restart is needed after each run: the patch stays in RAM
  and a second run would jump to itself (the script asserts it).

## What it shows (vsyncs from the match object's appearance)
- Point 2's draws: 93 in ONE vsync (739), 5 AI objects x 13 call sites x 5 + 28 from one shared caller, the same
  bursts as the recording's 91 + 2 at its samples 725/726. In that vsync the phase byte changes 0 -> 2 and the phase's
  tick counter restarts at 1. Phase 0 ran ticks 1..327 up to the vsync before.
- Point 1's set-up draws the same 5 sets in two groups (72 at vsync 335 during the intro, 19 at 398, 63 vsyncs apart);
  point 2 draws them all at once at the phase change.
- Stalled vsyncs in phase 0 (tick counter not advancing): ~9 right after tick 1 (the first loading stall, already
  handled by the harness: B723), **2 at tick 124 (vsync 532-533) and 3 at tick 250 (660-663)**. The 2 + 3 = 5 are the
  "5 ticks late". Play's tick numbers don't include them.

## Not verified / not 1:1
- Why play spreads the 89 draws over 3 ticks (572-574) where the game does them in the one phase-change tick: not
  captured on play's side this task; the follow-up checks it once ticks are mapped.
- The stall positions come from one capture (not a repeat run); the 5 total agrees with the offset found in part 7.
- The stalls' cause (loading) is not identified; the harness only needs where they fall.

## Next
Follow-up queued: map the recording's vsyncs to ticks through the match object's tick counter in
`the_european_generators_play_as_recorded` (and the EU match probe), then check play's reset tick.
