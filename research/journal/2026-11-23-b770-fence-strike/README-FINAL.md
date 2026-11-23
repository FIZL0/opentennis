# B770: court 6's fence stops the ball in the test harness too; the computer players hear the strike the tick after next (FINAL)

## Result
The stage that B768 and B723 called "the ball meets court 6's stage" on tick 471 (point 1's point-over tick 34, EU
match) is the ball striking one of court 6's lattice fences behind the far baseline. That fence is a ball-hit obstacle
creature (trigger type 21). Play already had this behaviour: the obstacle's box test, the ball's step back off it, and
the computer players' strike handling. The gap was in the test harness. The bot-match harness in
`crates/hst-play/src/play/fifty_hz.rs` (`played_ticks_rng`) never made or stepped the court figures, so there was no
fence there for the ball to hit.

With the figures made and stepped, play's ball matches the recording bit for bit from tick 465 to 571. In the recording
the ball's x stops dead at -4.364768 between ticks 470 and 471, and its z slows from about 0.05 to 0.004 a tick (the ball
is at about x -4.36, z 18.14, 0.6 high). Tick 572 is where the new point places the ball.

The 6 AI draws come from the strike itself. The obstacle tells the match the ball struck it, and each computer player that
is in its receive or rally state goes back to the start of that state. That is the doubles AI's strike handling, and it
runs synchronously inside the message. The decompile's trigger notes say ball-hit obstacles broadcast the strike message,
and the doubles AI's message handler re-enters its state on it. Nothing new had to be ported for this, so there is no
spec phase.

The timing took the most work. With the figures in, play drew the 6 two ticks early in the harness's generator mapping:
play's tick 470 against the game's 471. Calibrating against the serve contact at tick 141 settles it. There the game's AI
draws come one recorded interval before the ball's first move at the new velocity, and play makes the ball's contact a
tick before its AI's draws. At the fence the game's draws fall in the same recorded interval as the ball's frozen move,
so the game's obstacles act before that tick's ball move. Play's computer players run a tick behind its ball against the
recording (the harness's usual one-tick offset). So the AI has to hear the strike two ticks after play's figures see it,
at the end of a tick. With that, the 6 draws land on tick 471, as in the game.

In numbers: the EU AI generator now matches the recording on every tick from 1 to 1180, except the three serve contacts
(140, 742 and 1181), where the draws are split a tick apart as known before. Point 2's set-up runs its 164 ticks (it was
154). The match's phases match the recording through point 3's set-up (135, 303, 134, 164, 84, 134, 221). Point 3's
rally is the first divergence: it runs 684 ticks in the recording and 771 in play.

## Changes
- `crates/hst-play/src/play/npcs.rs`:
  - `sim_plugin` (test-only) makes the court figures at setup and steps them each tick with nothing drawn, for the
    harness.
  - A new `hear_strike` system runs before `control`, in both the app plugin and `sim_plugin`. A ball strike on a creature
    now sets a countdown, `strike_heard = 2`, in `Npcs` (a serde default, so snapshots keep it), instead of calling
    `doubles_ai::struck` at once. The countdown runs at the start of the tick after next. The ball's step back off the
    creature and the shared path and record clearing still happen at the strike itself.
- `crates/hst-play/src/play/fifty_hz.rs`:
  - The bot-match harness adds `npcs::sim_plugin`.
  - `the_european_generators_play_as_recorded`: the 6-draw allowance is dropped. The test asserts 6 draws on tick 471,
    the AI generator as recorded through 1180 (except 140 and 742), and the first 7 phases as recorded. The default length
    is now 1190 ticks.
  - `the_european_first_point_ends_as_recorded`: the ball is checked as recorded through tick 571 (it was 470).
- `tools/check.sh -p hst-play` passes (287 tests).

## Not verified / not 1:1
- The two-tick delay in the AI's strike handling is set from this one fence strike and from the serve contact's timing in
  the recordings. No recording has a strike while a computer player is in its receive or rally alongside other AI draws
  in the same ticks, so its order against those draws is untested. Here everyone is in the point-over, so the order
  within the tick isn't tested either. Court 7's struck props (types 27 to 29) take the same path, and no AI generator
  recording checks them.
- The players' footing still parts at tick 465, the reaction after the scoreboard pause (B726). Unchanged.
- The serve contacts' one-tick split (140, 742 and 1181) is unchanged. B771 covers the missing contact draw.
- Point 3's rally parts from the recording (684 against 771 ticks). Not looked at here.

## Next
Point 3's rally length (684 recorded against 771 in play) is the next divergence in the EU match.
