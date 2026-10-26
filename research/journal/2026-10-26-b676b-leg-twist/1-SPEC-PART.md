# B676b — thigh "bend" over the clip: spec phase

## What I found
B676's equal thigh and calf "bends" are one change made to the thigh's drawn matrix after the pose is propagated. The
thigh keeps its animated bone axis, but its roll about that axis is blended with the model's rest roll, carried onto
the animated axis. The blend factor is per character and costume group: 0.5 for chars 0, 2, 4, 10, 12 and for char 6
in costumes 5–9, and 1.0 (skipped) for the rest. The calf and below aren't moved, which is why the probe saw the calf
"turn" by the same angle. It's skin only, and nothing in gameplay reads it. The full rule is in `SPEC.md`.

## How it was checked
`scripts/b676b_thigh.py <slot> <frames>` (run under `tools/pcsx2.sh`) reads, in one settled frame per player and
thigh: the factor, the animated local, the model rest local, the parent's world and the drawn thigh world. It
rebuilds the drawn world from the rule (float64). Slots 3, 4 and 5 (chars 6/4/3/11 and 0/2/1/5): error ≤ 1e-6 on
every thigh with factor 0.5. Players with factor 1.0 are skipped by the game, and there the plain local × parent
gives the drawn thigh (up to 4e-4 on the moving player 3, a torn read during a crossfade, not this rule).

## Next (Port phase)
Implement from SPEC.md in `crates/hst-base/src/character.rs` (animate). A test can pin it with the B676 fixtures
(`fixtures/b676/s4.bin` holds chars 6 and 4 with factor 0.5): the drawn thigh world from the port against the dump's
thigh world. Carol there is costume 9. The dump doesn't store the costume, so the test has to state it.

## Not verified / not 1:1
- Characters 7–13 weren't on court in any save state. Their factor is read from the game's table, not seen live.
- Carol in costumes 0–4 (factor 1.0) wasn't seen live. Same reason: it's straight from the table.
- The degenerate case (animated axis exactly along the rest axis) was never seen. Its behaviour comes from how the
  vector unit handles a zero length.
- Mod characters and front-end previews: the original has no equivalent, so it's our choice (see SPEC.md *Port notes*).
