# M188: the contact search's predicted path (spec phase)

Spec in `SPEC.md`. Next session: the port phase (decompile locked).

## What I found
- The game's contact searches read the AI's path record, the prediction made at the launch. Play re-predicts from
  the live ball on every search.
- The record's predictor bounces off the flat court plane. The live ball bounces off the court mesh, so the two
  differ by a few ulps from the first court contact on. That is the 6e-6 between the game's predicted contact and the
  flown ball.
- The searches that read the record: the press search, the dive, the auto-approach and the whiff search. Each one
  reads a window of the record from `start`, cut short where the record ends.
- `scripts/record_vs_live.py` compares the record with the live ball over the p11n1 recordings.
  - doubles: equal on 752 pre-contact frames, different on all 282 after.
  - singles: equal on 634, different on 166.

## Not verified / not 1:1
- **No port yet** (spec phase): the US slide is still 4e-6 off until the port lands.
- **The auto-approach with an empty window** reads stale memory in the game. The spec says to treat it as no
  approach. I couldn't reproduce it, and a live press shouldn't meet it.
- **The serve AI's toss search** is left on the live prediction. I didn't check which source the game reads. Before
  any court contact the two are identical in both recordings, so the values match whichever it is.
