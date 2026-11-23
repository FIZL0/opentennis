# B785 spec: when a fault's call is heard to end at 50 Hz (European disc)

## What the task assumed, and what the recording shows
The European bot match's first fault (probe phase 24, recorded tick 6636) runs its point-over 64 ticks. Play runs
it 65, as it does the match's other faults, which the game also runs 65.

That serve was not a net cord in the sense of "clipped the tape and went over". It went into the net and dropped
back on the server's side: the judge's ball call is "net, then out", and the verdict is a fault. Eight of the
match's nine called faults are serves into the net like this one: the same path, the same call, and the same
umpire (umpire 0, voice a) with the same clip (29 frames of 60 Hz audio). Seven of them run 65 ticks. Only this
one runs 64. The ninth called fault, a plain out serve, also runs 65. So neither the net nor the kind of fault
decides the length.

## The rule
- A called point-over's call sprite settles on the first tick that two things hold:
  - its animation has played out;
  - the umpire's call line is heard to have stopped, or the call's countdown has run out.
- The settled sprite then holds 29 ticks and fades out over 5 (unchanged, `flow.rs`).
- "Heard to have stopped" is the sound driver's answer to "is the umpire's voice still sounding?". The answer comes back
  from the sound processor, which does not run in step with the game's frames. So the tick the stop is first
  heard follows the clip's real-time length, not a tick count the game keeps:
  - Usually: the clip's ticks (frames × 50/60, rounded down) plus the voice-stop lag (5 ticks,
    `umpire::VOICE_LAG`), counted from the tick play starts the line (24 + 5). That is tick 29 for a fault here.
  - Now and then, one tick sooner (tick 28).
- At 50 Hz a 29-frame clip lasts 24⅙ ticks. Whether its end, together with the driver's reply delay, lands
  before or after a frame boundary depends on where within its frame the sound processor started the line. That
  position cannot be read from game state.
- Rates seen so far:
  - European bot match (M182's `eu-bot-match.bin`): 1 of 11 call lines heard a tick early. The early one is the
    one at vsync 44584 (this phase).
  - US slot-5 match (P0b4c): 3 of 10.
  - Neither recording shows a game-state cause.

## What the port does
- Play keeps the usual rule. No game-state predictor exists, so play has nothing to change. The port should not
  add a net-cord special case: it would break the seven matching serves into the net.
- The M183a1 whole-match proof must feed the recording's line end for any call heard a tick early. Without it the
  probe can never match past such a call.
  - It is a test-only stand-in, like the change-of-ends draw stand-in (`fifty_hz::CHANGE_ENDS_DRAWS`).
  - Keyed on the recorded tick the point-over starts at (stable across probe changes, unlike a phase number). The
    European test lists those ticks; for a listed one, play counts the line's ticks left one shorter.
  - It lives under `#[cfg(test)]` beside the change-of-ends stand-in and defaults to an empty list, so release
    play is untouched.
  - The fault at recorded tick 6636 is the one known case.
  - Phase 33 and the other faults keep the usual end.
- Check: with the stand-in on for tick 6636, phase 24 runs 64 ticks. That alone is given by construction; what the
  check proves is that one tick of line end is the whole difference: phases 25 to 37 must then match as well (M182's
  flow test already reproduces this phase exactly with line end 28), and phase 38's known rally part is the first
  allowed miss.
- The match's other calls (out after net at vsyncs 53275 and 55830) end on the usual tick and need nothing.

## Not covered
- An exact rule for which calls are heard early. The deciding input is the sound processor's timing, which the
  game's own state does not hold, so no port can predict it; this is the accepted limit of the match proof here,
  as P0b4c already accepted it for the US build. It is not an open question for the port: the port's work is the
  stand-in above.
- How the early ticks are spread over a match: two recordings, 4 early of 21. That is too few to model, and the
  cause sits outside the game's own state.
- Other umpires and voice b at 50 Hz: no recording ends a call on their voice.

## Open questions
none
