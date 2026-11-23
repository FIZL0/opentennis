# B784 spec: the effects seed is redrawn at every change of ends

## What draws it
The match's effects (the rain grid, the wind's leaves and ripples) have their own small generator: the effects LCG
(×214013 + 2531011). It keeps a seed, the **effects seed**. Every serve message (a new point's set-up) restarts the
effects LCG from that seed and clears the effects' per-point state. The seed itself is a `rand()` output.

The effects listen for two messages that redraw the seed, one `rand()` call each:
- the **match start** message, during the match setup: this is the setup's effects seed that play already draws
  (`Rngs::setup_rand`, step 5 of P3f's setup sequence);
- the **change of ends** message: play does not draw this one. It is the missing draw.

Nothing else is drawn for it, and the drawn value is only stored. It has no effect on the tick it is drawn.

## When and in what order
- The change of ends message goes out on the tick the match leaves the point-over for the change of ends. That is
  the same moment play takes `Next::ChangeEnds` and calls the sound manager's reseed.
- The effects hear the message before the sound manager. So on that tick `rand()` gives, in this order:
  1. the new effects seed;
  2. the sound manager's new seed (`reseed_sound`, as now).
- The match start message has the same order: the setup's effects seed comes just before the sound manager's
  second reseed. Play already has that order.
- The tick's lens flare draws (24 while the weather is below 2) come after both, as at every other tick.
- The change of ends' end (the serve message into the next point) draws as it does now: shared and court, and no
  sound reseed. The serve message restarts the effects LCG, now from the seed drawn at the change of ends.
- This holds for every change of ends message, whatever caused it (odd games, a set's end, a tiebreak's
  six-point changes): the effects react to the message and do not check the score.

## Both builds
The same in the US and European programs. The task's line says European only, but the US draws the same:
- European disc (`eu-bot-start.p2s`, slot 8): the change of ends' start frame (vsync 43492) draws 2 besides the
  flare's 24. They come from the effects' handler and then the sound manager. The setup frame (37518) has the same
  pair after its 30 cloud draws.
- US disc: `fixtures/b767/us-rng-game2.bin` (slot 5's game 2) has 26 draws on the change of ends' start frame
  (14154), and 26 on its end frame (14235): 24 flare + shared + court. The US caller log from slot 5
  (`context/b784/run4/us.bin`) has the start frame's two draws from the same callers in the same order: the effects'
  handler, then the sound manager.
- The US probe doesn't catch it yet: past the change of ends it checks phases, positions and the AI generator, not
  `rand()` or the shared generator.

## What the port changes
- At the change of ends' start, before `reseed_sound`, draw one `rand()`. The stand-in draws after the reseed, so the
  draw moves; it doesn't just lose its test gate. The AI's change-of-ends message (`doubles_ai::heard`) draws no
  `rand()` (the start frame has only the two calls), so the draw can go either side of it. That value becomes the match's effects seed
  from then on (`play::set_effects_seed`), so the next serve's effects LCG restart uses it. The same applies after
  every change of ends.
- Remove `fifty_hz::CHANGE_ENDS_DRAWS` and its loop. Remove `HST_M183A1_CHANGE_ENDS` from the EU probe. The draw is
  not test-only, and it is not a 50 Hz rule.
- Rollback/netplay: the effects seed is now match state that changes mid-match. Today it is a process-wide value set
  once at setup (`play::set_effects_seed`), not part of the match. A snapshot must carry it, or a
  rollback over a change of ends restarts the effects LCG from the wrong seed. That is presentation only: the
  `rand()` stream itself is already in the snapshot.

## Checks for the port
- EU probe (`the_european_bot_match_plays_as_recorded`) with no env: 24 phases match (what `HST_M183A1_CHANGE_ENDS=1`
  gave). `eu_point_reseeds`: the game-2 first point seeds at the same offset as every other point.
- US (`the_us_bot_match_plays_as_recorded` with `us-rng-game2.bin`): through the change of ends, play's `rand()`
  state equals the recording's at the same ticks. Before the port it is one draw behind from the change of ends'
  start frame on.

## Open questions
none
