# B785: a net-cord fault's call timing at 50 Hz (spec phase)

Spec phase. The port phase follows from `SPEC.md` in this dir. Probe is M183a1 part 12's EU bot match.

## Findings
- Phase 24 is the point-over after a serve fault. Its recorded tick is 6636.
- It records 64 ticks. Play plays 65.
- Phase 33 is another fault. It matches at 65 ticks.
- The task assumed the net cord causes the extra tick. It does not.
- Probe run with `HST_M183A1_CHANGE_ENDS=1 HST_M183A1_PHASES=40`: still 24 of 144 phases match. Phase 24 wants 64, gets 65.
- Debug print in play (since removed):
  - Both faults run the same way in play.
  - The umpire's line is heard stopping at post-point tick 29. Voice ticks left is 28 at tick 1.
  - The call settles there, then hold 29 and fade 5.
  - The countdown (37) never decides.
  - The animation length is 0 in the headless test.
- M182's European flow test (`crates/hst-sim/tests/europe.rs`, `match_flow_runs_on_the_european_timing`, fixture `eu-bot-match.bin`) with a temporary print (since removed):
  - It lists 11 called phases.
  - 9 are faults:
    - 8 serves into the net. Each drops back on the server's side. The judge's ball call is "net, then out".
    - 1 plain out serve (vsync 52454).
  - 2 are out-after-net calls in rallies (vsyncs 53275 and 55830).
  - Only vsync 44584 has a line end of 28. That is probe phase 24, the match's first fault.
  - The other 10 all have 29.
  - That includes 7 serves into the net with the same path, call, umpire (0, voice a) and clip (29 frames at 60 Hz).
- So the net cord is not the cause.
- The cause is the voice-stop read jitter already noted in two places:
  - P0b4c (`research/journal/2026-10-06-p0b-rules/6-CALL-TIMING-FINAL.md`): in US calls, 3 of 10 heard the line a tick early.
  - M182: "a tick sooner once".
- How it works, as far as the data shows:
  - The game asks the sound driver whether the umpire slot is still sounding.
  - The sound processor is not locked to the frame.
  - At 50 Hz a 29-frame clip is 24⅙ ticks long.
  - So where in its frame the line started decides the tick.
- No game-state cause was found.

## Outcome
- `SPEC.md` says play keeps the usual rule. The stop is read from the sound driver as before.
- Port phase, test-only:
  - Add a stand-in to the M183a1 EU probe. Listed recorded ticks (6636 for phase 24) hear the line a tick early.
  - It defaults to none, so nothing changes outside the probe.
  - It lives under `#[cfg(test)]`, next to the change-of-ends stand-in from part 12.
  - After it is in, run the probe and check that phase 24 matches.
- Not a gameplay change. No release note.

## Not verified / not 1:1
- The jitter's cause is inferred from the data. It is not traced inside the sound driver. Its timing is off the EE side (the sound processor), so it is not modelled.
- The stand-in is not yet written or run. That is the port phase.
- The 50 Hz line ends were checked only for umpire 0, voice a.
