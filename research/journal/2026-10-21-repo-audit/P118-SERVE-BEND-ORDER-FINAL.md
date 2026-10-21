# P118 — serve bend vs stick order (FINAL)

Question (T8 ledger): `serve_bent` (the box-margin "bent" test before a serve's aim is pulled inside the box) used
the serve kind *before* the stick turned a topspin into a flat. Which kind does the original test?

## Finding
Checked by reading the game's frame order for a human serve (no code copied):
- Each frame the server's aim step runs first. Its very first act is the stick rule: a topspin code with the stick
  toward the net (facing · stick / |stick| ≥ 0.5) is rewritten to the flat code, in the player's shot code itself.
- Only after that does the contact step read the serve kind from the shot code, and that (already flat) kind is the
  one handed to the launch, where the "does this character bend this kind and is it special" test runs, and where
  the box-margin pull uses it.
- So the original tests the bend on the **stick-turned** kind. A flat serve never bends (only topspin and slice
  serves have a bend entry), so a strong-tossed, sweet-spot topspin that the stick turned flat gets the scaled
  (normal) margins, not the bent serve's unscaled ones.

Ours differed: fixed in `play.rs` (serve contact) by applying `shot::stick_kind` before `serve::inside` and passing
the turned kind to `serve_bent`; the launch (`strike`, shot effect, recovery motion) already used the turned kind,
unchanged. A CPU server goes through the same path with its planned aim in place of the stick, as before.

## Not verified / not 1:1
- No recording of the exact case (human server, character with a serve bend, strong toss hit within a frame of the
  sweet spot, stick held toward the net): it needs frame-precise vpad timing on a bender, and the oracle can't run
  the contact in isolation (it is in the player's per-frame tick with the whole match state). The order was
  established from the original's control flow, not from a capture; the fix is a reorder of two existing calls.
