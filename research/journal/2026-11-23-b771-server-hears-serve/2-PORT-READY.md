# B771 port phase

## What changed

The doubles AI now hears the hit message of its own serve. Before, the message only went to computer players in the
receive or rally state, so the serving player skipped it. Now the serve state gets it too, alongside receive and
rally.

The rally-only work inside the hit handler (going back to waiting, clearing what is held) still runs only in the
rally state, as before. The toss path, where nobody has hit yet, is unchanged. A hit by nobody does no team work, so
letting the serve state through there would change nothing.

## Files

- `crates/hst-play/src/play/doubles_ai.rs`: `heard_hit` passes the doubles hit message to a computer player in the
  serve, receive or rally state. Previously only receive and rally.
- `crates/hst-play/src/play/fifty_hz.rs`: `the_us_bot_match_plays_as_recorded` now asserts at least 29 phases (through
  game 2's second serve's set-up), and that every player matches on ticks 219 to 7387, except on each point-over's
  last tick (1084, 2354, 3190, 3795, 4391, 5011, 5689, 6543, and 6941 for the fault).

## Checks

Run with `HST_M183A1_OFF=1` and the B767 and M183a1 fixtures present:

`CARGO_INCREMENTAL=0 tools/check.sh -p hst-play --lib fifty_hz -- --include-ignored`

All 8 tests pass. Incremental builds of bevy_render and bevy_pbr in this worktree crashed rustc with a missing
`.pre-lto.bc` file. A non-incremental build worked. This is a tooling note only.

## Not verified / not 1:1

- No new PINE recording was taken this session. The proof is the existing slot 5 US recordings (player positions and
  generator samples) through tick 7386.
- Beyond tick 7387 the match still parts from the recording. The first miss is the players at game 2's second serve
  onward. That is a separate gap and not in this task.
- Singles is not touched.
- Why team 1's server's spot moves at that serve was not broken down. Same as in the spec phase.
