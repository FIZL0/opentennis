# B700 spec phase (PART: the port session continues from SPEC.md)

Spec: `SPEC.md` (this dir). A computer server only takes its contact pick once it is 8 or fewer frames away
(always 8 in practice), then presses when countdown + err ≤ 8, so it presses at `min(8, 8 − err)` and never early.
The swing's own search then lands on the same ball frame, so the swing offset is `−max(err, 0)`.

## Done
- Read the singles and doubles AI serve states and the contact search they use (both run the same rule).
- Live check on PCSX2 copy 1 (save slot 8 copied from the will-curve session's copy 7: Ashley human vs CPU Will),
  `research/b700_serve_press.py 8 900`: 19 Will serves. The countdown always read 8 when the pick was taken, and the
  swing's real timing offset was exactly press countdown − 8 (0 at 8, −1 at 7, −4 at 4). The AI pick's look-ahead
  read live: from 2 frames ahead, 180 long.

## Correction to the will-curve notes
The "offset" those notes recorded is the serve's **depth error** (tenths of a metre), not the swing's timing
offset. The original only puts a serve's timing bias into the depth error when the stick points mostly forward or
back, and into the side error otherwise. So that column read 0 for clean serves and for mistimed serves aimed
sideways. This explains their "err 1, 2 → 0" rows (err 1 is really offset −1) and their unbent "offset 0" topspin.
The press frames they measured are right, and they match the decompile.

## Flat share (left to B700a)
The AI's kind roll (at the press) and its aim grid (the frame before contact) draw nothing that depends on the
press frame. Yet both late presses here (offset −4) came out flat, and the will-curve run saw 10 of 15. The link
must be downstream: possibly which stick the contact reads, or the AI handing over to its rally routine after the
aim frame. That needs its own live probe, so it went to B700a. The press rule doesn't depend on it.

## Not verified / not 1:1
- Doubles CPU serves: same rule in the decompile, not recorded live for this task.
- The flat share against press timing: open, B700a.
- err = 0, 2, 3 presses weren't seen in this 19-serve run (the will-curve countdown logs cover 2 and 3).
