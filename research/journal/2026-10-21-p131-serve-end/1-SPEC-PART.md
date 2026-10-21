# P131 — spec phase

Wrote `SPEC.md`: the serve state ends when the server's stroke flag clears (frames since contact reach the serve's
recovery, regardless of stick or motion end). The AI sees it one frame later (its update runs before the player's
step), so the hand-over is at contact + recovery + 1, the 31 frames recorded with a 30-frame recovery. The serve
routine only runs in the serve and rally phases, so a point over before then gives the server no rally entry. Both
modes' serve routines end the same way. Next: the port session (decompile locked).

## Not verified / not 1:1

- Nothing ported yet (spec phase).
- Both recorded hand-overs are full-strength serves (recovery 30). The 15-frame case (slice/underhand/weak toss)
  comes from the rule, not from a recording.
- The no-hand-over-in-a-finished-point quirk comes from reading the game. `ai_glue_s05.bin` has no fault or ace to
  show it.
