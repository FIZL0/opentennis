# M183a1 part 10: after B768, B769 and B770 (PART: blocked on B771 and B772)

Reopened because B768, B769 and B770 landed. Re-ran the probes 2026-11-23:
`cargo test -p hst-play --lib <name> -- --ignored --nocapture`.

## US (`the_us_bot_match_plays_as_recorded`)
- 28 of 112 phases match (27 before).
- Players part at game 2's first serve. Phase 28's set-up runs 253 recorded ticks against 240 in play. The game's AI
  generator draws one more than play at that serve's contact. That is B771 (open).

## EU (`the_european_bot_match_plays_as_recorded`)
- 7 of 144 phases match (3 before): point 1, point 2 and point 3's set-up (135, 303, 134, 164, 84, 134, 221).
- Point 3's rally runs 684 ticks in the game and 771 in play.
- With `HST_M183A1_FROM=1302` the players and the ball match tick for tick to 1584. Recorded tick 1300 has no sample of
  its own (a stall) and repeats 1299; from 1301 on they match again.
- The AI generator (`HST_M183A1_KEEP=2200 HST_B768_DRAWS=…` on `the_european_generators_play_as_recorded`) follows the
  recording through tick 1584. Point 3's serve contact is still split a tick apart (1181: game 0+70, play 2+68).

## Tick 1585: the net player
- The AI draws are game 4, 6, 4, 10 at ticks 1585–1588. Play draws 5, 3, 0, 0.
- In the recording the near-side net player (player index 2, standing at x −2.7425, z −4.1746) starts running at 1585.
  Its steps are 0.0775, 0.0780, 0.0786 a tick in a fixed direction (0.472, −0.881). That is a run from a standstill,
  the speed ramp of a 0.9 speed stat. In play it stays put.
- The baseline player (index 0) matches until it reaches its contact. The ball parts at 1610.

## What play does at that tick
A temporary trace, since reverted, showed the following.
- The far player's shot gives a new predicted path: 86 entries, window from entry 2.
- Player 2 runs all four tiered contact searches, one run-round chance draw each, and finds nothing.
- It then makes its formation repick (1 draw). That is 5 draws in all.
- Every candidate fails the in-time test. In the tight tiers the failures are small: path entry 23 needs 24 frames with
  21 available, and entry 27 needs 28 with 25. After the bounce the failure is large: entry 39 needs 54 with 37.
- Its runner state was stamina 40, standing, agility 40, size 100, Europe build.

## What the game does instead
- The game's run direction points at the stand spot of path entry 39, about (−0.41, −8.53). That is the ball just
  after its bounce. The direction there is (0.4713, −0.8820) against the recorded (0.4723, −0.8813).
- So the game's net player goes for the ball after the bounce, a spot play rates unreachable.
- The game's 4 draws fit 3 tier searches plus one more draw.

## Experiment (reverted)
- Cutting every search's run estimate to 5/6, or by 3 frames, breaks point 1 (1 phase matches).
- So a blanket 50 Hz rescale of the estimate isn't the answer.

## Open
- Is this 50 Hz only? No US recording shows a net player with no reachable contact, so it can't be told yet.
- The rule that lets the game's net player take that spot needs the original. That is B772 (spec).

## No code changed this session.

## Status
Blocked on B771 (US) and B772 (EU point 3's net player). When they land: EU tick for tick, then flip
`match_speed::FIFTY_HZ_SIM` and add the release note.

## Not verified / not 1:1
- Whole-match proof is still not done. US parts at game 2's first serve, EU at tick 1585 of point 3's rally.
- `FIFTY_HZ_SIM` is not flipped and there is no release note.
- The earlier parts' items stand.
