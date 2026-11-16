# M186: the returner's slide into the return

## What it was
- The slide before a stroke's contact is the contact IK's body step. At the press, the stroke's arm is solved
  against the contact point. Then the body moves toward the solve position plus step × weight, over the IK's
  ramp (8 ticks for the US return, 2 for the EU one).
- slot 5's own recording of that solve (`fixtures/anim_s05.bin`, the solve at vsync 7854, player 1) has
  - contact (−4.24201775, −0.78939843, 11.3753042)
  - position (−3.0927196, 0, 11.776895)
  - step −0.12284347, so 0.0153554 a tick over 8 ticks, the slide the match recording shows.
- Play passed the contact point with its height negated. The predicted path is already Y down, as the solve
  wants (the recorded contact's y is negative), so play solved for a ball 0.79 below the ground. That gave a step
  of −0.36682 (US) and −0.34440 (EU) instead of −0.12284 and −0.15473.
- No decompile was read: the recordings showed the cause, so this is a one-sign fix and not a clean-room port.

## After the fix
| | US (slot 5, 60 Hz) | EU (M177, 50 Hz) |
|---|---|---|
| step | −0.12283945 (game −0.12284347) | −0.15473157 |
| returner's slide | within 4e-6 of the recording, 291–298 | bit for bit, 229–230 |
| players | as above | all bit for bit through 248 (was 228) |
| the return launch | still ~3.6e-4 off at 299 | still ~8e-4 off at 231 |

- So the slide was **not** what threw the return launch off: the EU returner stands bit for bit at the contact
  and the launch is still off. That is new task M187 (spec first).
- The US leftover 4e-6 comes from the contact point: the game's predicted contact is ~6e-6 off the ball it then
  flies, while play's prediction is exactly the flown ball. So the game predicts the path in a slightly different
  way. That is new task M188 (spec first).

## Proof (`play::fifty_hz`)
- `the_european_first_serve_set_up_plays_as_recorded`: `serve_to_return` checks every player bit for bit through
  248, slide included. With the old sign it fails at 229.
- `the_us_first_serve_set_up_plays_as_recorded`: the returner's slide 291–298 within 1e-5 on x, z exact. With the
  old sign it was 0.03–0.24 off.
- `play::snapshot::a_bot_taking_over_plays_on` broke. The bot match's rallies changed, and the "rally" case's
  first qualifying point now ends with 1P's own side winning, so idle 1P then held up its own serve until the
  3-minute cap. Changed the test only: when the point ends before the hand-over, the seat goes back to the bot and
  the test waits for another rally.

## Not verified / not 1:1
- **US slide 4e-6 off** (within 1e-5): the game solves for its own predicted contact point, which play doesn't
  reproduce yet (M188).
- **The return launch** is still off in both matches (M187), so neither whole-match probe gets further than the
  return. The US one also still has M185 at 318.
- Other strokes' slides now use the right contact height. They weren't compared one by one against
  `anim_s05.bin`'s ~40 solves in play: `contact_solve_anim` already proves the solve itself on those recorded
  inputs, and this fix only changes what play feeds it.
