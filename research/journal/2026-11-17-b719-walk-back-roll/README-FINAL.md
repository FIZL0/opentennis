# B719: the US bot match's stand/walk split at 331

Done from recordings and fixtures only. I did not read the decompile.

## What differed

1. **Walk-back roll column.** After its own side hits, a computer player waits, then rolls whether to walk back
   to its spot. Play's doubles CPUs used the row's *doubles* centre rate and radius (100 / 0.5), so player 1's
   roll always passed and it walked at 331. The game's AI blocks in `ai_net_s05.bin` hold the row's *singles*
   centre numbers (rows 84/85/86/89: 55/60/55/70 with radius 1.0) for all four computer players. The singles
   fixtures show the same. The formation code (`ai_wait`) already used this rule: doubles numbers only beside a
   human partner. Fixed in `doubles_ai::run`. Probe: `b719_walk_back_columns` (ai_rally.rs, ignored).
2. **Stroke flag one tick late.** The routines' stroke flag in the game clears exactly `recover` frames (30, or 15
   after a slice) after the swing countdown first reads done. That holds for every stroke in ai_net_s05 and
   ai_rally_s05_all. Probe: `b719_stroke_lengths`. Play reported the whole follow-through motion as the stroke,
   which ends one tick later, when a stick may break it off. As a result, SWING → WAIT and the receiver's entry to
   the rally came a tick late: the US AI-generator miss at 329, and player 0's part at 402 after fix 1. Body.stroke
   now clears at `after > recover` (`doubles_ai::recovered`). Locomotion is still gated on the full motion.

## Result

- US probe: players part at 417 (was 331), ball at 703 (was 443). The AI generator matches every tick through 386
  except the known 218, and stays on the recording's draws through 695 (was 426).
- EU probe: players part at 303 (was 256). The AI generator matches through 301 (was 260).
- Tests tightened: both first-serve set-ups check every player through the return's flight (US 370, EU 302).
  The US bot match checks the generator through 386 and 695. The EU generators test checks through 301.
  `serve_over::a_server_enters_its_rally_return_roll_first` now expects the singles column's rate.

## Not verified / not 1:1

- Player 2's dive (US, pressed at 319): the game keeps the stroke flag set 5 ticks longer than play (through 390,
  not 385). Its wait therefore starts early, so the generator misses at 387–391 and 417–421 and the players part
  at 417. This is follow-up B720. The game's body also reports `moving 2` during the dive where play gives 0.
  Not checked whether that matters.
- Player 0's doubles_ai frame label lags its partners' by one: it starts calling the routine later, after
  serving. Its stand decisions still match, so I left it unchanged.
- Players 2/3's shot records hold NONE (−2) in the game but CLEARED (−1) in play after a strike. Only checked
  that the rally routine compares against NONE only beside a human partner, so all-CPU matches can't see it.
- The game's swing countdown reads 3,2,1,−1, play's 2,1,0,−1 on the same ticks. No decision in these matches
  read the difference.
- Beside a human partner, the doubles column remains. The bot matches can't show it; it follows `ai_wait`'s rule.
- The stroke-flag change applies in singles too: `frame()` is shared. Its evidence is the doubles recordings
  only. The singles fixtures use another record format that `b719_stroke_lengths` can't read. The singles
  replay and oracle tests still pass. The singles bot match in `smash_marker::lob_places_its_own_marker` now
  reaches a volley lob, which the marker search marks like any other lob, so the test now counts volley lobs as lobs.
