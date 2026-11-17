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

## Port (second session, decompile locked)
- Play now has `recorded_path`: the window of the path record from this frame's entry, at most N entries, with heights
  negated back down. It uses the record's own contact counts.
- The press search and the dive now read the press's window: at least 16 entries, the search's horizon otherwise
  (`whiff_horizon`). `swing::search` cuts it to the horizon and `swing::dive` to entries 5–16. The auto-approach
  reads horizon + 20 and returns no approach when the window is empty. The whiff search reads `whiff_horizon`.
- The serve search and the partner-ball test helper stay on `predicted_path`, as the spec says.
- `fifty_hz` `the_us_first_serve_set_up_plays_as_recorded`: the returner's slide 291–298 is now checked bit for bit
  on x. It passes, and it fails with the old live prediction (tick 291: −3.108075 vs −3.1080744). All 263 hst-play
  tests pass, including the EU `serve_to_return` and the bot replays, with no changes.

## Not verified / not 1:1
- **The auto-approach with an empty window** reads stale memory in the game. The port treats it as no approach, as
  the spec says. I couldn't reproduce the case, and a press on a live ball shouldn't meet it.
- **The serve AI's toss search** stays on the live prediction. I didn't check which source the game reads. Before
  any court contact the two are identical in both recordings, so the values match whichever it is.
- **Searches late in a flight** (after the record's second predicted contact, when the window can be shorter than
  the horizon) aren't covered by a recording that tells them apart. The tests that pass don't exercise a short
  window that changes a result.
