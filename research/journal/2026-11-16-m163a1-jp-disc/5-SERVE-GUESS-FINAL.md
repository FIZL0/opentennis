# M163a1a1a: the 7-frame serve-return wait is a guess (ヤマ張り)

In `fixtures/m163a1a1/jp-bot-ai.bin`, at vsync 44645, player 1 waits 7 frames after the serve. No reaction draw
gives that. It is the serve guess:
- Player 1's AIParam row has guess = [15 % chance, 8 move frames, 32 stuck frames]. 7 is 8 less the frame player 1
  has already counted down by the end of the frame.
- The guess byte in the Japanese AI object (US offset −0x10, like the wait) is 2 (the far side) on that frame and
  0 on every other serve return. The spot the AI stood on when it drew follows the byte: (3.0, −12.25).
- The port already does this (`ai_draw_guess` in hst-play sets the hold to the move frames on a guess). The US
  does the same, and `guesses_match_the_game` (hst-sim `tests/ai.rs`, `ai_s05.bin`, 3 guesses) already checks it,
  so I didn't need a new US capture.

## The test (`japanese_cpu_reactions`)
- The known-case list is gone.
- Each serve return is now replayed whole from the receiver's AI record:
  1. The test finds the timing draw whose errors are the recorded ones, anywhere in the frame's draws. The inputs
     are the seen shots, the earlier shot and the earlier serve.
  2. The picks, the reaction and the guess follow it.
- The replay must use the frame's draws exactly: up to the end of the frame for player 1, and 15 short of it for
  player 0, since player 1's draws follow.
- It must give the recorded wait (the guess's move frames when it guesses) and the recorded guess byte (1 near
  side, 2 far side).
- All 29 serve returns pass, 1 of them a guess. The other 222 reactions keep the old check, so all 251 are
  explained.
- The old "returns-count draw that may follow on a serve" was really the guess's chance draw.
- What else the replay showed: two serve returns were caught off guard by a change of pace (pace −12, −1). The
  pace and fast-ball rules run the same as in the US port.

## Not verified / not 1:1
- **The verdict side of the guess in the Japanese build** (a right guess runs harder, a wrong one stands for the
  stuck frames) isn't checked on this recording. The one guess in run3 isn't followed by a recorded verdict check.
  The US verdict is checked in `guess_verdicts_match_the_game`.
- **The Japanese AI object layout** (the US one 0x10 lower from the seen shots on) is inferred from the fields
  that line up: kind, wait, errors, guess byte and spot. It was not mapped field by field.
