# B675 spec phase: players' head look-at

I found how the original turns a player's head toward the ball and wrote it up in `SPEC.md`. The port phase follows.

- Only the head bone turns: a turn of ±50° and a nod of 0–30°. The windows shift with the clip's pose. It moves at most
  7° per tick, and its target is the ball seen from a point 0.15 m above the head.
- It tracks from 15 ticks after a contact (a shared hold in doubles) until the point ends. It does not run during the
  serve wait or between points. On a stroke start it eases back to the clip pose ("release").
- It feeds gameplay: body-hit uses the head's x-row, so the port must be bit-exact. Body-hit then sees the previous
  tick's turned head.

Recording: `scripts/b675_look_rec.py 5 3000 context/b675/run1.jsonl` (under `tools/pcsx2.sh`, HST_PCSX2 set). It logs
the per-player look state and the head local each frame.

## Not verified / not 1:1
- Lefty mirroring (the parent's x column negated): slot 5 has no left-handed player, so this comes from the
  decompile only.
- The enable flag of the shared hold was always on in the recording, so its off case is unknown.
- The slow-motion factor and the clip-end counter branch were never active in the recording.
- Nothing has been ported yet (spec phase), so there is no bit-exact proof yet. That falls to the port phase.
