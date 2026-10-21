# B628 — spec phase

Wrote `SPEC.md`. Shots 8 and 9 (the alternate camera) share a per-shot hook that the cut-away runner never ran. It
does three things:

- **Pull-back:** after the orbit is built, the eye backs away horizontally along the view by min(ball depth − human
  depth, a limit). The ball depth is pushed back and floored 6 m behind the baseline, or plain ball + 0.3 before the
  first shot / in the rally's first 30 frames. The limit is 0 on the baseline and −3·(1 − depth/11.885) inside it,
  which is the 1.69 m net gap.
- **Head-room:** the ball is kept within 0.9 of the half-height above centre. This is the small tilt.
- **Smoothing:** from the shot's third frame on, the whole view (slerped rotation, eye, field of view) moves 5 % a
  frame toward the new one. This is what B621 fitted as an eased ball, so `BALL_EASE` goes.

The followed ball is the live ball, frozen once the point is over.

Black-box check of the pull-back rule against `context/b621/alt_follow_s04.bin` (Python, not kept): over 1861 frames
the eye's horizontal gap from the human comes out with mean error −0.007 m and standard deviation 0.088 m. That is
with a crude constant base gap.

The recording repeats 39 samples (same game frame read twice), which the port's test must skip.

Next: the port session (decompile locked).

## Not verified / not 1:1

- Nothing ported yet (spec phase). The rule is checked black-box only (above), not bit for bit.
- The quiet case's "no shot struck yet" can't be read from the recording (it has no shot count). The spec gives the
  inference.
- Singles (shot 8): same hook and constants by reading the game. Not compared with the original (no singles save
  state).
- The smoothing applying after the Select glide (so a switch keeps settling past 15 ticks) comes from reading the
  game. It is not checked against a recording of a switch.
