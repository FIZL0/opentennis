# M184: the spec's open questions, answered

Spec-only session (no Rust). `SPEC.md` is updated and its *Open questions* section is now empty.

## Answers
- **Second hand-point sample: a recording artefact.** The toss launch places the ball and marks it as flying. The
  ball update runs before the players and moves only a flying ball. So the game holds the ball at the hand for
  exactly one tick, the release tick. The launch resets the flight counters, and the first flight step is a normal
  one. Each flight step saves the ball's position before moving it, and M183a1's tick list reads that saved copy.
  The copy lags the live position by one tick through the whole flight. Checked in both recordings: the live
  position is the hand point on the release tick and has moved the tick after (US 169, EU 114). Spec section 1 is
  rewritten to match.
- **The 60 draws: 15 per computer player; "nobody" is no team.** Both team tests need a hitter ≥ 0, so −1 fails
  both. No shot is recorded and no reaction/pace/guess draws run. The dive chance isn't drawn either (it is only
  drawn at the point's start or on the own team's hit). The singles and doubles redraws each make 15 draws on that
  path: 8 for the timing errors, then quick serve, two shot-choice chances and four levels. The generator recording
  agrees: AI +60 and shared +2 on tick 169, nothing else on that tick.
- **European press tick: 76, release 114 (38 ticks).** Found in the EU match recording from the server's serve
  sub-state and toss kind, which change on the press tick, and from its "ball up" flag. The same fields in the US
  recording give press 124 and release 169 (45 ticks), which agrees with the generator draws there. The spec now
  has a table of recorded ticks for both builds (toss press, release, swing press, contact) as the port's targets.
  European contact: 141 on the live position (M183a1's 143 came from the saved copy and that test's alignment).

Scratch scripts for this (recording field scans, generator draw counts) stayed in the session scratchpad. They
read fixtures only.

## Not verified / not 1:1
- The European release threshold (37.5) comes from M181's build switch. This session's decompile is the US build's
  (threshold 45). The European recording's 38-tick press-to-release gap fits it.
- The order of the 60 draws among the four players was read from the redraw, not checked one value at a time
  against the recording. Only the total and the per-tick counts were checked.
- M183a1 measured the toss as 1 tick late at 60 Hz and 3 at 50 Hz. The release (+1 European) and the saved-copy lag
  explain most of the difference between the builds, but this wasn't re-run in play. The port phase proves it
  against the tick table.
