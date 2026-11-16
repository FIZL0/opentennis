# M184: a computer server's toss to contact, spec phase

The spec is in `SPEC.md`. Spec-only session: no Rust changed.

## Found
- **Release rule, both builds.** The release fires when posed frame + motion speed ≥ 45 (US) or 37.5 (EU), with
  the posed frame lagging the press by one tick. The European release lands one tick after play's.
- **The toss is a hit launch.** It replaces the predicted path and sends the match's hit message to every AI. So
  every computer player redraws its timing errors and picks on the release tick. That accounts for the 60
  AI-generator draws there in `rng_s05.bin` (draw counts per tick from the recording: +1 at the toss press, +60 at
  the release, +1 at the swing press, +68 at the contact). It also changes the server's quick-serve pick, serve
  error and serve level, which is the likely cause of gaps 3 and 4.
- **Last hitter is −1** at slot 5's first toss, read live over PINE (it is set at the first contact). Point set-up
  doesn't clear it.
- **The ball holds at the hand point.** The recorded ball sits at the release point for 2 samples, then follows
  play's curve 2 aligned ticks late, all the way to the contact.

## Not verified / not 1:1
- The per-AI split of the 60 draws (15 each) is inferred from the total and the usual redraw. It isn't counted per
  player.
- Whether "nobody" (−1) counts as team 1 by parity in the redraw's own-team/opponent branch.
- Whether the second hand-point sample is the game or a repeated recorder sample (the later lag says the game).
- The European chain (release +1, hold, redraw → contact at tick 143) is reasoned, not run. There is no European
  generator recording.
- Gaps 3 and 4 are explained through the toss-time redraw, not shown. The port phase proves them against both
  first-serve tests.
