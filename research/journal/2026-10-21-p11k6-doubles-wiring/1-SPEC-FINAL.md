# P11k6: doubles AI glue — spec phase

Spec session (clean room): read the game's doubles AI update and message handler, checked the app's
`play/doubles_ai.rs` glue item by item against the task line, recorded the two on the running game, wrote
`SPEC.md`. No Rust this session.

## What was done

- `tools/record_ai_rally.py`: tags 5 (the AI update, with the stick quad / button word it leaves) and 6 (the message
  handler, message id as its second argument) for doubles; their scratch areas sit between tags 0–3's; not combined
  with `HST_PROBE`.
- Recorded `HST_PCSX2=1 tools/pcsx2.sh python3 tools/record_ai_rally.py 5 1500 context/fixtures/ai_glue_s05.bin 1 5,6`:
  1549 frames, two points, 5448 updates and 112 messages, 61 MB, no unhooked calls.
- Scratchpad checks on it: per-message AI byte changes and draw counts, state transitions with their draws, the
  globals at message time (that's where "shot count and last hitter are still the previous shot's" comes from).

## Findings

Of the task line's items, most are already right (SPEC *Already right*): contact depth, smash offset, path
mode/gap and window, human records, hold flag, hit counters, timing draw shot memory, the caller's stick quad (the
caller clears it every frame, as `Out::default()`). Open for the port:

1. Own-team hit → formation repick (missing).
2. Strike / new path / hit re-entries at the message, not at the next update (draw order, put-none record order).
3. Receive → rally only on the receive routine's own return, in the same frame (drop `ai_update`'s rule).
4. The acceptance replay test over `ai_glue_s05.bin` (SPEC *Acceptance test*).

## Not verified / not 1:1

- No strike (0x14) or new-path (0x16) message reached the AIs in the recorded points (CPU-only doubles): their
  handler rules are from reading only. A recording with a let/net-cord or a dive return would cover them.
- The reaction message drew for only one of four AIs in the recording; the rule for which one is from reading
  (`Mind::point_result` already exists and is assumed right).
- The per-hit draw split (Return roll / repick / timing draw) is only checked as a total (15–21 per AI per hit).
