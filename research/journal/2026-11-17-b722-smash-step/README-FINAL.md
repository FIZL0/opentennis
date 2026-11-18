# B722: a smash's contact IK (slot 5's US bot match, player 1 at 855)

Done from recordings and earlier journals; the decompile was not read.

## What moved player 1

`HST_M183A1_PLAYER=1,780,870` (the probe now prints y as well): player 1 stands still from 853. In the game it then
steps +0.0046983 m along x on each of 855 and 856 and stops, and its root dips (y −0.141 at 856, back to 0 by 867).
The ball's direction changes at 856→857 at a height of 2.19 m, so player 1 is smashing. Play kept player 1 still.

Two equal steps over two frames is the contact IK's ramp (weight n/frames, step·weight) with two frames to contact.
The y dip is the overhead solve's jump on the root path, which play doesn't model (as for the serve). B29a's journal
already recorded the rule: the original runs the same overhead solve as the serve for smashes, using the smash
motion's own arm table. The solve and table were ported and oracle-checked, but the stroke start skipped IK for
smash motions.

## Change

- New `play/smash_ik.rs`: at a smash's start, the overhead solve from the smash motion's arm (`serve_arm` on the
  smash clip, frame 8). It is ticked as a stroke's contact IK (`ArmIk::new` with the frames to contact, through the
  mover). The jump scale is computed but not applied.
- `play.rs`: one `else if` in the stroke start that calls it for the smash motion, plus the `mod` line.

## Result

- US probe: every player matches bit for bit on the ground through the whole first point (was 855), the ball too.
  The AI generator matches every tick through 933, except the known 218 (was 869). 934–935 are the rally's last two
  ticks, which play ends two ticks early (B723).
- `the_us_bot_match_plays_as_recorded` now asserts that no player parts in point 1, and the generator through
  `end − 2`. It keeps 960 ticks of generators (was 900).
- EU probe unchanged: no player and no ball part in point 1.

## Not verified / not 1:1

- **The smash's jump (root y):** the solve computes it but play keeps players on the ground, as with the serve. It is
  presentation only, and the tests compare positions on the ground.
- **Only one smash checked against a recording**, a right-hander's with two frames to contact and a pure sideways
  step. Not checked: a smash whose solve steps forward or back (z), a left-hander's, or one with more than 8 frames
  to contact (the IK frame ramp is assumed to be the strokes' `ik_frames`).
- **The motion speed going back to 1 the frame before contact** (the IK's `unit` flag) now applies to smashes too.
  The match stays exact through the point, but I didn't check the smash's motion frames against a capture.
