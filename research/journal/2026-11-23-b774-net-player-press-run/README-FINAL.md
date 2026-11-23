# B774: the EU net player's press at point 3's tick 1588 (spec ready, port not done)

Behaviour is in [SPEC.md](SPEC.md). This is the session summary.

## Result

The game's computer net player moves on its press tick. When the press plans an approach run, the run's first step
happens on the press tick along the approach direction (0.9974, 0.0714), 19 frames counting the press tick, and the
search tick after it does not move. Play's doubles computer routine presses after its own run step for the tick, and
on a press the AI judged as a dive it skips that step. So the approach's first step waits a tick, and play stands still
on 1588, then matches the game's positions one tick late, catching up at 1608.

Trial in play (env-gated, reverted): players match through 1885 and part at 1886 (was 1588). The ball still parts at
1885, and the EU phase count stays at 16 of 144. The later part is a separate issue.

## How it was found

1. The probe `HST_M183A1_FROM=1302 HST_M183A1_PLAYER=2,1580,1612` on `the_european_bot_match_plays_as_recorded` shows
   player 2's positions from 1580 to 1612 against the recording.
2. A trace in play (temporary, reverted) showed the press is not a dive start. The press finds the ball on the player's
   side and plans the approach run: direction (0.9974, 0.0714), 19 frames counting the press tick. Play's doubles
   routine runs its press after its run step for the tick. On the search tick the doubles routine's preamble still runs
   the approach a step, so the run count is right (19) but one tick late, and play moves on the search tick.
3. B132 (`research/journal/2026-10-12-b132-lock-whiff/1-APPROACH-FINAL.md`) proved on the live game that an approach
   planned N frames runs exactly N frames, the press frame included, and the next frame does not move. That was for
   human presses. The game's computer player goes through the same press handling, so the rule carries over.
4. Trial in play (env-gated, reverted): on a fresh press with an approach run, the approach step replaces the stick move
   on the press tick, and the doubles preamble takes no approach step on the search frame. Results are as in Result. All
   8 `fifty_hz` tests, including the ignored one, pass; the US bot match asserts to 7387 with its known exceptions. The
   whole `hst-play` suite passes.

## Where the port goes

- `crates/hst-play/src/play/doubles_ai.rs`, the doubles computer routine's step: its press handling. On a fresh press
  with an approach run planned, the run's first step goes on the press tick in place of the stick move.
- The same file, the routine's preamble: its approach step must not run on the search frame.

No PCSX2 capture is needed for the port: the recording already shows the game's positions.

## Not verified / not 1:1

- The game's decompile was not traced for the order of the computer player's press and its run step. The rule rests on
  B132's live capture (human presses) and on this recording's bit-exact positions.
- The approach step in place of the stick on a non-dive press with an approach planned is not seen in any recording.
  No such press has parted the US or EU matches so far. It follows from the same press handling.
- No PCSX2 capture this session. Not needed for the positions above.
- The later EU part at 1885/1886 is not looked at.

## Closing

The spec is written and the port is scoped; the port phase implements it from SPEC.md with the decompile locked.
