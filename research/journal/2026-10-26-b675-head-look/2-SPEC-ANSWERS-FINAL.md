# B675 spec round 2: open questions answered

The first round left five open questions in `SPEC.md`. This round answers them from the decompile, plus a short PINE
read of slot 5: the identity matrix that holds the release vector, the ball position's w (1.0), and the hold's
flag and counter. `SPEC.md`'s *Open questions* now says none. Its *Answers to the first round's questions* section
lists what changed.

While answering them I found and fixed these errors in the first round:
- Only the ball target is remapped. F, R and the final forward are world rows and are used as they are.
- The release vector (0, 0, 1) is used raw. It is the identity's z row, read live. Remapping it would have asked
  for a 180° turn.
- The tie rule of the wrapped difference was backwards.
- The last final angles come from world′, not from the stored local.
- In the latch-off gating, a dive or a whiff never calls, whatever the frames since contact.
- The hold is cleared at match start, at the change-of-ends set-up and at the serve set-up.
- The doubles release check's comparisons are strict, and z = 0 counts as positive.
- The lefty target goes through the true parent. Everything else uses the mirrored parent′.
- The slow-motion in-between frames run the look too.
- The FPU and VU0 operation order is now given for the eye shift, the windows, the rate step and the apply.

## Not verified / not 1:1
- Lefty mirroring is still from the decompile only, because no save state has a left-hander. The spec sends the
  proof to the port phase: the function oracle with the mirror flag set.
- The slow-motion in-between frames and the clip-end branch were never seen live. Both come from the decompile.
- None of the arithmetic order is proven yet. That needs the port's bit-exact test against `context/b675/run1.jsonl`.
