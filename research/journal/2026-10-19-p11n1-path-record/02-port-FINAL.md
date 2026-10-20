# P11n1 port (2026-10-20)

Ported from SPEC.md and new recordings, decompile locked.

## What landed

- `hst-sim/src/path.rs`: the path record (entries, start, active, mode, gap, look countdown, predictor) with the
  per-frame update, the launch (`hit`), the net re-seed (`net`) and the stops.
- `play/doubles_ai.rs`: the AI's `World.path` is the record's window, `path_mode`/`path_gap` its mode and gap. The
  180-frame flight stepped each tick (`path`, `Steps`, `predict`, the cache and its test) is gone. Messages: NEW_PATH
  starts a path, HIT re-seeds and builds (from the serve on), STRIKE/REACTION stop it, SERVE/CHANGE_ENDS stop it and
  clear the look countdown; the claim is kept on REACTION and REACTION + 1; a HIT between ends leaves the shot records.
- `play.rs`: the update runs first in `control` (the game: ball step, then the path, then the players; the app's
  tick is players then ball, so the previous tick's ball step is already done). Every live-ball step that adds a
  special contact re-seeds the predictor and sends NEW_PATH (it was the first touch only); `path_base` stays for the
  human contact search.
- `npcs.rs`: a creature strike stops the record.
- Also fixed: the skipped smash-offset cache printed in the state text, so `doubles_at_100ms_stays_in_sync…` failed
  after a resync (B574); it is now left out of the Debug text.

## Recordings (context/live/p11n1, `run.sh a|b|c|d|e` under `tools/pcsx2.sh`)

`tools/record_live.py` gained `--path` (record header 0x130 bytes: buffer, end, start, active, mode, gap, the four
shot records, countdown, claim, AI copy stamp/count, phase block, side, shots, players; then the tip entries),
`--reseed` (with `--vel`: the predictor is rewritten with the poked ball and the path restarted, saved to `.seed`),
`--setpath V,END,START` (end/start poked after vsync V).

- doubles: slot 5, 1500 frames. singles: slot 8 = `bots_singles`, 1500 frames (one frame missed).
- cord: slot 5 ball poked into the cord at 7864 (net touch: same-frame re-seed, end 15 start 0, counters kept, the
  special count too, which corrects SPEC.md's "two counters").
- out_a/out_b: long balls (gaps 0.2233, 0.1907); cap/cap2/cap4: balls sent straight up (drag keeps them under 600
  entries; cap4 also lands out); cap_e/cap_f: end/start poked to 585/575 and 590/580: the batch is never cut (590 →
  605) and the record goes inactive after it.
- Serve paths (phase 2) keep mode 0; the rally phase calls.

`hst-sim/tests/path_record.rs` steps every frame from the previous frame's record and predictor (with the launch or
net touch where the game had one) and checks end/start/active/mode/gap, the new entries and the predictor bit for
bit: 4293 frames, 8500+ entries, 61 hits, 1 net touch, 3 outs, 105 capped frames, all exact. `bots_singles` plays on
a court with the surface of courts 1/2/3/9.

## Not verified / not 1:1

- The look countdown: kept and cleared per the spec, but no look-at-the-ball pose reads it; the recording only shows
  it follows 15 / count-down / 0, not which player states count as swinging (the app uses `swing`).
- Slow motion skipping the net re-seed: the app's match has no slow-motion steps of the live ball.
- REACTION + 1 and message 6 are not sent by the app (the claim rule for REACTION + 1 is in, unused).
- The gap's starting value: the game's is whatever is in RAM (a denormal in the recordings) until the first out;
  the app starts at 0.0. Only matters for a hold check before any out prediction, which compares mode first.
- The human contact search (`predicted_path`) still uses `path_base`, the first-net-touch approximation.
- Not run against a live match end to end in the app (the record feeds the AI; the per-frame test is against the game).
