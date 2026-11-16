# M184 — the port phase's three open questions answered (spec)

Answered from the decompile; SPEC.md sections 4 (launch inputs), 5 (later toss) and 6 (phase at the press).
Open questions: none. Next: port phase.

- **Q1, a later point's toss.** The hit message carries no data. Every AI reads one match-wide "last stroke"
  record, written only by real strokes, cleared once at match build and never at point set-up. A later toss
  re-announces the previous real stroke unchanged (hitter, kind, velocity, aim, the hitter's position at that
  stroke). The doubles repick uses that stored position. After a fault, the second toss re-notes the faulted
  serve: the receivers' serve history holds it twice.
- **Q2, phase at the press.** Starting the serve swing requests rally; the match applies it at the start of the
  next frame, before the players (press + 1). Whiffs too. From press + 1, bounces make sounds and first-bounce
  events, the match's hit handling counts hits, singles helpers stop treating it as serve, and doubles stamina
  ticks. The AIs only tell point over apart from serve or rally.
- **Q3, launch 1e-4.** The serve launch starts from the ball's live position, always at full power, toward an aim
  point fixed on the swing-press tick from the server's position on that tick. The server's height never enters.
  Play aims at the countdown's second-last frame instead: the likely source.

## Not verified / not 1:1

- Q3 is a reading of the decompile, not a measurement: no recording of the aim point at the press was compared with
  play's. If aiming on the press tick doesn't close the gap, the ball's contact-tick position is next (see spec).
- That the aim is set exactly on the press tick rests on the aim being computed once when the swing begins (it
  isn't recomputed) and on the AI generator staying still from the press to the contact. The server's position
  sample that feeds it wasn't traced to the frame.
- Q1: where the record's velocity comes from (before or after the stroke's own launch adjustments) wasn't traced;
  it was written just before the launch, from the stroke's computed values.
- Q1's second-serve double entry and the full/partial reset of the serve history at set-up weren't checked against
  a recording.
- Q2: the doubles stamina constants and the run-frame clamps are named, not specified (no change between press and
  contact in four to six ticks unless a 60-tick boundary falls there).
- Unchanged from 3-PORT-PART: per-player order of the 60 draws, no EU generator recording.
