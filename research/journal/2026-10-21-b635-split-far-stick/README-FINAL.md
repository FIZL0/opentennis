# B635 — split view: the far team's stick (local 2P-vs)

B633's split picture shows each team from behind its own end. Offline with humans on both teams there is no own
camera (`own_camera::local` returns none), so `own_camera::flipped` turns every stick by the shared camera's end:
right for the top half's team, reversed for a human on the other team in the bottom half.

## Change
- `split_view::turned(g, i)`: while the picture is split, player `i`'s own end's camera turns the stick differently
  from what `flipped` uses (`(end < 0) != flipped`). Only the far team of an offline both-teams game can hit that:
  online and one-team games have `cam_of` set (flipped already follows the player's end), a lone human's shared
  camera follows them, and the top team is at the shared camera's end.
- `tick_input::local` turns those seats' pads before `TickPads::push`: stick axes `255 − b` (exactly the negation on
  the game's `pad::axis` scale; the centre stays 0x80), d-pad up↔down, left↔right; buttons and the right stick's
  emote flick unchanged. So the pads recorded, replayed and hashed are what the sim played; no sim state touched.
- The turn follows the split being on (last frame's `split_view::on`): between points, cut-aways, replays and the
  change of ends show the one shared picture, and the stick steers by it again, as before.

## Tests
- `tick_input::tests::a_turned_pad_runs_the_other_way`: `pad_dir` of a turned pad is the negated run direction for
  every stick byte (exact off centre) and d-pad combo; buttons and emote kept.

## Not verified / not 1:1
- Not played live in a tall window with two local pads (unattended run, no second pad / user on hand); the rule is
  checked by reading `flipped`/`frame` and the byte turn by the test.
- A centred axis keeps its hair (+1 of 127 on the game's scale, ~0.45°) on the same side when turned: its exact
  opposite (byte 0x7e) would read as a stick off centre to the idle checks. 0x7f (only reachable without the
  driver's deadzone) turns into the centre.
- The split's on/off is the previous frame's: the frame the split starts or ends, the far human's stick can still
  steer by the old picture for one tick. Presentation of the input only; the recording is consistent either way.
