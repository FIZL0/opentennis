# B768: the EU match's ticks mapped to vsyncs; a computer server's serve pick moves to the end of its entry tick (FINAL)

## Result
Play's new-point reset now lands on the same ticks as the EU recording. The two loading stalls in point 1's point-over
phase were the cause of the "5 ticks late" that R14 reported, and they were never play's fault: the test compared play's
tick numbers with the recording's vsync numbers without mapping one to the other. Play's reset was not late. One real
difference was left: a computer server's serve pick was drawn a tick after its entries, where the game makes it in the
same tick. That is fixed. A separate gap remains on tick 471 (point 1's point-over tick 34), where the game's AI
generator draws 6 numbers that play does not, and it throws point 2's serve wait off by 6 places. That is B770.

## How it was measured
- New test helper `recorded_vsyncs` (`crates/hst-play/src/play/fifty_hz.rs`). For each game tick of the EU match
  recording (`fixtures/m169/eu-bot-match.bin`), counted the same way as `recorded_ticks`, it gives the vsync that tick's
  sample was taken at, or none for a tick that ran unsampled.
- `the_european_generators_play_as_recorded` now reads the recorded generators (`fixtures/m183a1/eu-rng-long.bin`,
  indexed by vsync) by game tick through that helper, not by vsync.
- Probe: `HST_B768_DRAWS=from,to` prints the AI draws for each game tick in that range, the recording's (where the tick
  was sampled) and play's, side by side.

## What it shows
- The ticks and vsyncs drift apart in this recording at two loading stalls that B723 found in point 1's point-over
  phase. Each is 3 vsyncs long, with one tick running unsampled inside it:
  - right after the scoreboard's pause ends (vsyncs 464 to 466): ticks run 2 behind from there, so tick 465 is vsync 467;
  - as the phase leaves for point 2's set-up (vsyncs 574 to 576): 4 behind, so tick 573 is vsync 577.
  Later the lag grows to 6 behind from tick 847 and to 8 from tick 955.
- R14 read the stalls differently: 2 vsyncs at tick 124 and 3 at tick 250 of the phase it called phase 0, 5 in total.
  R14 counted vsyncs from the match object's appearance and read a different counter. In the match recording itself the
  stalls are B723's two. R14's "5 ticks late" was play's tick number (572) set against the recorded vsync (577)
  without this mapping: 4 of the 5 come from the stalls, and 1 from the harness's usual one-tick offset between play's
  generators and the recording. Play's reset was not late.
- Point 2's new-point reset: the game draws 68 numbers on tick 572, the tick the set-up starts (17 for each of the 4 AI
  objects). Play used to spread the rest over two more ticks: 19 on tick 573 (the server's serve entry 17, and the two
  non-receivers' return roll, 1 each) and the server's serve spot and wait (2) on tick 574. The EU recording's single
  sample after the stall (vsync 577) covers ticks 572 and 573 and holds all 89, with nothing at 574. So the last 2 were
  a tick late.
- Slot 5's US recording has rand-mapped vsyncs and no stall there. It splits the reset the same way the fix does: 68 on
  one tick, then 21 (17 + 1 + 1 + 2) on the next. So the game does the reset in the phase-change tick, and the entries
  and the server's pick in the tick after. R14 said "all 89 in one tick"; its vsync must have covered two ticks.

## Changes
- `crates/hst-play/src/play.rs`: a computer server now picks its serve spot and wait at the end of its serve state's
  entry tick, after every player's turn. This is a new `ai_serve_entered` called from `control`. The pick itself is now
  `ai_serve_entry`, which `bot_serve` still calls. The order is entries first, then the pick. The US match already
  matched with this order, which is the evidence for it. The walk still starts the tick after, so nobody moves
  differently.
- `crates/hst-play/src/play/fifty_hz.rs`:
  - `recorded_vsyncs` added, and the EU generator test reads by game tick through it.
  - `first_point_index_matches_the_original`: third value 142 changed to 143. That value came from play's own model, not
    from a capture.
  - `every_point_enters_again`: 19 changed to 19 plus the pick's own draws, which is 1 or 2 depending on the serve level.
  - The US match's AI generator now also matches at the four set-ups' second ticks (1086, 2356, 3192, 3797), which were
    listed as off before. The remaining off ticks are the serves' contacts (218, 1269, 2578, 3484).
  - The EU generators now match the recording by game tick from tick 1 (the set-up's first tick, which was left out
    before) through 470, except the serve's contact at 140, as before.
  - The point 2 span check in the test asserts the set-up's span with the 6 draws of the gap below allowed for.
  - `HST_B768_DRAWS` probe added.

## Gap found (B770)
On tick 471 (point 1's point-over tick 34, vsync 473) the game's AI generator draws 6 times and play draws none. That is
the tick the game's ball meets court 6's stage. B723's journal says the ball meets the stage at 471 in the game but not
in play, and names a B725 for it, which was never filed. Play's AI generator is 6 draws short from there on. With those
6 added, play follows the recording through point 2's reset to tick 652. It then parts at tick 653, the server's toss,
because the serve wait is drawn 6 places off. That is why point 2's set-up runs 154 ticks in play against 164 in the
game. The test asserts this span with the 6 added. The follow-up is B770.

## Not verified / not 1:1
- Which game routine makes the 6 draws on the stage tick. That needs the decompile, so it is a spec task (B770).
- Whether the game splits the 68 and 21 draws across the two ticks in the EU match too. The EU sample covers both ticks,
  so only the US recording shows the split.
- At point 1 the game makes the entries (vsync 0) and the pick (vsync 1) on separate samples. Play still makes the
  resets a tick early and makes the entries and the pick together. The harness leaves tick 0 out.
- The stall positions come from one recording, not a repeat run.

## Next
B770: find what meets court 6's stage on tick 471 and what draws the 6 numbers (spec, then port), so play's AI generator
follows the recording past point 2's serve toss at 653. Then drop the 6-draw allowance in
`the_european_generators_play_as_recorded`.
