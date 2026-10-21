# P11l6 — singles AI wiring gaps: spec phase

This session read the decompile and wrote `SPEC.md`. No Rust was written. The port phase implements it with the
decompile locked.

## Findings (details in SPEC.md)

- **Shot target:** the player's own aimed point of its latest shot. Rally shots: `shot::aim`'s target. Serves:
  `serve::target`'s aim. In both it's taken before the margins, the scatter and the launch, and it is kept per player
  until the next aim. The app passes the current flight's landing instead.
- **Shot memory spot:** the hitter's position at the strike (w = 1, the jump's y). In the recordings the x and z
  equal what the app passes (the opponent's position at the AI's next step). Only y and w differ.
- **Message order:** a player's shot sends only the hit message. Strike means the ball hit a bystander, and new path
  means it touched the net. The singles hit only does three things: the shot memory, the dive dash and the timing
  draw. It does no re-entry, no return-to-centre roll, no net repick and no `held` clear. The app re-enters and
  draws on every new shot in singles.
  - The fixtures confirm it: across about 150 opponent hits, the wait substate's markers were never reset.
- **under_min:** the character's TParam value (0.40 and 0.44 m in the recordings). The app passes 0.

## Not verified / not 1:1

- **The aim's exact frame inside the stroke** wasn't pinned down. The routines read the target only once the
  stroke is over, so it shouldn't matter. The port checks the kept target against the recorded one call by call.
- **The toss's write to the target** (during the AI's own serve) is described as unobservable by the routines. That
  follows from the dispatcher's state, not from a recording.
- **The frame-exact `step` replay (the done check)** depends on P11k6's shared gaps too (contact depth, smash
  offset, path mode/gap, hold flag, stick quad). It can't pass on this task's changes alone.
- **The spot's y/w** come from the fixtures and the decompile. The app's ported routines use only x and z, so they
  affect only byte-level AI-object comparisons.
