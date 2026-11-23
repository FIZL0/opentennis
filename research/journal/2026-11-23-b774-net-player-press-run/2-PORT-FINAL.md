# B774 port: the doubles computer player's press runs its approach on the press tick

Port of [SPEC.md](SPEC.md). Summary of the session before it is in [README-PART.md](README-PART.md).

## What changed

Code: `crates/hst-play/src/play/doubles_ai.rs`, the doubles computer routine's step.

1. The routine's preamble runs a pending approach step only when the tick is not the press's search frame. It uses the
   same `searching` check the human and fallback bot paths already use, so the search frame no longer moves the player.
2. When the routine presses, the player has no approach yet, and an approach with frames > 0 is found from its spot
   before any step, the press runs first. If it planned an approach, the player takes that approach's first ordinary run
   step (`locomote` along the approach direction) in place of the stick move. This also holds for a press the AI judged
   as a dive. Any other press keeps the old order: stick move, then press.

Release note: one plain-English line in `RELEASE_NOTES.md` about the computer net player moving on its press tick.

## Results

- EU bot match probe (`HST_M183A1_FROM=1302 HST_M183A1_PLAYER=2,1580,1612` on `the_european_bot_match_plays_as_recorded`):
  player 2 matches the recording bit for bit from 1580 to 1611. That covers the press tick 1588, the 19-frame run to
  1606, standing still on 1607, and the 0.068 steps on 1608 and 1609. The players part at 1886 (was 1588), the ball at
  1885, and the phases at 1858.
- All 8 `fifty_hz` tests pass, including the ignored US and EU bot matches. The US match's assertions are unchanged.
- B132's `approach_runs_its_frames_then_locks` and `bots_dive` pass.
- The whole `hst-play` suite passes (287 tests).

## Not verified / not 1:1

- A non-dive computer press with an approach planned is not seen in any recording. It follows from the same press
  handling, as the spec says.
- The check that an approach exists before the press is a guard, so a press that plans none keeps the old order. This is
  a structural choice and is not seen in the game.
- The US match's full output was not diffed against main. Only its assertions were checked.
- No PCSX2 capture was made. The recording already holds the positions.
- The later EU part at 1885/1886 is not looked at.
