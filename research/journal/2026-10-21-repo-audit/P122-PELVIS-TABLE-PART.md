# P122 — pelvis turn table bit-exact

The turn's per-motion pelvis rows used to come from `pose::first_frame`: the first keys, unscaled, multiplied
down the hierarchy in host f32. That came within 1e-4 of the game's table but matched only 28 of 1056 entries
bit for bit.

Trying the game's own sampler, which was already ported (`Clip::locals`, PS2 arithmetic) at frame 0, with the
pelvis's local matrix taken up through its parents by VU0 products (`node_world` with an identity player
matrix), matches **all 1056** entries bit for bit. Those are the 14 characters × the motions each one has, from slot-5 RAM,
`1p3goodcpus_ee.bin` and the 14 `p7b_cNN_pelvis.bin` dumps. The decompile wasn't needed; this reuses ported code
and the dumps prove it.

- `pose::pelvis_row(sk, clip, node)` is new. `first_frame` and its host `rotation` helper are gone (it had no
  other users).
- `hst-base` `disc_motions_counted` builds each row from the motion's `Clip`.
- `tests/player.rs` `pelvis_table_from_disc` now asserts bit equality (≥560 rows, 0 off), and `lefty_s04` builds
  Will's rows the same way. All 13 player tests pass.

## Not verified / not 1:1
none. Every dumped table entry is bit-exact. Motions missing from a character's archive keep the `[0, 1]`
default as before; the dumps hold no such entry to compare against.
