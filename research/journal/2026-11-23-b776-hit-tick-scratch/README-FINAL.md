# B776: the stand test's scratch on player 1's hit ticks

Check task. Result: nothing to change. No spec, no port. M185's writer list stands as it is.

## Question

B773's port found three EU bot-match rally ticks (69, 188, 392) where the game's leftover scratch z, which a doubles
computer player's stand test reads, was 127, 8 and 128 instead of player 1's racket direction. Play's racket z on
69 and 392 had the other sign. The task was to find the writer and spec it as an override in M185's writer list.

## Method

- Dumped the full scratch quad (x, y, z, w) for slot 2 on those ticks from B772's EU dispatcher stack capture
  (`context/b772/run6/eu_stack.bin`). Also listed every quad with a non-unit vector or w ≠ 0 for slots 1–3 in the
  capture.
- Ran the EU bot-match probe (`the_european_bot_match_plays_as_recorded`, ignored test) with temporary prints in play's
  `scratch::z`, since reverted. At player 2's read they printed the shot count, the count kept at the end of the last tick,
  the last hitter, player 1's racket z and player 1's contact countdown. They ran at each strike and at each tick end.
  Log: `context/b776/eu.log`.

## Findings

- The three quads are trail colours. As four byte values (red, green, blue, alpha 70): tick 69 (62, 14, 127, 70),
  188 (128, 48, 8, 70), 392 (51, 51, 128, 70).
- This is M185's existing third writer. The previous player's hit starts the ball's flight trail later in that
  player's update, after the racket hand-off, and the trail setup leaves the colour in the slot. So it is not a new
  writer and needs no override.
- All three are player 1's contact ticks in the game. The brief's "clip time 7 of a queued swing" (69, 188) and
  "second tick of the speed-4 swing" (392) are where the swing reaches contact.
- In play, on the same ticks, player 1's strike runs in its own control step before player 2's AI. The shot count has
  gone up and player 1 is the last hitter. So `scratch::z` already takes the trail branch and returns +1. The game's
  values (127, 8, 128) are positive too, so the sign matches.
- The neighbouring ticks line up at shift 0. Game ticks 66–68 (−0.1697, −0.1160, −0.0712) and 70 (−0.0364) equal
  play's racket z on the ticks either side of its strike. Likewise 185–187 and 189 (−0.6497, −0.1694, 0.3050, 0.5984),
  and 390–391 and 393 (−0.7529, −0.3390, −0.0272).
- B773's "sign differs" compared the game's value with play's *racket* z printed for those ticks (−0.0185 at 69,
  −0.0030 at 392). It did not compare with what `scratch::z` returns. The racket value isn't used on a hit tick.
- Whole capture: every trail-colour quad has a blue byte of 8 or more, never negative. These are slot 1 at phase-3
  ticks 7, 169, 8, 8, 127 and 251; slot 2 at 96, 69, 188, 392 and 547; and slot 3 at 435.
- The phase-2 short non-unit vectors on slot 1 (e.g. (0.0162, −0.0400, 0.0029, 0)) are serve toss ticks. That is M185's
  toss writer, not the trail.
- This is the first check of M185's trail exception against the game. M185's port journal listed it as unverified.

## Not verified / not 1:1

- The US bot match's hit ticks weren't re-checked. The US recording has no full-quad dispatcher capture. The same
  branch runs there.
- The toss exception's timing in play (toss set before the next player's AI) was not checked in this task. It is
  outside B776's ticks.
