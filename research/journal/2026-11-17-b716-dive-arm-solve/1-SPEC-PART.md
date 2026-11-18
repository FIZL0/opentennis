# B716 — dive arm solve: spec phase (PART: port phase next)

Spec in `SPEC.md`. Where it came from (decompile, read this session):

- The dive search (`0x34d8a0`), contact branch only, calls the solve `0x3573a0` last, after the arm slide
  `0x3509e0` (whose outputs, tip and turned upper-arm matrix, are the solve's inputs) and with the path frame's
  ball position and velocity (y sign flipped back, as the slide does).
- The load-time snapshot `0x357150` (receive motion at time 5) keeps the Racket, RHand, RForearm, RUpperArm locals,
  the upper arm's parent chain, and the racket/upper-arm model matrices (the last two = `Reach::dive_arm`). Node
  slots: racket 0, right hand 9, right forearm 11, right upper arm 12, left upper arm 13 of the 17 named nodes.
- Per frame: the arm-solve block of the per-frame play routine `0x34ec70` (after the dive's own per-frame branch).
  The dive ramp is a separate branch keyed on a flag the dive solve alone sets; its weight branches were checked in
  the disassembly (`research/dis.sh 34fc00 34fd40`) because the decompile's "0 between n = 6 and K, 0 at n = K" looked
  wrong: it is right. K is read live as (dive end frame − 50) = the contact frame; nothing else writes either field.
- The racket turn rebuilds the Racket local from the load-time copy times a y rotation (axis-angle builder
  `0x1b3400`, the arm table's); the arm turns go through the same slerp-and-multiply as the contact IK.
- The rotation-from-cosine builder (`0x1b3560`) is the one `volley_solve` uses (`axis_cos`); the transposes are full
  3×3 transposes with a zero translation (checked the stack arguments in the disassembly at the call).
- Clears: the stroke/stand/run mode setter `0x34afc0` (at its start, before the search, so a fresh dive's solve
  survives), `0x34af60`, the placement `0x3449f0`, the contact/volley solves.

## Not verified / not 1:1

- Nothing here is checked against the running game: N6's recordings hold no contact dives (all 8 are miss dives),
  so there is no capture of the solve's quaternion, racket angle or the dived racket matrix yet. The port phase
  should get one: a contact dive on a save state (a CPU dive at a reachable ball, or a poked path), recording the
  player's racket node local and world row 0 for the dive's ~K+5 frames, and the next player's stand-test scratch.
- Degenerate NaN cases (arm already pointing at the ball, ball along the racket's y) described, not reproduced.
- In-between (high frame rate) frames left as presentation: the game's sub-frame weight has its own quirks at
  n = 5 and n = K that the spec doesn't carry over.
