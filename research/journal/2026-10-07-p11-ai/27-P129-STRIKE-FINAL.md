# P129: the doubles AI's strike (0x14) and new-path (0x16) messages, recorded

P116 left `Rally::heard_strike` and the message handler's new-path writes (hold flag on a serve's path, held cleared)
proven by P11k6's spec only: `ai_glue_s05.bin` (two points) had neither message.

## Recordings (slot 5, court 10, bot-only doubles; tag 6 = the AI message handler only)

- `context/fixtures/ai_netcord_s05.bin`: `tools/pcsx2.sh python3 tools/record_ai_rally.py 5 15000 … 1 6`, the
  match as it plays. Six net touches (24 handler calls): four on the serve (shot count 1), two in rallies (shot
  counts 3 and 4). The AIs heard them in the serve state, the receive and the rally (substates 0, 1 and 2, all sent
  back to the wait substate with the Return roll).
- `context/fixtures/ai_strike_s05.bin`: the same with `HST_STRIKE=43`. Nobody's ball hit a figure in 4 minutes of
  play, so the recorder now moves one of court 10's two ball-hit figures (type 43, found by vtable and type in the
  state's RAM) onto the ball once a point, the game paused, at the ball plus half its last move, like
  `record_npc.py`'s `HST_POKE` did for P14's `hit_c10`. The shot count it waits for (1–4) and how far past the net the
  ball is cycle point by point. 14 strikes (56 handler calls), and 8 net touches (32 calls), some of them the struck
  ball's. Covered: rallying AIs in substates 0/1/2 sent to wait, three AIs mid-swing kept their swing (no draw),
  receiving AIs back to the receive's wait (no draw), the serve state left alone.
- Only the figure's position is poked. The ball strikes it and the game's own code sends the message.

## Test

`crates/hst-sim/tests/ai_rally.rs` `strike_and_new_path_match_the_game` replays both files through
`glue_replay` (`glue_matches_the_game`'s loop moved into a helper, unchanged). Every message call is bit-exact against
the exit record: the AI's bytes, the shot records and the draw count, 1188 calls in the strike file. A check that
the test can fail: dropping `heard_strike`'s swing check makes 4 calls differ.

No code changes were needed. The sim's `heard_strike` and the app's `doubles_ai::heard`/`struck` wiring already
matched.

## Not verified / not 1:1

- The strikes are forced: the figure is moved onto the ball's line. Nothing in the AI's message handler reads the
  figure. The handler sees what a natural strike would give it, but the shots after a strike (the ball off a figure
  in mid-court) are not what a real match plays.
- The held flag: the replay clears it on a new path for an AI in its rally state. The app clears it when the match
  phase is Rally. Clearing it in every state also passes, because no recorded new path found held set. The two
  rules agree in every recorded case, but they could differ for a receiving AI with held set.
- A receiving AI mid-swing at the message (it should keep its swing) is not in either recording. The rally's
  mid-swing case is.
- The app's `doubles_ai::step` wiring as a whole is still checked by its playing tests only (gap P128). These
  recordings drive the sim's glue, as `glue_matches_the_game` does.

## Aside

`done.sh`'s clippy stopped on main's `swing.rs` `smash_mode` (P119), which negated an f32 in a
`deny(float_arithmetic)` module. It now negates the integer after the cast. The value is 0.5–10.5 before
truncation, so the result is the same.
